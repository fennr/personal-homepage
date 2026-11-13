//! Отдача вкомпилированной статики: CSS, шрифты, htmx.
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

const CACHE_CONTROL: &str = "public, max-age=86400";

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
            .header(header::CACHE_CONTROL, CACHE_CONTROL)
            .body(Body::empty())
            .expect("ответ 304 собирается");
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type(&path))
        .header(header::ETAG, etag)
        .header(header::CACHE_CONTROL, CACHE_CONTROL)
        .body(Body::from(file.contents()))
        .expect("ответ 200 собирается")
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

/// Тип содержимого по расширению: список закрыт тем, что реально лежит в
/// `static/`, а не полной таблицей MIME.
fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or_default() {
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
        assert!(!ASSETS.files().next().is_none(), "статика вкомпилирована");
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
        assert_eq!(content_type("fonts/f0.woff2"), "font/woff2");
        assert_eq!(
            content_type("htmx.min.js"),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(content_type("resume.pdf"), "application/pdf");
    }
}
