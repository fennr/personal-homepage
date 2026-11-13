//! Представление для шаблонов: готовые строки и посчитанная полоса карьеры.
//!
//! Домен остаётся в [`crate::content`]: здесь только то, что печатается, —
//! названия месяцев по-русски, склонение стажа, «сейчас» вместо даты.

use crate::content::date::End;
use crate::content::span::{Tenure, Timeline};
use crate::content::{Role, Site, YearMonth};

/// Всё, что нужно обычной версии страницы.
#[derive(Debug)]
pub struct Page {
    pub name: String,
    pub role: String,
    pub summary: String,
    /// Стаж словами: «8 лет 2 месяца».
    pub tenure: String,
    /// Подпись полосы карьеры для скринридера.
    pub career_label: String,
    pub about: Vec<String>,
    pub facts: Vec<Fact>,
    pub timeline: Timeline,
    pub stack: Vec<StackRow>,
    pub roles: Vec<RoleRow>,
    pub projects: Vec<ProjectRow>,
    pub contacts: Vec<LinkRow>,
    pub resume: Option<LinkRow>,
    /// Основной контакт: первый в файле, печатается кнопкой в шапке.
    pub primary: Option<LinkRow>,
}

/// Пара «подпись — значение» в шапке; значение может быть ссылкой.
#[derive(Debug)]
pub struct Fact {
    pub label: String,
    pub value: String,
    pub href: Option<String>,
}

/// Группа стека с отмеченными главными технологиями.
#[derive(Debug)]
pub struct StackRow {
    pub title: String,
    pub items: Vec<StackItem>,
}

/// Технология в группе.
#[derive(Debug)]
pub struct StackItem {
    pub text: String,
    pub emphasis: bool,
}

/// Место работы в том виде, в котором оно читается.
#[derive(Debug)]
pub struct RoleRow {
    /// «май 2024 — сейчас».
    pub period: String,
    pub company: String,
    pub title: String,
    pub place: String,
    pub summary: Option<String>,
    pub bullets: Vec<String>,
}

/// Проект: имя, ссылка, одна строка о смысле, стек строкой.
#[derive(Debug)]
pub struct ProjectRow {
    pub name: String,
    pub href: String,
    pub what: String,
    pub tech: String,
}

/// Контакт или файл резюме.
#[derive(Debug)]
pub struct LinkRow {
    pub label: String,
    /// Как найти внутри сервиса: `@fenn_r`.
    pub handle: Option<String>,
    pub href: String,
}

impl Page {
    /// Собирает страницу на дату `today`: от неё зависят стаж и полоса карьеры.
    pub fn build(site: &Site, today: YearMonth) -> Self {
        let profile = &site.profile;
        let tenure = Tenure::between(profile.career_start, today);
        let github = profile
            .contacts
            .iter()
            .find(|link| link.href.contains("github.com"))
            .map(|link| link.href.clone());

        let mut facts = vec![
            Fact {
                label: "Место".to_owned(),
                value: profile.location.clone(),
                href: None,
            },
            Fact {
                label: "Опыт".to_owned(),
                value: tenure_label(tenure),
                href: None,
            },
        ];
        if let Some(current) = site.roles.first().filter(|role| role.end.is_now()) {
            facts.push(Fact {
                label: "Сейчас".to_owned(),
                value: format!("{}, {}", current.company, current.title.to_lowercase()),
                href: None,
            });
        }
        if let Some(href) = &github {
            facts.push(Fact {
                label: "Код".to_owned(),
                value: short_href(href),
                href: Some(href.clone()),
            });
        }

        Self {
            name: profile.name.clone(),
            role: profile.role.clone(),
            summary: profile.summary.clone(),
            tenure: tenure_label(tenure),
            career_label: career_label(&site.roles, today),
            about: profile.about.clone(),
            facts,
            timeline: Timeline::build(&site.roles, today),
            stack: site.stack.iter().map(stack_row).collect(),
            roles: site.roles.iter().map(role_row).collect(),
            projects: site
                .projects
                .iter()
                .map(|project| ProjectRow {
                    name: project.name.clone(),
                    href: project.href.clone(),
                    what: project.what.clone(),
                    tech: project.tech.join(", "),
                })
                .collect(),
            contacts: profile.contacts.iter().map(link_row).collect(),
            resume: profile.resume.as_ref().map(link_row),
            primary: profile.contacts.first().map(link_row),
        }
    }
}

fn stack_row(group: &crate::content::StackGroup) -> StackRow {
    StackRow {
        title: group.title.clone(),
        items: group
            .items
            .iter()
            .map(|item| StackItem {
                text: item.clone(),
                emphasis: group.emphasis.contains(item),
            })
            .collect(),
    }
}

fn role_row(role: &Role) -> RoleRow {
    RoleRow {
        // Диапазон разрывается перед тире, а не после: в колонке 118 px он
        // почти всегда не влезает в строку, и «—» в конце строки читается как
        // обрывок. Неразрывный пробел держит тире рядом с датой конца.
        period: format!("{} —\u{a0}{}", month_label(role.start), end_label(role.end)),
        company: role.company.clone(),
        title: role.title.clone(),
        place: role.place.clone(),
        summary: role.summary.clone(),
        bullets: role.bullets.clone(),
    }
}

fn link_row(link: &crate::content::Link) -> LinkRow {
    LinkRow {
        label: link.label.clone(),
        handle: link.handle.clone(),
        href: link.href.clone(),
    }
}

/// Адрес без схемы и хвостового слэша: так он читается в шапке.
fn short_href(href: &str) -> String {
    href.trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_owned()
}

/// «май 2024» — в русском месяцы пишутся строчными.
pub fn month_label(month: YearMonth) -> String {
    const NAMES: [&str; 12] = [
        "январь",
        "февраль",
        "март",
        "апрель",
        "май",
        "июнь",
        "июль",
        "август",
        "сентябрь",
        "октябрь",
        "ноябрь",
        "декабрь",
    ];
    let name = NAMES[usize::from(month.month() - 1)];
    // Неразрывный пробел: «май» не должно остаться на строке без года.
    format!("{name}\u{a0}{}", month.year())
}

/// Конец периода словами: дата или «сейчас».
pub fn end_label(end: End) -> String {
    match end {
        End::Month(month) => month_label(month),
        End::Now => "сейчас".to_owned(),
    }
}

/// Стаж словами с русским склонением: «8 лет 2 месяца», «1 год», «11 месяцев».
pub fn tenure_label(tenure: Tenure) -> String {
    let years = tenure.years();
    let months = tenure.rest_months();
    match (years, months) {
        (0, 0) => "меньше месяца".to_owned(),
        (0, months) => plural(months, "месяц", "месяца", "месяцев"),
        (years, 0) => plural(years, "год", "года", "лет"),
        (years, months) => format!(
            "{} {}",
            plural(years, "год", "года", "лет"),
            plural(months, "месяц", "месяца", "месяцев")
        ),
    }
}

/// Русское склонение по числу: 1 год, 2 года, 5 лет, 11 лет, 21 год.
///
/// Число и слово связываются неразрывным пробелом: «8 лет» не должно разорваться
/// между цифрой и словом.
fn plural(count: i32, one: &str, few: &str, many: &str) -> String {
    let last_two = count % 100;
    let last = count % 10;
    let word = if (11..=14).contains(&last_two) {
        many
    } else if last == 1 {
        one
    } else if (2..=4).contains(&last) {
        few
    } else {
        many
    };
    format!("{count}\u{a0}{word}")
}

/// Подпись полосы карьеры: полоса — картинка, поэтому у неё есть описание, а
/// даты и подробности остаются в разделе «Опыт». Имена — те же, что на полосе.
fn career_label(roles: &[Role], today: YearMonth) -> String {
    let Some(oldest) = roles.last() else {
        return "Карьера".to_owned();
    };
    let companies = roles
        .iter()
        .rev()
        .map(|role| role.brand.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let years = format!("{}–{}", oldest.start.year(), today.year());
    match roles.first().filter(|role| role.end.is_now()) {
        Some(current) => format!("Карьера, {years}: {companies}. Сейчас — {}.", current.brand),
        None => format!("Карьера, {years}: {companies}."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Site;

    fn month(raw: &str) -> YearMonth {
        YearMonth::parse(raw).expect("валидный месяц")
    }

    #[test]
    fn months_are_written_lowercase_with_the_year() {
        assert_eq!(month_label(month("2024-05")), "май\u{a0}2024");
        assert_eq!(month_label(month("2022-01")), "январь\u{a0}2022");
        assert_eq!(end_label(End::Now), "сейчас");
    }

    #[test]
    fn tenure_declines_in_russian() {
        let cases = [
            ("2018-09", "2026-11", "8\u{a0}лет 3\u{a0}месяца"),
            ("2025-12", "2026-11", "1\u{a0}год"),
            ("2026-10", "2026-11", "2\u{a0}месяца"),
            ("2026-09", "2026-11", "3\u{a0}месяца"),
            ("2026-02", "2026-11", "10\u{a0}месяцев"),
            ("2026-01", "2026-11", "11\u{a0}месяцев"),
            ("2015-12", "2026-11", "11\u{a0}лет"),
            ("2020-02", "2026-11", "6\u{a0}лет 10\u{a0}месяцев"),
        ];
        for (start, today, expected) in cases {
            let label = tenure_label(Tenure::between(month(start), month(today)));
            assert_eq!(label, expected, "{start} — {today}");
        }

        let empty = Tenure::between(month("2026-11"), month("2020-01"));
        assert_eq!(tenure_label(empty), "меньше месяца");
    }

    #[test]
    fn page_carries_the_current_role_first() {
        let site = Site::embedded().expect("контент валиден");
        let page = Page::build(site, month("2026-11"));
        assert_eq!(page.name, "Виктор Меркурьев");
        assert_eq!(page.roles.len(), site.roles.len());
        assert_eq!(page.roles[0].period, "май\u{a0}2024 —\u{a0}сейчас");
        assert_eq!(
            page.roles[4].period,
            "сентябрь\u{a0}2018 —\u{a0}декабрь\u{a0}2019"
        );
        assert_eq!(page.tenure, "8\u{a0}лет 3\u{a0}месяца");
        let primary = page.primary.as_ref().expect("основной контакт");
        assert_eq!(primary.label, "Telegram");
        assert_eq!(primary.href, "https://t.me/fenn_r");
        assert_eq!(primary.handle.as_deref(), Some("@fenn_r"));
        assert_eq!(
            page.contacts.len(),
            5,
            "подвал перечисляет все контакты, а не только основной"
        );
        assert!(
            page.facts.iter().any(|fact| fact.label == "Код"
                && fact.href.as_deref() == Some("https://github.com/fennr")),
            "ссылка на код без схемы в тексте"
        );
    }

    #[test]
    fn career_label_describes_the_bar_for_screen_readers() {
        let site = Site::embedded().expect("контент валиден");
        let page = Page::build(site, month("2026-11"));
        assert_eq!(
            page.career_label,
            "Карьера, 2018–2026: BC Soft, Saprun, Promobot, Ujin, Uralchem. Сейчас — Uralchem."
        );
    }

    #[test]
    fn numbers_are_glued_to_their_words() {
        // Неразрывный пробел: «8 лет» и «май 2024» не должны разрываться по словам.
        let site = Site::embedded().expect("контент валиден");
        let page = Page::build(site, month("2026-11"));
        assert!(!page.tenure.contains(" лет"), "число и слово склеены");
        assert!(
            !page.roles[0].period.contains("май "),
            "месяц и год склеены"
        );
    }

    #[test]
    fn stack_group_titles_name_a_single_theme() {
        let site = Site::embedded().expect("контент валиден");
        let page = Page::build(site, month("2026-11"));
        for row in &page.stack {
            assert!(
                !row.title.contains(" и "),
                "заголовок {:?} перечисляет две темы",
                row.title
            );
        }
    }

    #[test]
    fn projects_are_listed_with_their_stack() {
        let site = Site::embedded().expect("контент валиден");
        let page = Page::build(site, month("2026-11"));
        let zipper = page
            .projects
            .iter()
            .find(|project| project.name == "zipper")
            .expect("проект zipper");
        assert_eq!(zipper.tech, "Rust, CLI");
    }
}
