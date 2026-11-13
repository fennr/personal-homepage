//! Точка входа бинаря: проверяет контент, поднимает HTTP-сервер.
//!
//! Контент проверяется на старте: сервис, который отдал страницу без данных,
//! хуже сервиса, который не запустился.

use std::net::SocketAddr;
use std::process::ExitCode;

use chrono::Datelike;
use personal_homepage::content::{Site, YearMonth};
use personal_homepage::web;

/// Порт по умолчанию: внутри контейнера `0.0.0.0:8080`, локально — только
/// петля, чтобы сервис не торчал в сеть с ноутбука.
const DEFAULT_BIND: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> ExitCode {
    let site = match Site::embedded() {
        Ok(site) => site,
        Err(errors) => {
            eprintln!("{errors}");
            return ExitCode::FAILURE;
        }
    };

    let bind = match bind_address() {
        Ok(bind) => bind,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };

    let listener = match tokio::net::TcpListener::bind(bind).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("не удалось занять {bind}: {error}");
            return ExitCode::FAILURE;
        }
    };
    println!("слушаю http://{bind}");

    let app = web::app(site, today());
    match axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("сервер остановился с ошибкой: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Адрес прослушивания из `PERSONAL_HOMEPAGE_BIND`.
fn bind_address() -> Result<SocketAddr, String> {
    match std::env::var("PERSONAL_HOMEPAGE_BIND") {
        Ok(raw) => raw
            .parse()
            .map_err(|error| format!("PERSONAL_HOMEPAGE_BIND={raw:?}: {error}")),
        Err(_) => DEFAULT_BIND
            .parse()
            .map_err(|error| format!("адрес по умолчанию не разобрался: {error}")),
    }
}

/// Сегодняшний месяц по местному времени: от него считаются стаж и полоса
/// карьеры.
fn today() -> YearMonth {
    let now = chrono::Local::now().date_naive();
    YearMonth::new(
        now.year() as u16,
        u8::try_from(now.month()).expect("месяц 1–12"),
    )
    .expect("текущий месяц существует")
}

/// Остановка по Ctrl+C и по SIGTERM: Docker присылает второе.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => eprintln!("не удалось подписаться на SIGTERM: {error}"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}
