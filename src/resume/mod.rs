//! Данные для PDF-резюме.
//!
//! PDF собирает Typst, а данные он читает из JSON — того же представления,
//! которое печатает страница. Поэтому даты, стаж и склонения считаются в одном
//! месте, и резюме не расходится со страницей.
//!
//! `resume/data.json` лежит в репозитории и обновляется `just resume`; тест
//! сравнивает файл с текущим контентом и ловит забытую пересборку PDF.

use serde::Serialize;

use crate::content::{Site, YearMonth};
use crate::web::view::Page;

/// Данные резюме: то же, что видно на странице, плюс месяц расчёта.
#[derive(Serialize)]
pub struct Resume<'a> {
    /// Месяц, на который посчитаны стаж и открытые периоды. По нему тест
    /// пересобирает данные: сравнение не зависит от сегодняшней даты.
    pub generated_for: String,
    #[serde(flatten)]
    pub page: &'a Page,
}

/// JSON для шаблона Typst.
pub fn export(site: &Site, today: YearMonth) -> String {
    let page = Page::build(site, today);
    let resume = Resume {
        generated_for: today.to_string(),
        page: &page,
    };
    serde_json::to_string_pretty(&resume).expect("данные резюме сериализуются")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn month(raw: &str) -> YearMonth {
        YearMonth::parse(raw).expect("валидный месяц")
    }

    #[test]
    fn export_carries_the_page_and_the_month() {
        let site = Site::embedded().expect("контент валиден");
        let json = export(site, month("2026-11"));
        assert!(json.contains("\"generated_for\": \"2026-11\""), "{json}");
        assert!(json.contains("\"roles\""), "опыт в резюме нужен");
        assert!(json.contains("\"period\": \"май\u{a0}2024 —\u{a0}сейчас\""));
        assert!(json.contains("\"stack\""), "стек в резюме нужен");
    }
}
