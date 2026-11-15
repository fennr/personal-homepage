//! HTTP-слой: страница отдаётся с данными, статика — с ETag, чужие пути — 404.
//!
//! Приложение собирается с фиксированной датой, поэтому «8 лет 2 месяца» и
//! полоса карьеры не зависят от дня запуска тестов.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Method, Request, StatusCode, header};
use tower::ServiceExt;

use personal_homepage::content::{Site, YearMonth};
use personal_homepage::web;

fn app() -> Router {
    web::app(
        Site::embedded().expect("контент валиден"),
        YearMonth::new(2026, 11).expect("месяц"),
    )
}

struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

/// Только код ответа: тело может быть двоичным (например, PNG).
async fn status(uri: &str) -> StatusCode {
    app()
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("запрос"),
        )
        .await
        .expect("ответ получен")
        .status()
}

async fn get(uri: &str) -> Reply {
    request(Method::GET, uri, &[]).await
}

async fn request(method: Method, uri: &str, headers: &[(&str, &str)]) -> Reply {
    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let response = app()
        .oneshot(builder.body(Body::empty()).expect("запрос собирается"))
        .await
        .expect("ответ получен");
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), 4 << 20)
        .await
        .expect("тело ответа");
    Reply {
        status,
        headers,
        body: String::from_utf8(bytes.to_vec()).expect("тело в utf-8"),
    }
}

fn header_of(reply: &Reply, name: header::HeaderName) -> String {
    reply
        .headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned()
}

#[tokio::test]
async fn homepage_shows_the_profile_and_every_section() {
    let reply = get("/").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(header_of(&reply, header::CONTENT_TYPE).starts_with("text/html"));

    for expected in [
        "Виктор Меркурьев",
        "Бэкенд-разработчик и архитектор",
        "О себе",
        "Стек",
        "Опыт",
        "Проекты",
        "Контакты",
        "Стек по группам",
        "8\u{a0}лет 3\u{a0}месяца",
        "Уралхим",
        "май\u{a0}2024 —\u{a0}сейчас",
        "periodparser",
        "mailto:fenrir1121@gmail.com",
    ] {
        assert!(
            reply.body.contains(expected),
            "на странице нет {expected:?}"
        );
    }
}

#[tokio::test]
async fn telegram_is_the_primary_contact_in_header_and_footer() {
    let reply = get("/").await;
    assert!(
        reply
            .body
            .contains("<a class=\"btn\" href=\"https://t.me/fenn_r\">Telegram</a>"),
        "основной контакт печатается кнопкой в шапке"
    );

    let footer = reply
        .body
        .split("class=\"contacts\"")
        .nth(1)
        .expect("подвал с контактами");
    assert!(
        footer.contains("<a href=\"https://t.me/fenn_r\">Telegram</a>"),
        "Telegram есть и в подвале"
    );
    assert!(
        footer.find("https://t.me/fenn_r") < footer.find("mailto:fenrir1121@gmail.com"),
        "основной контакт идёт первым"
    );
}

#[tokio::test]
async fn career_bar_uses_latin_brand_names() {
    let reply = get("/").await;
    for brand in ["BC Soft", "Saprun", "Promobot", "Ujin", "Uralchem"] {
        assert!(
            reply.body.contains(&format!(">{brand}</div>")),
            "на полосе нет отрезка {brand:?}"
        );
    }
}

#[tokio::test]
async fn homepage_lists_every_role_and_project() {
    let reply = get("/").await;
    assert_eq!(
        reply.body.matches("class=\"job\"").count(),
        5,
        "мест работы"
    );
    assert_eq!(
        reply.body.matches("class=\"project\"").count(),
        6,
        "проектов"
    );
    assert_eq!(
        reply.body.matches("class=\"seg now\"").count(),
        1,
        "текущее место работы помечено один раз"
    );
}

#[tokio::test]
async fn homepage_has_a_skip_link_main_and_semantic_table() {
    let reply = get("/").await;
    assert!(
        reply
            .body
            .contains("<a class=\"skip-link\" href=\"#main\">"),
        "skip-link"
    );
    assert!(
        reply.body.contains("<main id=\"main\" tabindex=\"-1\">"),
        "цель skip-ссылки принимает фокус, иначе следующий Tab идёт с начала"
    );

    // Шапка профиля — вводный блок: она должна остаться вне main, иначе ссылка
    // «К содержимому» ведёт на начало страницы и ничего не пропускает.
    let header = reply.body.find("</header>").expect("шапка закрыта");
    let main = reply.body.find("<main id=\"main\"").expect("main есть");
    assert!(
        header < main,
        "main начинается после шапки и полосы карьеры"
    );
    assert!(
        reply.body.contains("<caption class=\"visually-hidden\">"),
        "подпись таблицы для скринридера"
    );
    assert!(
        reply.body.contains("role=\"img\" aria-label=\"Карьера"),
        "у полосы карьеры есть текстовая альтернатива"
    );
}

#[tokio::test]
async fn resume_button_stands_next_to_telegram() {
    let reply = get("/").await;
    let telegram = reply
        .body
        .find("<a class=\"btn\" href=\"https://t.me/fenn_r\">Telegram</a>")
        .expect("кнопка Telegram");
    let resume = reply
        .body
        .find("<a class=\"btn btn-quiet\" href=\"/static/resume.pdf\">Резюме, PDF</a>")
        .expect("кнопка резюме");
    assert!(
        telegram < resume,
        "резюме идёт сразу после Telegram, а не отдельным блоком"
    );
    assert_eq!(
        status("/static/resume.pdf").await,
        StatusCode::OK,
        "кнопка не должна вести на 404"
    );
}

#[tokio::test]
async fn open_graph_points_at_a_published_image() {
    let reply = get("/").await;
    assert!(
        reply
            .body
            .contains("property=\"og:image\" content=\"https://fennr.ru/static/og.png\"")
    );
    assert!(
        reply
            .body
            .contains("property=\"og:image:width\" content=\"1200\"")
    );
    assert!(
        reply
            .body
            .contains("name=\"twitter:card\" content=\"summary_large_image\"")
    );
    assert!(
        reply
            .body
            .contains("name=\"theme-color\" content=\"#F1F2EE\"")
    );
    assert!(
        reply
            .body
            .contains("name=\"color-scheme\" content=\"light\"")
    );
    assert_eq!(status("/static/og.png").await, StatusCode::OK);
}

#[tokio::test]
async fn terminal_metadata_is_its_own() {
    let reply = get("/terminal").await;
    assert!(
        reply
            .body
            .contains("property=\"og:url\" content=\"https://fennr.ru/terminal\"")
    );
    assert!(
        reply
            .body
            .contains("name=\"theme-color\" content=\"#14181B\"")
    );
    assert!(
        reply
            .body
            .contains("name=\"color-scheme\" content=\"dark\"")
    );
}

#[tokio::test]
async fn robots_and_sitemap_are_served() {
    let robots = get("/robots.txt").await;
    assert_eq!(robots.status, StatusCode::OK);
    assert!(header_of(&robots, header::CONTENT_TYPE).starts_with("text/plain"));
    assert!(
        robots
            .body
            .contains("Sitemap: https://fennr.ru/sitemap.xml")
    );

    let sitemap = get("/sitemap.xml").await;
    assert_eq!(sitemap.status, StatusCode::OK);
    assert!(header_of(&sitemap, header::CONTENT_TYPE).starts_with("application/xml"));
    assert!(sitemap.body.contains("<loc>https://fennr.ru/</loc>"));
    assert!(
        sitemap
            .body
            .contains("<loc>https://fennr.ru/terminal</loc>")
    );
}

#[tokio::test]
async fn responses_carry_baseline_security_headers() {
    let reply = get("/").await;
    assert_eq!(header_of(&reply, header::X_CONTENT_TYPE_OPTIONS), "nosniff");
    assert_eq!(
        header_of(&reply, header::REFERRER_POLICY),
        "strict-origin-when-cross-origin"
    );
}

#[tokio::test]
async fn homepage_is_not_cached_by_the_browser() {
    let reply = get("/").await;
    assert_eq!(header_of(&reply, header::CACHE_CONTROL), "no-cache");
}

#[tokio::test]
async fn head_request_returns_headers_without_body() {
    let reply = request(Method::HEAD, "/", &[]).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.body.is_empty(), "у HEAD нет тела");
}

#[tokio::test]
async fn health_reports_ok() {
    let reply = get("/healthz").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body, "ok");
}

#[tokio::test]
async fn unknown_path_is_not_found() {
    let reply = get("/nope").await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn stylesheet_is_served_with_cache_headers() {
    let reply = get("/static/app.css").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        header_of(&reply, header::CONTENT_TYPE),
        "text/css; charset=utf-8"
    );
    assert!(
        header_of(&reply, header::CACHE_CONTROL).contains("must-revalidate"),
        "стили перепроверяются: имён с хешем у нас нет"
    );
    assert!(header_of(&reply, header::ETAG).starts_with('"'));
    assert!(reply.body.contains("--accent"), "отдан именно файл стилей");
}

#[tokio::test]
async fn fonts_are_served_from_the_image() {
    // Тело — двоичный woff2, поэтому проверяется только ответ: тип и кэш
    // покрыты unit-тестами статики.
    for path in ["/static/fonts/plex.woff2", "/static/fonts/mono.woff2"] {
        assert_eq!(status(path).await, StatusCode::OK, "{path}");
    }
}

#[tokio::test]
async fn static_replies_not_modified_for_the_same_etag() {
    let first = get("/static/app.css").await;
    let etag = header_of(&first, header::ETAG);
    assert!(!etag.is_empty(), "ETag есть у первого ответа");

    let second = request(
        Method::GET,
        "/static/app.css",
        &[("If-None-Match", etag.as_str())],
    )
    .await;
    assert_eq!(second.status, StatusCode::NOT_MODIFIED);
    assert!(second.body.is_empty(), "у 304 нет тела");
}

#[tokio::test]
async fn missing_static_file_is_not_found() {
    let reply = get("/static/нет-такого.css").await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}
