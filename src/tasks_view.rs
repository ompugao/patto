//! Task grouping shared by every client.
//!
//! Pending tasks are bucketed by deadline and completed tasks by recency; the
//! review timeframes mirror the `experimental/tasks_review` LSP command. All
//! functions take `today` explicitly so callers (and tests) control the clock.

use chrono::{Datelike, NaiveDate};

use crate::parser::Deadline;

/// Deadline bucket for a pending task, relative to `today`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PendingGroup {
    Overdue,
    Today,
    Tomorrow,
    ThisWeek,
    ThisMonth,
    Later,
    /// The `due=` value could not be parsed as a date.
    Uninterpretable,
}

impl PendingGroup {
    pub fn label(&self) -> &'static str {
        match self {
            PendingGroup::Overdue => "Overdue",
            PendingGroup::Today => "Today",
            PendingGroup::Tomorrow => "Tomorrow",
            PendingGroup::ThisWeek => "This Week",
            PendingGroup::ThisMonth => "This Month",
            PendingGroup::Later => "Later",
            PendingGroup::Uninterpretable => "Uninterpretable Deadline",
        }
    }
}

/// Last day of the month containing `day`.
fn month_end(day: NaiveDate) -> Option<NaiveDate> {
    let first_of_next = if day.month() == 12 {
        NaiveDate::from_ymd_opt(day.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(day.year(), day.month() + 1, 1)
    }?;
    first_of_next.pred_opt()
}

/// Monday of the week containing `day`.
pub fn week_start(day: NaiveDate) -> NaiveDate {
    day - chrono::Duration::days(day.weekday().num_days_from_monday() as i64)
}

/// Classify a pending task's deadline relative to `today`.
pub fn pending_group(due: &Deadline, today: NaiveDate) -> PendingGroup {
    match due {
        Deadline::DateTime(dt) => pending_group(&Deadline::Date(dt.date()), today),
        Deadline::Uninterpretable(_) => PendingGroup::Uninterpretable,
        Deadline::Date(d) => {
            let diff = (*d - today).num_days();
            if diff < 0 {
                return PendingGroup::Overdue;
            }
            if diff == 0 {
                return PendingGroup::Today;
            }
            if diff == 1 {
                return PendingGroup::Tomorrow;
            }
            // Days remaining until the coming Saturday.
            let days_until_sat = (5 - today.weekday().num_days_from_monday() as i64).rem_euclid(7);
            if diff <= days_until_sat {
                return PendingGroup::ThisWeek;
            }
            match month_end(today) {
                Some(end) if *d <= end => PendingGroup::ThisMonth,
                _ => PendingGroup::Later,
            }
        }
    }
}

/// Recency bucket for a completed task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompletedGroup {
    Today,
    Yesterday,
    ThisWeek,
    LastWeek,
    ThisMonth,
    Older,
}

impl CompletedGroup {
    pub fn label(&self) -> &'static str {
        match self {
            CompletedGroup::Today => "Today",
            CompletedGroup::Yesterday => "Yesterday",
            CompletedGroup::ThisWeek => "This Week",
            CompletedGroup::LastWeek => "Last Week",
            CompletedGroup::ThisMonth => "This Month",
            CompletedGroup::Older => "Older",
        }
    }
}

/// Classify a `completed_at` date into a recency bucket relative to `today`.
pub fn completed_group(date: NaiveDate, today: NaiveDate) -> CompletedGroup {
    let diff = (today - date).num_days();
    if diff == 0 {
        return CompletedGroup::Today;
    }
    if diff == 1 {
        return CompletedGroup::Yesterday;
    }

    let this_week_start = week_start(today);
    let last_week_start = this_week_start - chrono::Duration::days(7);
    let this_month_start = today.with_day(1).unwrap_or(today);

    if date >= this_week_start {
        CompletedGroup::ThisWeek
    } else if date >= last_week_start {
        CompletedGroup::LastWeek
    } else if date >= this_month_start {
        CompletedGroup::ThisMonth
    } else {
        CompletedGroup::Older
    }
}

/// Time window presets for reviewing completed tasks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewTimeframe {
    Today,
    Yesterday,
    ThisWeek,
    LastWeek,
    ThisMonth,
    Custom {
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    },
}

impl ReviewTimeframe {
    /// Parse the timeframe name accepted by `experimental/tasks_review`.
    /// Unknown names fall back to `Today`, matching the LSP command.
    pub fn from_name(name: &str, from: Option<NaiveDate>, to: Option<NaiveDate>) -> Self {
        match name {
            "yesterday" => ReviewTimeframe::Yesterday,
            "this_week" => ReviewTimeframe::ThisWeek,
            "last_week" => ReviewTimeframe::LastWeek,
            "this_month" => ReviewTimeframe::ThisMonth,
            "custom" => ReviewTimeframe::Custom { from, to },
            _ => ReviewTimeframe::Today,
        }
    }
}

/// Inclusive `(from, to)` bounds for a timeframe. `None` means unbounded.
pub fn timeframe_bounds(
    timeframe: &ReviewTimeframe,
    today: NaiveDate,
) -> (Option<NaiveDate>, Option<NaiveDate>) {
    match timeframe {
        ReviewTimeframe::Today => (Some(today), Some(today)),
        ReviewTimeframe::Yesterday => {
            let yesterday = today - chrono::Duration::days(1);
            (Some(yesterday), Some(yesterday))
        }
        ReviewTimeframe::ThisWeek => (Some(week_start(today)), Some(today)),
        ReviewTimeframe::LastWeek => {
            let this_week_start = week_start(today);
            (
                Some(this_week_start - chrono::Duration::days(7)),
                Some(this_week_start - chrono::Duration::days(1)),
            )
        }
        ReviewTimeframe::ThisMonth => (Some(today.with_day(1).unwrap_or(today)), Some(today)),
        ReviewTimeframe::Custom { from, to } => (*from, *to),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn pending_groups_relative_to_a_wednesday() {
        // 2026-09-16 is a Wednesday; the coming Saturday is 3 days away.
        let today = date("2026-09-16");
        let g = |d: &str| pending_group(&Deadline::Date(date(d)), today);

        assert_eq!(g("2026-09-15"), PendingGroup::Overdue);
        assert_eq!(g("2026-09-16"), PendingGroup::Today);
        assert_eq!(g("2026-09-17"), PendingGroup::Tomorrow);
        assert_eq!(g("2026-09-19"), PendingGroup::ThisWeek);
        assert_eq!(g("2026-09-20"), PendingGroup::ThisMonth);
        assert_eq!(g("2026-09-30"), PendingGroup::ThisMonth);
        assert_eq!(g("2026-10-01"), PendingGroup::Later);
    }

    #[test]
    fn pending_group_on_saturday_has_no_this_week_bucket() {
        // On a Saturday, days_until_sat == 0, so anything past tomorrow moves on.
        let today = date("2026-09-19");
        assert_eq!(
            pending_group(&Deadline::Date(date("2026-09-21")), today),
            PendingGroup::ThisMonth
        );
    }

    #[test]
    fn uninterpretable_due_is_its_own_group() {
        let today = date("2026-09-16");
        assert_eq!(
            pending_group(&Deadline::Uninterpretable(String::new()), today),
            PendingGroup::Uninterpretable
        );
    }

    #[test]
    fn completed_groups_relative_to_a_wednesday() {
        let today = date("2026-09-16");
        let g = |d: &str| completed_group(date(d), today);

        assert_eq!(g("2026-09-16"), CompletedGroup::Today);
        assert_eq!(g("2026-09-15"), CompletedGroup::Yesterday);
        assert_eq!(g("2026-09-14"), CompletedGroup::ThisWeek); // Monday
        assert_eq!(g("2026-09-13"), CompletedGroup::LastWeek); // previous Sunday
        assert_eq!(g("2026-09-07"), CompletedGroup::LastWeek);
        assert_eq!(g("2026-09-06"), CompletedGroup::ThisMonth);
        assert_eq!(g("2026-08-31"), CompletedGroup::Older);
    }

    #[test]
    fn timeframe_bounds_match_the_lsp_command() {
        let today = date("2026-09-16");
        assert_eq!(
            timeframe_bounds(&ReviewTimeframe::Today, today),
            (Some(today), Some(today))
        );
        assert_eq!(
            timeframe_bounds(&ReviewTimeframe::Yesterday, today),
            (Some(date("2026-09-15")), Some(date("2026-09-15")))
        );
        assert_eq!(
            timeframe_bounds(&ReviewTimeframe::ThisWeek, today),
            (Some(date("2026-09-14")), Some(today))
        );
        assert_eq!(
            timeframe_bounds(&ReviewTimeframe::LastWeek, today),
            (Some(date("2026-09-07")), Some(date("2026-09-13")))
        );
        assert_eq!(
            timeframe_bounds(&ReviewTimeframe::ThisMonth, today),
            (Some(date("2026-09-01")), Some(today))
        );
        assert_eq!(
            timeframe_bounds(
                &ReviewTimeframe::Custom {
                    from: Some(date("2026-01-01")),
                    to: None
                },
                today
            ),
            (Some(date("2026-01-01")), None)
        );
    }

    #[test]
    fn unknown_timeframe_name_falls_back_to_today() {
        assert_eq!(
            ReviewTimeframe::from_name("nonsense", None, None),
            ReviewTimeframe::Today
        );
    }
}
