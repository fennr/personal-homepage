//! Арифметика карьеры: месяцы между датами, стаж и полоса «когда что было».
//!
//! Месяцы считаются включительно: месяц, в который человек работал хотя бы
//! часть времени, считается целым. Поэтому карьера с сентября 2018 по ноябрь
//! 2026 — это 99 месяцев, а не 98: ноябрь уже идёт. По тому же правилу
//! считаются отрезки полосы, иначе она теряла бы месяц на каждой границе.

use super::date::YearMonth;
use super::model::Role;

/// Месяцы от `start` до `end` включительно.
///
/// Для `end` раньше `start` результат отрицательный: такой контент отсекается
/// проверкой правил, поэтому вызывающий код просто не увидит его в `Site`.
pub fn months_inclusive(start: YearMonth, end: YearMonth) -> i32 {
    end.ordinal() - start.ordinal() + 1
}

/// Стаж: полные годы и месяцы сверх них.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tenure {
    months: i32,
}

impl Tenure {
    /// Стаж от `start` до `today`; отрицательная разница считается нулём.
    pub fn between(start: YearMonth, today: YearMonth) -> Self {
        Self {
            months: months_inclusive(start, today).max(0),
        }
    }

    /// Всего месяцев.
    pub fn months(self) -> i32 {
        self.months
    }

    /// Полные годы.
    pub fn years(self) -> i32 {
        self.months / 12
    }

    /// Месяцы сверх полных лет.
    pub fn rest_months(self) -> i32 {
        self.months % 12
    }
}

/// Отрезок полосы карьеры: одна компания и её длина в месяцах.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// Имя на полосе: то же, что `brand` места работы.
    pub label: String,
    pub months: i32,
    pub current: bool,
    /// Отрезок уже пятой части полосы: на узком экране его подпись не
    /// помещается, поэтому показывается только цвет.
    pub narrow: bool,
}

/// Год на оси под полосой: где ставить подпись.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YearTick {
    pub year: u16,
    /// Положение в процентах, уже отформатированное для CSS.
    pub percent: String,
}

/// Полоса карьеры: отрезки и ось лет.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timeline {
    pub segments: Vec<Segment>,
    pub ticks: Vec<YearTick>,
}

impl Timeline {
    /// Строит полосу из мест работы, перечисленных от нового к давнему.
    ///
    /// Отрезки разворачиваются в хронологический порядок: слева самое давнее
    /// место, справа текущее, иначе полоса читается против оси времени.
    /// Незакрытый период (`end = "now"`) считается до `today`.
    pub fn build(roles: &[Role], today: YearMonth) -> Self {
        let segments: Vec<Segment> = roles
            .iter()
            .rev()
            .map(|role| Segment {
                label: role.brand.clone(),
                months: months_inclusive(role.start, role.end.month().unwrap_or(today)).max(0),
                current: role.end.is_now(),
                narrow: false,
            })
            .collect();

        let total: i32 = segments.iter().map(|segment| segment.months).sum();
        let segments = segments
            .into_iter()
            .map(|segment| Segment {
                narrow: segment.months * 5 < total,
                ..segment
            })
            .collect::<Vec<_>>();
        Self {
            ticks: ticks(roles, today, total),
            segments,
        }
    }
}

/// Ось лет: первый год карьеры пропускается, потому что полоса начинается в
/// его середине, а последний может выйти за правый край.
fn ticks(roles: &[Role], today: YearMonth, total: i32) -> Vec<YearTick> {
    let Some(oldest) = roles.last() else {
        return Vec::new();
    };
    if total <= 0 {
        return Vec::new();
    }

    let start = oldest.start;
    (start.year() + 1..=today.year())
        .filter_map(|year| YearMonth::new(year, 1))
        .filter_map(|january| {
            let months = months_inclusive(start, january) - 1;
            let percent = f64::from(months) * 100.0 / f64::from(total);
            (0.0 < percent && percent < 100.0).then(|| YearTick {
                year: january.year(),
                percent: format!("{percent:.2}"),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Site;

    fn month(raw: &str) -> YearMonth {
        YearMonth::parse(raw).expect("валидный месяц")
    }

    #[test]
    fn a_single_month_counts_as_one() {
        assert_eq!(
            months_inclusive(month("2024-05"), month("2024-05")),
            1,
            "место работы внутри одного месяца"
        );
        assert_eq!(months_inclusive(month("2023-05"), month("2024-05")), 13);
    }

    #[test]
    fn tenure_splits_into_years_and_months() {
        let tenure = Tenure::between(month("2018-09"), month("2026-11"));
        assert_eq!(tenure.years(), 8);
        assert_eq!(tenure.rest_months(), 3);
        assert_eq!(tenure.months(), 99);
    }

    #[test]
    fn tenure_never_goes_negative() {
        let tenure = Tenure::between(month("2026-11"), month("2020-01"));
        assert_eq!(tenure.months(), 0);
    }

    #[test]
    fn timeline_covers_every_role() {
        let site = Site::embedded().expect("контент валиден");
        let timeline = Timeline::build(&site.roles, month("2026-11"));
        assert_eq!(timeline.segments.len(), site.roles.len());
        assert!(
            timeline.segments.last().is_some_and(|last| last.current),
            "полоса идёт по времени: текущее место работы — последнее"
        );
        assert!(
            timeline
                .segments
                .first()
                .is_some_and(|first| !first.current),
            "слева самое давнее место работы"
        );
        assert_eq!(
            timeline
                .segments
                .iter()
                .map(|segment| segment.label.as_str())
                .collect::<Vec<_>>(),
            ["BC Soft", "Saprun", "Promobot", "Ujin", "Uralchem"],
            "слева давнее; имена на полосе — латиницей"
        );
        assert_eq!(
            timeline
                .segments
                .iter()
                .map(|segment| segment.months)
                .sum::<i32>(),
            100,
            "сумма отрезков: 16 + 34 + 8 + 11 + 31 месяц"
        );
    }

    #[test]
    fn timeline_axis_stays_inside_the_bar() {
        let site = Site::embedded().expect("контент валиден");
        let timeline = Timeline::build(&site.roles, month("2026-11"));
        assert_eq!(timeline.ticks.first().map(|tick| tick.year), Some(2019));
        assert_eq!(timeline.ticks.last().map(|tick| tick.year), Some(2026));
        for tick in &timeline.ticks {
            let percent: f64 = tick.percent.parse().expect("процент числом");
            assert!((0.0..100.0).contains(&percent), "{tick:?}");
        }
    }
}
