//! Точка входа бинаря.
//!
//! На этом шаге проверяет вкомпилированный контент и печатает его состав:
//! HTTP-сервер появится следующим шагом, а проверка контента останется в
//! `cargo test`, чтобы падение было видно до сборки образа.

use std::process::ExitCode;

use personal_homepage::content::Site;

fn main() -> ExitCode {
    match Site::embedded() {
        Ok(site) => {
            println!(
                "контент проверен: {} мест работы, {} проектов, {} групп стека",
                site.roles.len(),
                site.projects.len(),
                site.stack.len()
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("{errors}");
            ExitCode::FAILURE
        }
    }
}
