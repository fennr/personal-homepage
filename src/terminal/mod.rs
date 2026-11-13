//! Терминальный режим: разбор команд и текстовые ответы.
//!
//! Здесь нет ни HTTP, ни состояния между запросами: [`answer`] — чистая
//! функция от ввода и контента. Поэтому команды проверяются юнит-тестами, а не
//! запросами, и один и тот же ответ печатается и по `hx-post`, и без JS.

use crate::content::Site;

/// Команда, которую понимает терминал.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Help,
    About,
    Stack,
    Work,
    Projects,
    Contact,
    Resume,
    Ls,
    Cat(Section),
    Clear,
    Sudo,
    Exit,
}

/// Раздел контента: то, что печатает `cat` и называют подсказки.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    About,
    Stack,
    Work,
    Projects,
    Contact,
}

impl Section {
    /// Разбирает имя раздела; неизвестное имя — `None`.
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "about" => Some(Self::About),
            "stack" => Some(Self::Stack),
            "work" => Some(Self::Work),
            "projects" => Some(Self::Projects),
            "contact" => Some(Self::Contact),
            _ => None,
        }
    }
}

/// Разобранный ввод.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parsed {
    /// Пустая строка: нажали Enter, печатать нечего.
    Empty,
    Known(Command),
    /// Неизвестная команда или лишние аргументы.
    Unknown(String),
    /// `cat` с именем, которого нет среди разделов.
    UnknownSection(String),
}

/// Разбирает строку из поля ввода.
///
/// Имена команд приводятся к нижнему регистру: терминал здесь игрушечный, и
/// `Help` не должен упираться в ошибку. Раздел у `cat` — тоже.
pub fn parse(raw: &str) -> Parsed {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Parsed::Empty;
    }

    let mut words = trimmed.split_whitespace();
    let name = words.next().unwrap_or_default().to_lowercase();
    let argument = words.next().map(str::to_lowercase);

    if words.next().is_some() {
        return Parsed::Unknown(trimmed.to_owned());
    }

    let command = match (name.as_str(), argument.as_deref()) {
        ("help", None) => Command::Help,
        ("about", None) => Command::About,
        ("stack", None) => Command::Stack,
        ("work", None) => Command::Work,
        ("projects", None) => Command::Projects,
        ("contact", None) => Command::Contact,
        ("resume", None) => Command::Resume,
        ("ls", None) => Command::Ls,
        ("clear", None) => Command::Clear,
        ("sudo", None) => Command::Sudo,
        ("exit", None) => Command::Exit,
        ("cat", Some(slug)) => match Section::parse(slug) {
            Some(section) => Command::Cat(section),
            None => return Parsed::UnknownSection(slug.to_owned()),
        },
        _ => return Parsed::Unknown(trimmed.to_owned()),
    };
    Parsed::Known(command)
}

/// Строка вывода.
///
/// `class` — имя CSS-класса, а не перечисление: шаблон печатает его как есть,
/// и новый вид строки не требует правки шаблона.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub class: &'static str,
    /// Строка ввода: перед ней печатается приглашение.
    pub prompt: bool,
    pub text: String,
}

impl Line {
    fn output(text: impl Into<String>) -> Self {
        Self {
            class: "out",
            prompt: false,
            text: text.into(),
        }
    }

    fn muted(text: impl Into<String>) -> Self {
        Self {
            class: "muted",
            ..Self::output(text)
        }
    }

    fn error(text: impl Into<String>) -> Self {
        Self {
            class: "error",
            ..Self::output(text)
        }
    }

    fn echo(text: impl Into<String>) -> Self {
        Self {
            class: "echo",
            prompt: true,
            text: text.into(),
        }
    }

    /// Пункт списка внутри ответа: с висячим отступом, чтобы перенос
    /// продолжения не начинался с нулевой колонки.
    fn item(text: impl Into<String>) -> Self {
        Self {
            class: "item muted",
            ..Self::output(text)
        }
    }

    /// Пустая строка: отбивка между блоками вывода.
    fn blank() -> Self {
        Self::muted("")
    }
}

/// Ответ терминала.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Answer {
    pub lines: Vec<Line>,
    pub hints: Vec<&'static str>,
    /// `clear`: вывод нужно очистить, а не дописать.
    pub clear: bool,
}

/// Ответ на введённую строку.
pub fn answer(raw: &str, site: &Site) -> Answer {
    let mut answer = Answer::default();

    match parse(raw) {
        Parsed::Empty => return answer,
        Parsed::Known(Command::Clear) => {
            answer.clear = true;
            return answer;
        }
        Parsed::Unknown(name) => {
            answer.lines.push(Line::echo(raw.trim()));
            answer
                .lines
                .push(Line::error(format!("{name}: команда не найдена")));
            answer.lines.push(Line::muted("help — список команд"));
            answer.hints = vec!["help"];
        }
        Parsed::UnknownSection(name) => {
            answer.lines.push(Line::echo(raw.trim()));
            answer
                .lines
                .push(Line::error(format!("cat: нет такого раздела: {name}")));
            answer.lines.push(Line::muted("ls — список разделов"));
            answer.hints = vec!["ls"];
        }
        Parsed::Known(command) => {
            answer.lines.push(Line::echo(raw.trim()));
            let (lines, hints) = print(command, site);
            answer.lines.extend(lines);
            answer.hints = hints;
        }
    }

    answer
}

/// Что печатает терминал при открытии страницы: без строки ввода.
pub fn welcome(site: &Site) -> Answer {
    let mut answer = Answer {
        lines: vec![
            Line::output("Терминальная версия fennr.ru."),
            Line::muted("Введите команду или нажмите подсказку ниже."),
            Line::blank(),
        ],
        ..Answer::default()
    };
    let (lines, hints) = print(Command::Help, site);
    answer.lines.extend(lines);
    answer.hints = hints;
    answer
}

/// Вывод команды и подсказки следующих команд.
fn print(command: Command, site: &Site) -> (Vec<Line>, Vec<&'static str>) {
    match command {
        Command::Help => (help(), vec!["about", "work", "projects"]),
        Command::About => (about(site), vec!["stack", "work"]),
        Command::Stack | Command::Cat(Section::Stack) => (stack(site), vec!["work", "projects"]),
        Command::Work | Command::Cat(Section::Work) => (work(site), vec!["projects", "contact"]),
        Command::Projects | Command::Cat(Section::Projects) => {
            (projects(site), vec!["contact", "help"])
        }
        Command::Contact | Command::Cat(Section::Contact) => {
            (contact(site), vec!["resume", "work"])
        }
        Command::Cat(Section::About) => (about(site), vec!["stack", "work"]),
        Command::Resume => (resume(site), vec!["contact"]),
        Command::Ls => (sections(), vec!["about", "work"]),
        Command::Sudo => (
            vec![Line::output(
                "visitor is not in the sudoers file. This incident has been reported.",
            )],
            vec!["help"],
        ),
        Command::Exit => (
            vec![
                Line::output("Выхода нет. Обычная версия — на этом же домене, по адресу /."),
                Line::muted("/ — вернуться к документу"),
            ],
            vec!["help"],
        ),
        Command::Clear => (Vec::new(), Vec::new()),
    }
}

fn help() -> Vec<Line> {
    let mut lines = vec![Line::output("Доступные команды:")];
    for (name, description) in [
        ("help", "эта справка"),
        ("about", "о себе"),
        ("stack", "стек по группам"),
        ("work", "места работы"),
        ("projects", "проекты"),
        ("contact", "контакты"),
        ("resume", "резюме в PDF"),
        ("ls", "разделы контента"),
        ("cat <раздел>", "содержимое раздела"),
        ("clear", "очистить вывод"),
    ] {
        lines.push(Line::muted(format!("  {name:<15}{description}")));
    }
    lines
}

fn about(site: &Site) -> Vec<Line> {
    let mut lines = vec![Line::output(site.profile.summary.clone()), Line::blank()];
    for (index, paragraph) in site.profile.about.iter().enumerate() {
        if index > 0 {
            lines.push(Line::blank());
        }
        lines.push(Line::output(paragraph.clone()));
    }
    lines
}

fn stack(site: &Site) -> Vec<Line> {
    site.stack
        .iter()
        .map(|group| Line::output(format!("{}: {}", group.title, group.items.join(", "))))
        .collect()
}

fn work(site: &Site) -> Vec<Line> {
    let mut lines = Vec::new();
    for (index, role) in site.roles.iter().enumerate() {
        if index > 0 {
            lines.push(Line::blank());
        }
        let end = role
            .end
            .month()
            .map_or_else(|| "сейчас".to_owned(), |month| month.to_string());
        lines.push(Line::output(format!(
            "{} — {end}  {}, {}",
            role.start, role.title, role.company
        )));
        if let Some(summary) = &role.summary {
            lines.push(Line::item(summary.clone()));
        }
        lines.extend(
            role.bullets
                .iter()
                .map(|bullet| Line::item(format!("— {bullet}"))),
        );
    }
    lines
}

fn projects(site: &Site) -> Vec<Line> {
    site.projects
        .iter()
        .flat_map(|project| {
            let tech = if project.tech.is_empty() {
                String::new()
            } else {
                format!(" · {}", project.tech.join(", "))
            };
            [
                Line::output(format!("{} — {}", project.name, project.what)),
                Line::muted(format!("  {}{tech}", project.href)),
            ]
        })
        .collect()
}

fn contact(site: &Site) -> Vec<Line> {
    site.profile
        .contacts
        .iter()
        .map(|link| {
            // Адрес внутри сервиса печатается первым: его диктуют голосом.
            match &link.handle {
                Some(handle) => Line::output(format!("{}: {handle} · {}", link.label, link.href)),
                None => Line::output(format!("{}: {}", link.label, link.href)),
            }
        })
        .collect()
}

fn resume(site: &Site) -> Vec<Line> {
    match &site.profile.resume {
        Some(resume) => vec![Line::output(format!("{}: {}", resume.label, resume.href))],
        None => vec![Line::muted("резюме пока не собрано")],
    }
}

fn sections() -> Vec<Line> {
    vec![Line::output("about  stack  work  projects  contact")]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site() -> &'static Site {
        Site::embedded().expect("контент валиден")
    }

    fn text(answer: &Answer) -> Vec<&str> {
        answer.lines.iter().map(|line| line.text.as_str()).collect()
    }

    #[test]
    fn parses_every_known_command() {
        assert_eq!(parse("help"), Parsed::Known(Command::Help));
        assert_eq!(parse("  WORK  "), Parsed::Known(Command::Work));
        assert_eq!(
            parse("cat about"),
            Parsed::Known(Command::Cat(Section::About))
        );
        assert_eq!(
            parse("cat STACK"),
            Parsed::Known(Command::Cat(Section::Stack))
        );
        assert_eq!(parse("clear"), Parsed::Known(Command::Clear));
        assert_eq!(parse(""), Parsed::Empty);
        assert_eq!(parse("   "), Parsed::Empty);
    }

    #[test]
    fn rejects_unknown_commands_and_sections() {
        assert_eq!(parse("nope"), Parsed::Unknown("nope".to_owned()));
        assert_eq!(parse("cat csv"), Parsed::UnknownSection("csv".to_owned()));
        assert_eq!(
            parse("cd home"),
            Parsed::Unknown("cd home".to_owned()),
            "лишний аргумент — не команда"
        );
    }

    #[test]
    fn every_command_answers_with_something() {
        for command in [
            Command::Help,
            Command::About,
            Command::Stack,
            Command::Work,
            Command::Projects,
            Command::Contact,
            Command::Resume,
            Command::Ls,
            Command::Sudo,
            Command::Exit,
        ] {
            let (lines, hints) = print(command, site());
            assert!(!lines.is_empty(), "{command:?} печатает пустоту");
            assert!(!hints.is_empty(), "{command:?} без подсказок");
        }
    }

    #[test]
    fn work_prints_every_role_with_dates() {
        let answer = answer("work", site());
        let lines = text(&answer);
        assert_eq!(lines[0], "work");
        assert!(answer.lines[0].prompt, "ввод печатается с приглашением");
        assert!(lines.iter().any(|line| line.contains("Уралхим")));
        assert!(lines.iter().any(|line| line.contains("2024-05 — сейчас")));
        assert!(
            lines.iter().any(|line| line.contains("2018-09 — 2019-12")),
            "закрытый период печатается месяцем"
        );
        assert_eq!(
            answer
                .lines
                .iter()
                .filter(|line| line.text.is_empty())
                .count(),
            4,
            "отбивка между пятью местами работы"
        );
        assert_eq!(
            answer
                .lines
                .iter()
                .filter(|line| line.class == "item muted")
                .count(),
            8,
            "шесть пунктов и две строки summary"
        );
    }

    #[test]
    fn contact_shows_the_handle_before_the_url() {
        let answer = answer("contact", site());
        let lines = text(&answer);
        assert_eq!(lines[0], "contact");
        assert_eq!(
            lines[1], "Telegram: @fenn_r · https://t.me/fenn_r",
            "основной контакт стоит первым и читается без перехода по ссылке"
        );
    }

    #[test]
    fn unknown_command_explains_itself() {
        let answer = answer("nope", site());
        assert_eq!(answer.lines[1].class, "error");
        assert!(answer.lines[1].text.contains("команда не найдена"));
        assert_eq!(answer.hints, vec!["help"]);
    }

    #[test]
    fn clear_asks_to_wipe_the_output() {
        let answer = answer("clear", site());
        assert!(answer.clear);
        assert!(answer.lines.is_empty());
        assert!(answer.hints.is_empty());
    }

    #[test]
    fn empty_input_prints_nothing() {
        let answer = answer("   ", site());
        assert_eq!(answer, Answer::default());
    }

    #[test]
    fn cat_prints_the_same_as_the_section_command() {
        let (cat_lines, cat_hints) = print(Command::Cat(Section::Work), site());
        let (work_lines, work_hints) = print(Command::Work, site());
        assert_eq!(cat_lines, work_lines);
        assert_eq!(cat_hints, work_hints);
    }

    #[test]
    fn welcome_starts_with_help_but_without_a_command_line() {
        let answer = welcome(site());
        assert!(!answer.lines[0].prompt, "приглашения в приветствии нет");
        assert!(
            answer
                .lines
                .iter()
                .any(|line| line.text == "Доступные команды:")
        );
        assert!(!answer.clear, "приветствие ничего не очищает");
    }

    #[test]
    fn tech_and_links_are_printed_for_projects() {
        let answer = answer("projects", site());
        let lines = text(&answer);
        assert!(lines.iter().any(|line| line.contains("periodparser")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("https://pypi.org/project/periodparser/"))
        );
        assert!(lines.iter().any(|line| line.contains("Python, PyPI")));
    }
}
