//! Контент обязан проходить все правила, а каждый вид поломки — сообщать файл
//! и поле. Сообщение об ошибке здесь такой же контракт, как и сам разбор:
//! контент правят руками, и по тексту ошибки должно быть понятно, куда идти.

use std::fs;
use std::path::Path;

use personal_homepage::content::{End, Site};

/// Контент, лежащий в репозитории: с него начинается каждый тест.
const VALID: [(&str, &str); 4] = [
    ("profile.toml", include_str!("../content/profile.toml")),
    ("stack.toml", include_str!("../content/stack.toml")),
    (
        "experience.toml",
        include_str!("../content/experience.toml"),
    ),
    ("projects.toml", include_str!("../content/projects.toml")),
];

/// Каталог с валидным контентом, где в одном файле сделана правка.
///
/// Если правка не нашлась, тест падает сразу: иначе он проверял бы нетронутый
/// контент и молча проходил.
fn with_edit(file: &str, from: &str, to: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("временный каталог");
    for (name, text) in VALID {
        let text = if name == file {
            assert!(text.contains(from), "в {file} нет {from:?}");
            text.replace(from, to)
        } else {
            text.to_owned()
        };
        fs::write(dir.path().join(name), text).expect("запись файла");
    }
    dir
}

fn error_of(dir: &Path) -> String {
    Site::load(dir).expect_err("ожидалась ошибка").to_string()
}

#[test]
fn embedded_content_satisfies_every_rule() {
    let site = Site::embedded().expect("контент в репозитории должен быть валидным");
    assert_eq!(site.roles.len(), 5, "мест работы");
    assert_eq!(site.projects.len(), 6, "проектов");
    assert!(!site.stack.is_empty(), "групп стека");
    assert!(
        site.roles[0].end.is_now(),
        "текущее место работы идёт первым"
    );
}

#[test]
fn broken_syntax_names_the_file() {
    let dir = with_edit("stack.toml", "title = \"Языки\"", "title = \"Языки");
    let error = error_of(dir.path());
    assert!(error.starts_with("stack.toml:"), "{error}");
}

#[test]
fn missing_file_names_itself() {
    let dir = with_edit("projects.toml", "", "");
    fs::remove_file(dir.path().join("projects.toml")).expect("удаление файла");
    let error = error_of(dir.path());
    assert!(error.contains("projects.toml"), "{error}");
    assert!(error.contains("не удалось прочитать"), "{error}");
}

#[test]
fn empty_required_field_points_to_it() {
    let dir = with_edit(
        "profile.toml",
        "name = \"Виктор Меркурьев\"",
        "name = \"  \"",
    );
    let error = error_of(dir.path());
    assert!(error.contains("profile.toml: name"), "{error}");
    assert!(error.contains("пустым"), "{error}");
}

#[test]
fn month_must_have_four_digits_and_two_month_digits() {
    let dir = with_edit(
        "experience.toml",
        "start = \"2020-01\"",
        "start = \"20-01\"",
    );
    let error = error_of(dir.path());
    assert!(error.contains("experience.toml"), "{error}");
    assert!(error.contains("YYYY-MM"), "{error}");
}

#[test]
fn role_end_before_start_is_reported() {
    let dir = with_edit("experience.toml", "end = \"2022-10\"", "end = \"2019-01\"");
    let error = error_of(dir.path());
    assert!(error.contains("experience.toml: roles[3].end"), "{error}");
    assert!(error.contains("раньше начала"), "{error}");
}

#[test]
fn current_role_must_be_first() {
    let dir = with_edit("experience.toml", "end = \"2022-10\"", "end = \"now\"");
    let error = error_of(dir.path());
    assert!(error.contains("experience.toml: roles[3].end"), "{error}");
    assert!(error.contains("первым в файле"), "{error}");
}

#[test]
fn roles_go_from_newest_to_oldest() {
    let dir = with_edit(
        "experience.toml",
        "start = \"2018-09\"",
        "start = \"2025-01\"",
    );
    let error = error_of(dir.path());
    assert!(error.contains("experience.toml: roles[4].start"), "{error}");
    assert!(error.contains("позже начала предыдущего"), "{error}");
}

#[test]
fn emphasis_must_belong_to_its_group() {
    let dir = with_edit(
        "stack.toml",
        "emphasis = [\"Python\", \"Go\", \"Rust\"]",
        "emphasis = [\"Python\", \"Go\", \"Rust\", \"Haskell\"]",
    );
    let error = error_of(dir.path());
    assert!(error.contains("stack.toml: groups[0].emphasis"), "{error}");
    assert!(error.contains("Haskell"), "{error}");
}

#[test]
fn project_link_must_be_absolute() {
    let dir = with_edit(
        "projects.toml",
        "href = \"https://github.com/fennr/zipper\"",
        "href = \"/zipper\"",
    );
    let error = error_of(dir.path());
    assert!(error.contains("projects.toml: projects[3].href"), "{error}");
    assert!(error.contains("абсолютной"), "{error}");
}

#[test]
fn duplicate_project_names_are_reported() {
    let dir = with_edit(
        "projects.toml",
        "name = \"credinform_cli\"",
        "name = \"zipper\"",
    );
    let error = error_of(dir.path());
    assert!(error.contains("projects.toml: projects[5].name"), "{error}");
    assert!(error.contains("уже объявлен"), "{error}");
}

#[test]
fn unknown_field_is_not_silently_ignored() {
    let dir = with_edit("experience.toml", "place = \"Пермь\"", "places = \"Пермь\"");
    let error = error_of(dir.path());
    assert!(error.contains("experience.toml"), "{error}");
    assert!(error.contains("places"), "{error}");
}

#[test]
fn contact_needs_a_supported_scheme() {
    let dir = with_edit(
        "profile.toml",
        "href = \"https://t.me/fenn_r\"",
        "href = \"t.me/fenn_r\"",
    );
    let error = error_of(dir.path());
    assert!(error.contains("profile.toml: contact[0].href"), "{error}");
    assert!(error.contains("должен начинаться"), "{error}");
}

#[test]
fn brand_on_the_career_bar_is_written_in_latin() {
    let dir = with_edit(
        "experience.toml",
        "brand = \"Uralchem\"",
        "brand = \"Уралхим\"",
    );
    let error = error_of(dir.path());
    assert!(error.contains("experience.toml: roles[0].brand"), "{error}");
    assert!(error.contains("латиницей"), "{error}");
}

#[test]
fn every_violation_is_reported_in_one_run() {
    let dir = with_edit(
        "profile.toml",
        "role = \"Бэкенд-разработчик и архитектор\"",
        "role = \"\"",
    );
    let dir = {
        // Вторая поломка в другом файле: обе должны попасть в один отчёт.
        let text = fs::read_to_string(dir.path().join("experience.toml")).expect("чтение");
        fs::write(
            dir.path().join("experience.toml"),
            text.replace("end = \"2022-10\"", "end = \"2019-01\""),
        )
        .expect("запись");
        dir
    };

    let site = Site::load(dir.path());
    let error = site.expect_err("ожидалась ошибка").to_string();
    assert!(error.contains("profile.toml: role"), "{error}");
    assert!(error.contains("experience.toml: roles[3].end"), "{error}");
    assert_eq!(error.lines().count(), 2, "{error}");
}

#[test]
fn end_marks_the_open_period() {
    let site = Site::embedded().expect("контент валиден");
    assert_eq!(site.roles[0].end, End::Now);
    assert!(site.roles[1].end.month().is_some());
}
