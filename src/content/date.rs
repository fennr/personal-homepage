//! Месяцы карьеры: тип [`YearMonth`] и конец периода [`End`].

use std::fmt;

use serde::{Deserialize, Deserializer};

/// Месяц без дня, в котором измеряется карьера: `2024-05`.
///
/// Порядок сравнения задан календарём, поэтому «конец не раньше начала» и
/// сортировка мест работы проверяются на этом типе, а не на строках.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct YearMonth {
    year: u16,
    month: u8,
}

impl YearMonth {
    /// Разбирает `YYYY-MM`.
    ///
    /// Текст ошибки не содержит имени поля: его добавляет вызывающая сторона,
    /// которая знает путь к файлу и месту в нём.
    pub fn parse(raw: &str) -> Result<Self, String> {
        let invalid = || format!("ожидается YYYY-MM, получено {raw:?}");
        let (year, month) = raw.split_once('-').ok_or_else(invalid)?;
        if year.len() != 4 || month.len() != 2 {
            return Err(invalid());
        }
        let year: u16 = year.parse().map_err(|_| invalid())?;
        let month: u8 = month.parse().map_err(|_| invalid())?;
        if year == 0 || !(1..=12).contains(&month) {
            return Err(invalid());
        }
        Ok(Self { year, month })
    }

    /// Год месяца.
    pub fn year(self) -> u16 {
        self.year
    }

    /// Номер месяца, 1–12.
    pub fn month(self) -> u8 {
        self.month
    }

    /// Месяц по году и номеру; `None`, если номер вне 1–12.
    pub fn new(year: u16, month: u8) -> Option<Self> {
        if year == 0 || !(1..=12).contains(&month) {
            return None;
        }
        Some(Self { year, month })
    }

    /// Порядковый номер месяца от начала летоисчисления: по нему считается
    /// разница между датами.
    pub fn ordinal(self) -> i32 {
        i32::from(self.year) * 12 + i32::from(self.month)
    }
}

impl fmt::Display for YearMonth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:04}-{:02}", self.year, self.month)
    }
}

impl<'de> Deserialize<'de> for YearMonth {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        YearMonth::parse(&raw).map_err(serde::de::Error::custom)
    }
}

/// Конец периода работы: конкретный месяц или «по настоящее время».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    Month(YearMonth),
    Now,
}

impl End {
    /// Месяц, которым период закрыт; `None` — период продолжается.
    pub fn month(self) -> Option<YearMonth> {
        match self {
            Self::Month(month) => Some(month),
            Self::Now => None,
        }
    }

    /// Продолжается ли период прямо сейчас.
    pub fn is_now(self) -> bool {
        matches!(self, Self::Now)
    }
}

impl fmt::Display for End {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Month(month) => write!(formatter, "{month}"),
            Self::Now => write!(formatter, "now"),
        }
    }
}

impl<'de> Deserialize<'de> for End {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;

        let raw = String::deserialize(deserializer)?;
        if raw == "now" {
            return Ok(Self::Now);
        }
        YearMonth::parse(&raw).map(Self::Month).map_err(|error| {
            D::Error::custom(format!("{error}; для текущего места работы — \"now\""))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_month_and_orders_by_calendar() {
        let january = YearMonth::parse("2024-01").expect("валидный месяц");
        let december = YearMonth::parse("2023-12").expect("валидный месяц");
        assert!(december < january);
        assert_eq!(january.to_string(), "2024-01");
    }

    #[test]
    fn rejects_malformed_months() {
        for raw in [
            "2024-13",
            "2024-0",
            "24-05",
            "2024-5",
            "2024",
            "2024-05-01",
            "",
        ] {
            assert!(
                YearMonth::parse(raw).is_err(),
                "{raw:?} должен быть отвергнут"
            );
        }
    }

    #[test]
    fn end_accepts_now_and_month_values() {
        assert!(matches!(toml_end("\"now\""), End::Now));
        assert_eq!(
            toml_end("\"2024-05\"").month(),
            YearMonth::parse("2024-05").ok()
        );
    }

    #[test]
    fn end_reports_how_to_mark_the_current_job() {
        let error = toml_error("\"май\"");
        assert!(error.contains("YYYY-MM"), "{error}");
        assert!(error.contains("now"), "{error}");
    }

    /// Разбирает значение конца периода через TOML — так же, как это делает
    /// загрузчик контента.
    fn toml_end(raw: &str) -> End {
        toml::from_str::<Wrapper>(&format!("end = {raw}"))
            .expect("значение должно разбираться")
            .end
    }

    fn toml_error(raw: &str) -> String {
        toml::from_str::<Wrapper>(&format!("end = {raw}"))
            .expect_err("значение должно быть отвергнуто")
            .to_string()
    }

    #[derive(Debug, serde::Deserialize)]
    struct Wrapper {
        end: End,
    }
}
