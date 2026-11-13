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
        "8 лет 3 месяца",
        "Уралхим",
        "май 2024 — сейчас",
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
    assert!(header_of(&reply, header::CACHE_CONTROL).contains("max-age"));
    assert!(header_of(&reply, header::ETAG).starts_with('"'));
    assert!(reply.body.contains("--accent"), "отдан именно файл стилей");
}

#[tokio::test]
async fn fonts_are_served_from_the_image() {
    let reply = get("/static/fonts.css").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(
        reply.body.contains("/static/fonts/"),
        "пути к файлам шрифтов"
    );
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
