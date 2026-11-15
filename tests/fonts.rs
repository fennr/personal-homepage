//! Нарезанные шрифты обязаны покрывать всё, что печатают страницы.
//!
//! Набор глифов в `static/fonts` фиксирован по алфавитам, поэтому обычная
//! правка текста шрифт не ломает. А символ вне набора (стрелка, иероглиф,
//! математический знак) молча уедет в системный шрифт — этот тест его ловит.
//!
//! `static/fonts/coverage.txt` пишет `just fonts` тем же набором, из которого
//! нарезан шрифт: разбирать woff2 в тесте незачем.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use tower::ServiceExt;

use personal_homepage::content::{Site, YearMonth};
use personal_homepage::web;

fn app() -> Router {
    web::app(
        Site::embedded().expect("контент валиден"),
        YearMonth::new(2026, 11).expect("месяц"),
    )
}

async fn body_of(method: Method, uri: &str, form: Option<&str>) -> String {
    let response = app()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(form.unwrap_or_default().to_owned()))
                .expect("запрос собирается"),
        )
        .await
        .expect("ответ получен");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1 << 20)
        .await
        .expect("тело ответа");
    String::from_utf8(bytes.to_vec()).expect("ответ в UTF-8")
}

/// Текст, который браузер показывает: без тегов, комментариев и сущностей.
fn visible_text(html: &str) -> String {
    let body = match html.find("<body") {
        Some(index) => &html[index..],
        None => html,
    };
    let mut text = String::with_capacity(body.len());
    let mut in_tag = false;
    let mut in_entity = false;
    for ch in body.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            '&' if !in_tag => in_entity = true,
            ';' if in_entity => in_entity = false,
            _ if in_tag || in_entity => {}
            _ if ch.is_control() => {}
            _ => text.push(ch),
        }
    }
    text
}

/// Символы текста, которых нет в наборе покрытия шрифта.
fn missing(text: &str) -> Vec<char> {
    let coverage = include_str!("../static/fonts/coverage.txt");
    let mut missing: Vec<char> = text.chars().filter(|ch| !coverage.contains(*ch)).collect();
    missing.sort_unstable();
    missing.dedup();
    missing
}

fn assert_covered(page: &str, text: &str) {
    let missing = missing(text);
    assert!(
        missing.is_empty(),
        "{page}: этих символов нет в наборе шрифта: {missing:?}; \
         дополните диапазоны в scripts/fonts и запустите `just fonts`"
    );
}

#[test]
fn main_page_is_covered_by_the_page_font() {
    let runtime = tokio::runtime::Runtime::new().expect("рантайм");
    let page = runtime.block_on(body_of(Method::GET, "/", None));
    assert_covered("/", &visible_text(&page));
}

#[test]
fn terminal_is_covered_by_the_mono_font() {
    let runtime = tokio::runtime::Runtime::new().expect("рантайм");
    let page = runtime.block_on(body_of(Method::GET, "/terminal", None));
    let help = runtime.block_on(body_of(
        Method::POST,
        "/terminal/exec",
        Some("command=help"),
    ));
    assert_covered("/terminal", &(visible_text(&page) + &visible_text(&help)));
}

#[test]
fn styles_point_to_the_fonts_that_exist() {
    for (css, font) in [
        (include_str!("../static/app.css"), "plex.woff2"),
        (include_str!("../static/terminal.css"), "mono.woff2"),
    ] {
        let path = format!("/static/fonts/{font}");
        assert!(
            css.contains(&path),
            "в стилях нет ссылки на {path}: шрифт не загрузится"
        );
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("static/fonts")
            .join(font);
        assert!(file.is_file(), "нет файла {}", file.display());
    }
}
