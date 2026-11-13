//! Представление для шаблонов: готовые строки и посчитанная полоса карьеры.
//!
//! Домен остаётся в [`crate::content`]: здесь только то, что печатается, —
//! названия месяцев по-русски, склонение стажа, «сейчас» вместо даты.

use crate::content::date::End;
use crate::content::span::{Tenure, Timeline};
use crate::content::{Profile, Site, YearMonth};

/// Всё, что нужно обычной версии страницы.
#[derive(Debug)]
pub struct Page {
    pub name: String,
    pub role: String,
    pub summary: String,
    /// Стаж словами: «8 лет 2 месяца».
    pub tenure: String,
    pub about: Vec<String>,
    pub facts: Vec<Fact>,
    pub timeline: Timeline,
    pub stack: Vec<StackRow>,
    pub roles: Vec<RoleRow>,
    pub projects: Vec<ProjectRow>,
    pub contacts: Vec<LinkRow>,
    pub resume: Option<LinkRow>,
    /// Адрес для кнопки «Написать».
    pub email: Option<String>,
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
            email: email_of(profile),
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

fn role_row(role: &crate::content::Role) -> RoleRow {
    RoleRow {
        period: format!("{} — {}", month_label(role.start), end_label(role.end)),
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
        href: link.href.clone(),
    }
}

/// Адрес почты из первого контакта `mailto:`.
fn email_of(profile: &Profile) -> Option<String> {
    profile
        .contacts
        .iter()
        .find_map(|link| link.href.strip_prefix("mailto:").map(str::to_owned))
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
    format!("{name} {}", month.year())
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
    format!("{count} {word}")
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
        assert_eq!(month_label(month("2024-05")), "май 2024");
        assert_eq!(month_label(month("2022-01")), "январь 2022");
        assert_eq!(end_label(End::Now), "сейчас");
    }

    #[test]
    fn tenure_declines_in_russian() {
        let cases = [
            ("2018-09", "2026-11", "8 лет 3 месяца"),
            ("2025-12", "2026-11", "1 год"),
            ("2026-10", "2026-11", "2 месяца"),
            ("2026-09", "2026-11", "3 месяца"),
            ("2026-02", "2026-11", "10 месяцев"),
            ("2026-01", "2026-11", "11 месяцев"),
            ("2015-12", "2026-11", "11 лет"),
            ("2020-02", "2026-11", "6 лет 10 месяцев"),
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
        assert_eq!(page.roles[0].period, "май 2024 — сейчас");
        assert_eq!(page.roles[4].period, "сентябрь 2018 — декабрь 2019");
        assert_eq!(page.tenure, "8 лет 3 месяца");
        assert_eq!(page.email.as_deref(), Some("fenrir1121@gmail.com"));
        assert!(
            page.facts.iter().any(|fact| fact.label == "Код"
                && fact.href.as_deref() == Some("https://github.com/fennr")),
            "ссылка на код без схемы в тексте"
        );
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
