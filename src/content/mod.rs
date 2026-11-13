//! Контент страницы: разбор `content/*.toml`, проверка инвариантов и единое
//! представление данных для обычной и терминальной версий.
//!
//! Источник правды — файлы в `content/`. В бинарь они попадают через
//! `include_str!`, поэтому проверенный контент и собранный образ не могут
//! разойтись. Каталог читается только ради тестов и локальной правки.

pub mod date;
pub mod error;
pub mod model;
pub mod span;

use std::path::Path;
use std::sync::OnceLock;

pub use date::{End, YearMonth};
pub use error::{ContentError, ContentErrors};
pub use model::{Experience, Link, Profile, Project, Projects, Role, Stack, StackGroup};
pub use span::{Tenure, Timeline};

const PROFILE_FILE: &str = "profile.toml";
const STACK_FILE: &str = "stack.toml";
const EXPERIENCE_FILE: &str = "experience.toml";
const PROJECTS_FILE: &str = "projects.toml";

mod embedded {
    /// Тексты контента, вкомпилированные в бинарь.
    pub const PROFILE: &str = include_str!("../../content/profile.toml");
    pub const STACK: &str = include_str!("../../content/stack.toml");
    pub const EXPERIENCE: &str = include_str!("../../content/experience.toml");
    pub const PROJECTS: &str = include_str!("../../content/projects.toml");
}

/// Проверенный контент страницы.
///
/// Порядок мест работы и проектов — тот, в котором они перечислены в файлах:
/// это порядок вывода, а не сортировка на лету.
#[derive(Debug)]
pub struct Site {
    pub profile: Profile,
    pub stack: Vec<StackGroup>,
    pub roles: Vec<Role>,
    pub projects: Vec<Project>,
}

impl Site {
    /// Контент, вкомпилированный в бинарь. Разбирается и проверяется один раз
    /// за время жизни процесса.
    pub fn embedded() -> Result<&'static Site, ContentErrors> {
        static SITE: OnceLock<Result<Site, ContentErrors>> = OnceLock::new();
        SITE.get_or_init(|| Raw::from_embedded().and_then(Site::from_raw))
            .as_ref()
            .map_err(Clone::clone)
    }

    /// Читает и проверяет контент из каталога.
    pub fn load(dir: &Path) -> Result<Site, ContentErrors> {
        Site::from_raw(Raw::from_dir(dir)?)
    }

    /// Проверяет разобранные файлы и собирает [`Site`].
    fn from_raw(raw: Raw) -> Result<Site, ContentErrors> {
        let mut errors = Vec::new();
        validate_profile(&raw.profile, &mut errors);
        validate_stack(&raw.stack, &mut errors);
        validate_experience(&raw.experience, &mut errors);
        validate_projects(&raw.projects, &mut errors);

        if !errors.is_empty() {
            return Err(ContentErrors::new(errors));
        }

        Ok(Site {
            profile: raw.profile,
            stack: raw.stack.groups,
            roles: raw.experience.roles,
            projects: raw.projects.projects,
        })
    }
}

/// Разобранные файлы до проверки правил.
struct Raw {
    profile: Profile,
    stack: Stack,
    experience: Experience,
    projects: Projects,
}

impl Raw {
    fn from_embedded() -> Result<Self, ContentErrors> {
        Self::assemble(
            Raw::parse_text(PROFILE_FILE, embedded::PROFILE),
            Raw::parse_text(STACK_FILE, embedded::STACK),
            Raw::parse_text(EXPERIENCE_FILE, embedded::EXPERIENCE),
            Raw::parse_text(PROJECTS_FILE, embedded::PROJECTS),
        )
    }

    fn from_dir(dir: &Path) -> Result<Self, ContentErrors> {
        Self::assemble(
            Raw::parse_file(dir, PROFILE_FILE),
            Raw::parse_file(dir, STACK_FILE),
            Raw::parse_file(dir, EXPERIENCE_FILE),
            Raw::parse_file(dir, PROJECTS_FILE),
        )
    }

    /// Собирает файлы, накапливая ошибки: если битых файлов несколько, чинить
    /// их по одному за запуск неудобно.
    fn assemble(
        profile: Result<Profile, ContentError>,
        stack: Result<Stack, ContentError>,
        experience: Result<Experience, ContentError>,
        projects: Result<Projects, ContentError>,
    ) -> Result<Self, ContentErrors> {
        let mut errors = Vec::new();
        let profile = take(profile, &mut errors);
        let stack = take(stack, &mut errors);
        let experience = take(experience, &mut errors);
        let projects = take(projects, &mut errors);

        match (profile, stack, experience, projects) {
            (Some(profile), Some(stack), Some(experience), Some(projects)) => Ok(Self {
                profile,
                stack,
                experience,
                projects,
            }),
            _ => Err(ContentErrors::new(errors)),
        }
    }

    /// Разбирает текст файла.
    fn parse_text<T: serde::de::DeserializeOwned>(
        file: &'static str,
        text: &str,
    ) -> Result<T, ContentError> {
        toml::from_str(text).map_err(|error| ContentError::syntax(file, error.to_string()))
    }

    /// Читает файл из каталога и разбирает его.
    fn parse_file<T: serde::de::DeserializeOwned>(
        dir: &Path,
        file: &'static str,
    ) -> Result<T, ContentError> {
        let path = dir.join(file);
        let text = std::fs::read_to_string(&path).map_err(|error| {
            ContentError::syntax(
                file,
                format!("не удалось прочитать {}: {error}", path.display()),
            )
        })?;
        Raw::parse_text(file, &text)
    }
}

/// Достаёт значение из результата, складывая ошибки в общий список.
fn take<T>(result: Result<T, ContentError>, errors: &mut Vec<ContentError>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            errors.push(error);
            None
        }
    }
}

fn validate_profile(profile: &Profile, errors: &mut Vec<ContentError>) {
    require_text(errors, PROFILE_FILE, "name", &profile.name);
    require_text(errors, PROFILE_FILE, "role", &profile.role);
    require_text(errors, PROFILE_FILE, "summary", &profile.summary);
    require_text(errors, PROFILE_FILE, "location", &profile.location);

    if profile.about.is_empty() {
        errors.push(ContentError::new(
            PROFILE_FILE,
            "about",
            "нужен хотя бы один абзац",
        ));
    }
    for (index, paragraph) in profile.about.iter().enumerate() {
        require_text(errors, PROFILE_FILE, format!("about[{index}]"), paragraph);
    }

    if profile.contacts.is_empty() {
        errors.push(ContentError::new(
            PROFILE_FILE,
            "contact",
            "нужна хотя бы одна ссылка",
        ));
    }
    for (index, link) in profile.contacts.iter().enumerate() {
        require_link(errors, PROFILE_FILE, format!("contact[{index}]"), link);
    }
    if let Some(resume) = &profile.resume {
        require_link(errors, PROFILE_FILE, "resume", resume);
    }
}

fn validate_stack(stack: &Stack, errors: &mut Vec<ContentError>) {
    if stack.groups.is_empty() {
        errors.push(ContentError::new(
            STACK_FILE,
            "group",
            "нужна хотя бы одна группа",
        ));
    }

    let mut titles: Vec<&str> = Vec::new();
    for (index, group) in stack.groups.iter().enumerate() {
        let at = |field: &str| format!("groups[{index}].{field}");
        require_text(errors, STACK_FILE, at("title"), &group.title);

        if titles.contains(&group.title.as_str()) {
            errors.push(ContentError::new(
                STACK_FILE,
                at("title"),
                format!("группа {:?} уже объявлена выше", group.title),
            ));
        }
        titles.push(&group.title);

        if group.items.is_empty() {
            errors.push(ContentError::new(
                STACK_FILE,
                at("items"),
                "нужна хотя бы одна технология",
            ));
        }
        for (position, item) in group.items.iter().enumerate() {
            require_text(
                errors,
                STACK_FILE,
                format!("groups[{index}].items[{position}]"),
                item,
            );
        }
        for tech in &group.emphasis {
            if !group.items.contains(tech) {
                errors.push(ContentError::new(
                    STACK_FILE,
                    at("emphasis"),
                    format!("{tech:?} нет в items этой же группы"),
                ));
            }
        }
    }
}

fn validate_experience(experience: &Experience, errors: &mut Vec<ContentError>) {
    if experience.roles.is_empty() {
        errors.push(ContentError::new(
            EXPERIENCE_FILE,
            "role",
            "нужно хотя бы одно место работы",
        ));
    }

    for (index, role) in experience.roles.iter().enumerate() {
        let at = |field: &str| format!("roles[{index}].{field}");
        require_text(errors, EXPERIENCE_FILE, at("company"), &role.company);
        if let Some(short) = &role.short {
            require_text(errors, EXPERIENCE_FILE, at("short"), short);
        }
        require_text(errors, EXPERIENCE_FILE, at("title"), &role.title);
        require_text(errors, EXPERIENCE_FILE, at("place"), &role.place);

        if let Some(summary) = &role.summary {
            require_text(errors, EXPERIENCE_FILE, at("summary"), summary);
        }
        for (position, bullet) in role.bullets.iter().enumerate() {
            require_text(
                errors,
                EXPERIENCE_FILE,
                format!("roles[{index}].bullets[{position}]"),
                bullet,
            );
        }

        if let Some(end) = role.end.month()
            && end < role.start
        {
            errors.push(ContentError::new(
                EXPERIENCE_FILE,
                at("end"),
                format!("{end} раньше начала {}", role.start),
            ));
        }

        if role.end.is_now() && index > 0 {
            errors.push(ContentError::new(
                EXPERIENCE_FILE,
                at("end"),
                "текущее место работы должно быть первым в файле",
            ));
        }

        if index > 0 {
            let previous = &experience.roles[index - 1];
            if role.start > previous.start {
                errors.push(ContentError::new(
                    EXPERIENCE_FILE,
                    at("start"),
                    format!(
                        "{} позже начала предыдущего места работы {}",
                        role.start, previous.start
                    ),
                ));
            }
        }
    }
}

fn validate_projects(projects: &Projects, errors: &mut Vec<ContentError>) {
    if projects.projects.is_empty() {
        errors.push(ContentError::new(
            PROJECTS_FILE,
            "project",
            "нужен хотя бы один проект",
        ));
    }

    let mut names: Vec<&str> = Vec::new();
    for (index, project) in projects.projects.iter().enumerate() {
        let at = |field: &str| format!("projects[{index}].{field}");
        require_text(errors, PROJECTS_FILE, at("name"), &project.name);
        require_text(errors, PROJECTS_FILE, at("what"), &project.what);
        require_text(errors, PROJECTS_FILE, at("href"), &project.href);

        if !project.href.is_empty()
            && !(project.href.starts_with("https://") || project.href.starts_with("http://"))
        {
            errors.push(ContentError::new(
                PROJECTS_FILE,
                at("href"),
                format!(
                    "ссылка на проект должна быть абсолютной, получено {:?}",
                    project.href
                ),
            ));
        }

        if names.contains(&project.name.as_str()) {
            errors.push(ContentError::new(
                PROJECTS_FILE,
                at("name"),
                format!("проект {:?} уже объявлен выше", project.name),
            ));
        }
        names.push(&project.name);

        for (position, tech) in project.tech.iter().enumerate() {
            require_text(
                errors,
                PROJECTS_FILE,
                format!("projects[{index}].tech[{position}]"),
                tech,
            );
        }
    }
}

/// Требует непустое значение: пустая строка в контенте означает забытое поле,
/// а не намеренную пустоту.
fn require_text(
    errors: &mut Vec<ContentError>,
    file: &'static str,
    field: impl Into<String>,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(ContentError::new(file, field, "поле не может быть пустым"));
    }
}

/// Проверяет ссылку целиком: подпись, адрес и схему адреса.
fn require_link(
    errors: &mut Vec<ContentError>,
    file: &'static str,
    field: impl Into<String>,
    link: &Link,
) {
    let field = field.into();
    require_text(errors, file, format!("{field}.label"), &link.label);
    require_text(errors, file, format!("{field}.href"), &link.href);
    if !link.href.is_empty()
        && !["https://", "http://", "mailto:", "/"]
            .iter()
            .any(|prefix| link.href.starts_with(prefix))
    {
        errors.push(ContentError::new(
            file,
            format!("{field}.href"),
            format!(
                "адрес должен начинаться с https://, mailto: или /, получено {:?}",
                link.href
            ),
        ));
    }
}
