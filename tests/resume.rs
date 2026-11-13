//! Данные резюме в репозитории обязаны совпадать с контентом: иначе PDF уедет
//! на прод устаревшим, а страница будет ссылаться на старую версию.

use personal_homepage::content::{Site, YearMonth};
use personal_homepage::resume;

#[test]
fn committed_resume_data_matches_the_content() {
    let site = Site::embedded().expect("контент валиден");
    let committed = include_str!("../resume/data.json");

    // Месяц расчёта записан в самих данных: тест пересобирает их на ту же дату
    // и не зависит от сегодняшнего числа.
    let value: serde_json::Value =
        serde_json::from_str(committed).expect("resume/data.json — валидный JSON");
    let month = value["generated_for"]
        .as_str()
        .expect("в данных есть generated_for");
    let today = YearMonth::parse(month).expect("месяц записан как YYYY-MM");

    assert_eq!(
        resume::export(site, today).trim(),
        committed.trim(),
        "данные резюме устарели: запустите `just resume`"
    );
}

#[test]
fn the_pdf_is_published() {
    let pdf = include_bytes!("../static/resume.pdf");
    assert!(pdf.starts_with(b"%PDF"), "static/resume.pdf — не PDF");
}
