//! Сырые данные из `content/*.toml`.
//!
//! Каждое поле либо обязательно, либо `Option`/пустой список: шаблоны не
//! печатают блоки, которых нет, и не показывают пустые заголовки.
//! `deny_unknown_fields` превращает опечатку в имени поля в ошибку загрузки,
//! а не в молча потерянный текст.

use serde::Deserialize;

use super::date::{End, YearMonth};

/// `profile.toml` — кто, где и как связаться.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub name: String,
    pub role: String,
    /// Одно предложение под именем: что делаю и для кого.
    pub summary: String,
    pub location: String,
    /// Месяц начала карьеры: от него считается стаж на странице.
    pub career_start: YearMonth,
    /// Абзацы блока «о себе».
    #[serde(default)]
    pub about: Vec<String>,
    /// Файл резюме; отсутствует, пока резюме не собрано.
    #[serde(default)]
    pub resume: Option<Link>,
    /// Порядок — порядок вывода: первый контакт печатается кнопкой в шапке и
    /// стоит первым в подвале, поэтому первым идёт тот, кому пишут чаще.
    #[serde(default, rename = "contact")]
    pub contacts: Vec<Link>,
}

/// Ссылка в интерфейсе: подпись для человека и адрес.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub label: String,
    pub href: String,
}

/// `stack.toml` — технологии, сгруппированные по назначению.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stack {
    #[serde(default, rename = "group")]
    pub groups: Vec<StackGroup>,
}

/// Одна группа технологий: заголовок и состав.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StackGroup {
    pub title: String,
    pub items: Vec<String>,
    /// Технологии, которые в группе главные: печатаются полужирным.
    /// Проверяется, что каждая входит в `items`.
    #[serde(default)]
    pub emphasis: Vec<String>,
}

/// `experience.toml` — места работы от новых к давним.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Experience {
    #[serde(default, rename = "role")]
    pub roles: Vec<Role>,
}

/// Место работы.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Role {
    /// Название в тексте: как компания называется в русской прозе.
    pub company: String,
    /// Имя на полосе карьеры: латиницей и в одном написании у всех мест,
    /// иначе подписи соседних отрезков оказываются в разных алфавитах.
    pub brand: String,
    pub title: String,
    /// Город, а для давних мест — ещё и перечень клиентов.
    pub place: String,
    pub start: YearMonth,
    /// `"now"` для текущего места работы.
    pub end: End,
    /// Одна строка вместо списка: для мест, где важен факт, а не детали.
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub bullets: Vec<String>,
}

/// `projects.toml` — публичные проекты, которые стоит показать.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Projects {
    #[serde(default, rename = "project")]
    pub projects: Vec<Project>,
}

/// Один проект: имя, ссылка, одна строка о смысле и стек.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub name: String,
    pub href: String,
    /// Что это и зачем — одна строка, без продающего тона.
    pub what: String,
    #[serde(default)]
    pub tech: Vec<String>,
}
