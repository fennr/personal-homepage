//! Точка входа бинаря: сервер и подкоманда `health`.
//!
//! Контент проверяется на старте: сервис, который отдал страницу без данных,
//! хуже сервиса, который не запустился.
//!
//! Подкоманда `health` нужна контейнеру: в distroless нет ни shell, ни curl,
//! поэтому `HEALTHCHECK` вызывает тот же бинарь, а тот сам делает
//! `GET /healthz` и возвращает код выхода.
//!
//! Подкоманда `resume` печатает данные для PDF: их читает шаблон Typst при
//! сборке `just resume`.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::ExitCode;
use std::time::Duration;

use chrono::Datelike;
use personal_homepage::content::{Site, YearMonth};
use personal_homepage::web;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Порт по умолчанию: внутри контейнера `0.0.0.0:8080`, локально — только
/// петля, чтобы сервис не торчал в сеть с ноутбука.
const DEFAULT_BIND: &str = "127.0.0.1:8080";

/// Сколько ждать ответа `healthz`: этого хватает, чтобы отличить занятый
/// процесс от зависшего.
const HEALTH_TIMEOUT: Duration = Duration::from_secs(2);

#[tokio::main]
async fn main() -> ExitCode {
    match std::env::args().nth(1) {
        None => serve().await,
        Some(argument) if argument == "health" => health().await,
        Some(argument) if argument == "resume" => resume(),
        Some(argument) => {
            eprintln!("неизвестная подкоманда {argument:?}: поддерживаются health и resume");
            ExitCode::from(2)
        }
    }
}

/// Запускает HTTP-сервер.
async fn serve() -> ExitCode {
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

/// Печатает JSON для шаблона PDF-резюме.
fn resume() -> ExitCode {
    let site = match Site::embedded() {
        Ok(site) => site,
        Err(errors) => {
            eprintln!("{errors}");
            return ExitCode::FAILURE;
        }
    };
    println!("{}", personal_homepage::resume::export(site, today()));
    ExitCode::SUCCESS
}

/// Проверяет `healthz` на том же порту, где слушает сервер.
async fn health() -> ExitCode {
    let target = match bind_address() {
        Ok(bind) => SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), bind.port()),
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };

    match probe(target).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("healthz не отвечает на {target}: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Минимальный HTTP-запрос: без клиентской библиотеки, потому что больше от
/// проверки ничего не нужно.
async fn probe(target: SocketAddr) -> Result<(), String> {
    let response = async {
        let mut stream = tokio::net::TcpStream::connect(target)
            .await
            .map_err(|error| format!("подключение: {error}"))?;
        let request =
            format!("GET /healthz HTTP/1.0\r\nHost: {target}\r\nConnection: close\r\n\r\n");
        stream
            .write_all(request.as_bytes())
            .await
            .map_err(|error| format!("запрос: {error}"))?;
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .await
            .map_err(|error| format!("ответ: {error}"))?;
        Ok::<String, String>(response)
    };

    let response = tokio::time::timeout(HEALTH_TIMEOUT, response)
        .await
        .map_err(|_| format!("нет ответа за {HEALTH_TIMEOUT:?}"))??;

    if response.starts_with("HTTP/1.0 200") || response.starts_with("HTTP/1.1 200") {
        Ok(())
    } else {
        Err(format!(
            "ответ без кода 200: {:?}",
            response.lines().next().unwrap_or_default()
        ))
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
