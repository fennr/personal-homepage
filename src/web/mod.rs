//! HTTP-слой: маршруты, шаблоны и состояние приложения.

mod static_files;
pub mod view;

use askama::Template;
use axum::Router;
use axum::body::Body;
use axum::extract::{Form, Query, State};
use axum::http::{HeaderMap, HeaderName, Response, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::get;
use serde::Deserialize;
use tower_http::compression::CompressionLayer;

use crate::content::{Site, YearMonth};
use crate::terminal;
use view::Page;

/// Адрес сайта: один домен, поэтому он задан здесь, а не в контенте.
const SITE_URL: &str = "https://fennr.ru";

/// Состояние приложения: проверенный контент и дата, от которой считается стаж.
///
/// `today` приходит снаружи, а не берётся из часов внутри обработчика: иначе
/// тесты зависели бы от дня запуска.
#[derive(Clone, Copy)]
pub struct AppState {
    pub site: &'static Site,
    pub today: YearMonth,
}

/// Собирает приложение целиком.
pub fn app(site: &'static Site, today: YearMonth) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/terminal", get(terminal_page))
        .route("/terminal/exec", get(terminal_get).post(terminal_post))
        .route("/robots.txt", get(robots))
        .route("/sitemap.xml", get(sitemap))
        .route("/healthz", get(health))
        .route("/static/{*path}", get(static_files::serve))
        .fallback(not_found)
        .layer(CompressionLayer::new())
        .with_state(AppState { site, today })
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    page: Page,
}

#[derive(Template)]
#[template(path = "terminal.html")]
struct TerminalTemplate {
    page: Page,
    answer: terminal::Answer,
}

/// Фрагмент вывода для `hx-post` и подсказок.
#[derive(Template)]
#[template(path = "partials/answer.html")]
struct AnswerTemplate {
    answer: terminal::Answer,
}

/// Поле формы терминала.
#[derive(Debug, Deserialize)]
struct ExecForm {
    command: String,
}

/// Команда из подсказки: подсказка — ссылка, а не форма.
#[derive(Debug, Deserialize)]
struct ExecQuery {
    cmd: String,
}

/// Собирает страницу и убирает ссылки на файлы, которых нет в образе.
fn page(state: AppState) -> Page {
    let mut page = Page::build(state.site, state.today);
    page.resume = page.resume.filter(|resume| static_files::has(&resume.href));
    page
}

async fn index(State(state): State<AppState>) -> Response<Body> {
    render(IndexTemplate { page: page(state) })
}

async fn terminal_page(State(state): State<AppState>) -> Response<Body> {
    render(TerminalTemplate {
        page: page(state),
        answer: terminal::welcome(state.site),
    })
}

async fn terminal_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<ExecForm>,
) -> Response<Body> {
    terminal_respond(&form.command, &headers, state)
}

async fn terminal_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ExecQuery>,
) -> Response<Body> {
    terminal_respond(&query.cmd, &headers, state)
}

/// Ответ терминала: фрагмент для htmx и полная страница без JS.
fn terminal_respond(raw: &str, headers: &HeaderMap, state: AppState) -> Response<Body> {
    let answer = terminal::answer(raw, state.site);
    if is_htmx(headers) {
        render(AnswerTemplate { answer })
    } else {
        render(TerminalTemplate {
            page: page(state),
            answer,
        })
    }
}

/// htmx помечает свои запросы заголовком: по нему выбирается фрагмент вместо
/// целой страницы.
fn is_htmx(headers: &HeaderMap) -> bool {
    static HX_REQUEST: HeaderName = HeaderName::from_static("hx-request");
    headers
        .get(&HX_REQUEST)
        .and_then(|value| value.to_str().ok())
        == Some("true")
}

async fn robots() -> Response<Body> {
    text_response(
        "text/plain; charset=utf-8",
        format!("User-agent: *\nAllow: /\n\nSitemap: {SITE_URL}/sitemap.xml\n"),
    )
}

async fn sitemap() -> Response<Body> {
    text_response(
        "application/xml; charset=utf-8",
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n\
             <url><loc>{SITE_URL}/</loc></url>\n\
             <url><loc>{SITE_URL}/terminal</loc></url>\n\
             </urlset>\n"
        ),
    )
}

fn render<T: Template>(template: T) -> Response<Body> {
    match template.render() {
        Ok(html) => text_response("text/html; charset=utf-8", html),
        Err(error) => {
            eprintln!("не удалось отрисовать страницу: {error}");
            (StatusCode::INTERNAL_SERVER_ERROR, "страница не собралась").into_response()
        }
    }
}

/// Общие заголовки для всего, что отдаёт сервис: тип не угадывается, адрес
/// перехода не утекает на чужие домены.
fn text_response(content_type: &'static str, body: String) -> Response<Body> {
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-cache"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (header::REFERRER_POLICY, "strict-origin-when-cross-origin"),
        ],
        body,
    )
        .into_response()
}

async fn health() -> &'static str {
    "ok"
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "страница не найдена")
}
