//! Ошибки контента: путь `файл: поле` и объяснение на человеческом языке.
//!
//! Контент правится руками и часто, поэтому сообщение об ошибке — интерфейс:
//! по нему должно быть понятно, какой файл открыть и что там заменить.

use std::fmt;

/// Одно нарушение в файле контента.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentError {
    file: &'static str,
    field: String,
    problem: String,
}

impl ContentError {
    /// Нарушение правила в конкретном поле.
    pub fn new(file: &'static str, field: impl Into<String>, problem: impl Into<String>) -> Self {
        Self {
            file,
            field: field.into(),
            problem: problem.into(),
        }
    }

    /// Ошибка разбора TOML: позицию в тексте уже сообщает сам `toml`,
    /// поэтому поле остаётся пустым.
    pub fn syntax(file: &'static str, problem: impl Into<String>) -> Self {
        Self {
            file,
            field: String::new(),
            problem: problem.into(),
        }
    }

    /// Файл, в котором нашлось нарушение.
    pub fn file(&self) -> &'static str {
        self.file
    }

    /// Путь к полю внутри файла; пусто для синтаксических ошибок.
    pub fn field(&self) -> &str {
        &self.field
    }

    /// Что именно не так.
    pub fn problem(&self) -> &str {
        &self.problem
    }
}

impl fmt::Display for ContentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.field.is_empty() {
            write!(formatter, "{}: {}", self.file, self.problem)
        } else {
            write!(formatter, "{}: {}: {}", self.file, self.field, self.problem)
        }
    }
}

/// Все найденные нарушения: контент проверяется целиком, чтобы не чинить
/// файлы по одному за запуск.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentErrors(Vec<ContentError>);

impl ContentErrors {
    /// Собирает список нарушений.
    pub fn new(errors: Vec<ContentError>) -> Self {
        Self(errors)
    }

    /// Нарушения по порядку проверки.
    pub fn as_slice(&self) -> &[ContentError] {
        &self.0
    }
}

impl fmt::Display for ContentErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, error) in self.0.iter().enumerate() {
            if index > 0 {
                formatter.write_str("\n")?;
            }
            write!(formatter, "{error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ContentErrors {}

impl IntoIterator for ContentErrors {
    type Item = ContentError;
    type IntoIter = std::vec::IntoIter<ContentError>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
