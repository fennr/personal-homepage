//! HTTP-слой: маршруты, шаблоны и состояние приложения.

mod static_files;
pub mod view;

use askama::Template;
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Response, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::get;
use tower_http::compression::CompressionLayer;

use crate::content::{Site, YearMonth};
use view::Page;

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

async fn index(State(state): State<AppState>) -> Response<Body> {
    let template = IndexTemplate {
        page: Page::build(state.site, state.today),
    };
    match template.render() {
        Ok(html) => (
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            html,
        )
            .into_response(),
        Err(error) => {
            eprintln!("не удалось отрисовать страницу: {error}");
            (StatusCode::INTERNAL_SERVER_ERROR, "страница не собралась").into_response()
        }
    }
}

async fn health() -> &'static str {
    "ok"
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "страница не найдена")
}
