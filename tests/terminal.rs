//! Терминальный режим по HTTP: фрагмент для htmx, полная страница без JS,
//! очистка вывода и подсказки-ссылки.

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

struct Reply {
    status: StatusCode,
    body: String,
}

async fn send(request: Request<Body>) -> Reply {
    let response = app().oneshot(request).await.expect("ответ получен");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 4 << 20)
        .await
        .expect("тело ответа");
    Reply {
        status,
        body: String::from_utf8(bytes.to_vec()).expect("тело в utf-8"),
    }
}

async fn get(uri: &str) -> Reply {
    send(
        Request::builder()
            .uri(uri)
            .body(Body::empty())
            .expect("запрос"),
    )
    .await
}

/// Команда так, как её отправит htmx.
async fn exec(command: &str) -> Reply {
    send(
        Request::builder()
            .method(Method::POST)
            .uri("/terminal/exec")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header("HX-Request", "true")
            .body(Body::from(format!("command={command}")))
            .expect("запрос"),
    )
    .await
}

/// Команда без htmx: обычная отправка формы.
async fn exec_without_htmx(command: &str) -> Reply {
    send(
        Request::builder()
            .method(Method::POST)
            .uri("/terminal/exec")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(format!("command={command}")))
            .expect("запрос"),
    )
    .await
}

#[tokio::test]
async fn terminal_page_prompts_for_a_command() {
    let reply = get("/terminal").await;
    assert_eq!(reply.status, StatusCode::OK);
    for expected in [
        "visitor@fennr",
        "Доступные команды:",
        "hx-post=\"/terminal/exec\"",
        "/static/htmx.min.js",
        "/static/terminal.css",
        "обычная версия",
    ] {
        assert!(
            reply.body.contains(expected),
            "на странице терминала нет {expected:?}"
        );
    }
}

#[tokio::test]
async fn terminal_page_is_canonical_to_itself() {
    let reply = get("/terminal").await;
    assert!(
        reply
            .body
            .contains("<link rel=\"canonical\" href=\"https://fennr.ru/terminal\">"),
        "канонический адрес терминала"
    );
}

#[tokio::test]
async fn htmx_gets_a_fragment_with_the_answer() {
    let reply = exec("about").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(
        !reply.body.contains("<!DOCTYPE"),
        "htmx получает фрагмент, а не страницу"
    );
    assert!(reply.body.contains("class=\"line echo\""), "эхо команды");
    assert!(reply.body.contains("Восемь лет в бэкенде"), "ответ целиком");
    assert!(reply.body.contains("class=\"hints\""), "подсказки дальше");
}

#[tokio::test]
async fn a_form_submit_without_javascript_returns_the_whole_page() {
    let reply = exec_without_htmx("stack").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.body.contains("<!DOCTYPE html>"), "полная страница");
    assert!(reply.body.contains("PostgreSQL"), "ответ в выводе");
}

#[tokio::test]
async fn hints_are_links_that_ask_for_fragments() {
    let reply = get("/terminal/exec?cmd=projects").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.body.contains("periodparser"), "ответ на команду");
    assert!(
        reply.body.contains("href=\"/terminal/exec?cmd=contact\""),
        "подсказка работает и без JS"
    );
    assert!(
        reply.body.contains("hx-get=\"/terminal/exec?cmd=contact\""),
        "и через htmx"
    );
}

#[tokio::test]
async fn clear_wipes_the_output_with_an_out_of_band_swap() {
    let reply = exec("clear").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(
        reply.body.contains("hx-swap-oob=\"innerHTML\""),
        "очистка вместо дописывания"
    );
    assert!(
        !reply.body.contains("class=\"line"),
        "у clear нет строк вывода: {}",
        reply.body
    );
}

#[tokio::test]
async fn unknown_command_prints_an_error_line() {
    let reply = exec("rm%20-rf").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.body.contains("class=\"line error\""));
    assert!(reply.body.contains("команда не найдена"));
    assert!(reply.body.contains("help"), "подсказка про справку");
}

#[tokio::test]
async fn empty_command_appends_nothing() {
    let reply = exec("").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(
        !reply.body.contains("class=\"line"),
        "пустой ввод не печатает строк: {}",
        reply.body
    );
}

#[tokio::test]
async fn homepage_points_to_the_terminal_version() {
    let reply = get("/").await;
    assert!(reply.body.contains("href=\"/terminal\""));
    assert!(reply.body.contains("терминальная версия"));
}
