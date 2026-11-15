//! Отдача вкомпилированной статики: CSS, шрифты, картинки, htmx.
//!
//! Файлы лежат в бинаре, поэтому «что в репозитории» и «что в образе» не могут
//! разойтись. `ETag` считается один раз на старте из байтов файла, браузер
//! получает `304` вместо повторной загрузки.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

use axum::body::Body;
use axum::extract::Path;
use axum::http::{HeaderMap, Response, StatusCode, header};
use include_dir::{Dir, include_dir};

/// Статика образа. Каталог читается на этапе компиляции.
static ASSETS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/static");

/// Шрифты и иконки меняются только при замене файла, поэтому их можно держать
/// в кэше неделю. У стилей имён с хешем нет: правка должна доезжать быстро.
const CACHE_LONG: &str = "public, max-age=604800";
const CACHE_SHORT: &str = "public, max-age=300, must-revalidate";

/// Отдаёт файл по пути внутри `static/`.
pub async fn serve(Path(path): Path<String>, headers: HeaderMap) -> Response<Body> {
    let Some(file) = ASSETS.get_file(&path) else {
        return missing();
    };
    let etag = etag_of(&path).unwrap_or_default();

    if is_not_modified(&headers, etag) {
        return Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, etag)
            .header(header::CACHE_CONTROL, cache_control(&path))
            .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
            .body(Body::empty())
            .expect("ответ 304 собирается");
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type(&path))
        .header(header::ETAG, etag)
        .header(header::CACHE_CONTROL, cache_control(&path))
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(file.contents()))
        .expect("ответ 200 собирается")
}

/// Есть ли файл в статике. Нужно, чтобы не отдавать ссылку на то, чего нет:
/// битая кнопка хуже отсутствующей.
pub fn has(href: &str) -> bool {
    let path = href.trim_start_matches('/');
    let path = path.strip_prefix("static/").unwrap_or(path);
    ASSETS.get_file(path).is_some()
}

/// `ETag` файла: хеш не криптографический, он нужен только чтобы отличить
/// версии одного и того же файла.
fn etag_of(path: &str) -> Option<&'static str> {
    static ETAGS: OnceLock<HashMap<&'static str, String>> = OnceLock::new();
    ETAGS
        .get_or_init(|| {
            ASSETS
                .files()
                .filter_map(|file| {
                    let path = file.path().to_str()?;
                    let mut hasher = DefaultHasher::new();
                    file.contents().hash(&mut hasher);
                    Some((path, format!("\"{:016x}\"", hasher.finish())))
                })
                .collect()
        })
        .get(path)
        .map(String::as_str)
}

/// `If-None-Match` может перечислять несколько тегов и содержать префикс `W/`.
fn is_not_modified(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .any(|token| token.trim().trim_start_matches("W/") == etag)
        })
}

fn cache_control(path: &str) -> &'static str {
    match extension(path) {
        "woff2" | "ico" | "png" | "svg" => CACHE_LONG,
        _ => CACHE_SHORT,
    }
}

/// Тип содержимого по расширению: список закрыт тем, что реально лежит в
/// `static/`, а не полной таблицей MIME.
fn content_type(path: &str) -> &'static str {
    match extension(path) {
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "woff2" => "font/woff2",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

fn extension(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or_default()
}

fn missing() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from("файл не найден"))
        .expect("ответ 404 собирается")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_embedded_file_gets_an_etag() {
        assert!(ASSETS.files().next().is_some(), "статика вкомпилирована");
        for file in ASSETS.files() {
            let path = file.path().to_str().expect("utf-8 путь");
            assert!(etag_of(path).is_some(), "нет ETag для {path}");
        }
    }

    #[test]
    fn etag_is_quoted_and_stable() {
        let etag = etag_of("app.css").expect("app.css в статике");
        assert!(etag.starts_with('"') && etag.ends_with('"'), "{etag}");
        assert_eq!(etag, etag_of("app.css").expect("app.css в статике"));
    }

    #[test]
    fn content_types_cover_the_static_directory() {
        assert_eq!(content_type("app.css"), "text/css; charset=utf-8");
        assert_eq!(content_type("fonts/plex.woff2"), "font/woff2");
        assert_eq!(
            content_type("htmx.min.js"),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(content_type("og.png"), "image/png");
    }

    #[test]
    fn styles_revalidate_and_fonts_cache_long() {
        assert!(cache_control("app.css").contains("must-revalidate"));
        assert!(cache_control("terminal.css").contains("must-revalidate"));
        assert!(cache_control("fonts/plex.woff2").contains("604800"));
        assert!(cache_control("og.png").contains("604800"));
    }

    #[test]
    fn has_accepts_page_and_static_paths() {
        assert!(has("/static/app.css"));
        assert!(has("app.css"));
        assert!(!has("/static/missing.pdf"));
    }
}
