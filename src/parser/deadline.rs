use std::cmp::Ordering;
use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Clone, Deserialize, Serialize)]
pub enum Deadline {
    DateTime(chrono::NaiveDateTime),
    Date(chrono::NaiveDate),
    Uninterpretable(String),
}

impl fmt::Display for Deadline {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Deadline::DateTime(dt) => {
                write!(f, "{}", dt.format("%Y-%m-%dT%H:%M"))?;
            }
            Deadline::Date(d) => {
                write!(f, "{}", d)?;
            }
            Deadline::Uninterpretable(s) => {
                write!(f, "{}", s)?;
            }
        }
        Ok(())
    }
}

impl PartialOrd for Deadline {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Deadline {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Deadline::Date(d1), Deadline::Date(d2)) => d1.cmp(d2),
            (Deadline::DateTime(dt1), Deadline::DateTime(dt2)) => dt1.cmp(dt2),
            (Deadline::Uninterpretable(t1), Deadline::Uninterpretable(t2)) => t1.cmp(t2),
            (Deadline::Date(d1), Deadline::DateTime(dt2)) => d1
                .and_hms_opt(0, 0, 0)
                .map_or(Ordering::Less, |dt1| dt1.cmp(dt2).then(Ordering::Less)),
            (Deadline::DateTime(dt1), Deadline::Date(d2)) => {
                d2.and_hms_opt(0, 0, 0).map_or(Ordering::Greater, |dt2| {
                    dt1.cmp(&dt2).then(Ordering::Greater)
                })
            }
            (Deadline::Date(_), _) => Ordering::Less,
            (Deadline::DateTime(_), Deadline::Uninterpretable(_)) => Ordering::Less,
            (Deadline::Uninterpretable(_), _) => Ordering::Greater,
        }
    }
}

/// Helper to parse deadline strings (public re-export for use in edit generation).
pub fn parse_deadline_pub(value: &str) -> Deadline {
    parse_deadline(value)
}

/// Helper to parse deadline strings
pub(super) fn parse_deadline(value: &str) -> Deadline {
    if let Ok(datetime) = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M") {
        Deadline::DateTime(datetime)
    } else if let Ok(date) = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        Deadline::Date(date)
    } else {
        Deadline::Uninterpretable(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(deprecated)]
    fn test_deadline_sorting_order() -> Result<(), Box<dyn std::error::Error>> {
        let mut values = [
            Deadline::Date(chrono::NaiveDate::from_ymd(2022, 1, 1)),
            Deadline::DateTime(chrono::NaiveDateTime::from_timestamp(1672531199, 0)),
            Deadline::Uninterpretable(String::from("Hello")),
            Deadline::Date(chrono::NaiveDate::from_ymd(2023, 1, 1)),
            Deadline::Date(chrono::NaiveDate::from_ymd(2022, 12, 31)),
            Deadline::DateTime(chrono::NaiveDateTime::from_timestamp(1672531200, 0)),
            Deadline::Uninterpretable(String::from("World")),
        ];

        let gt = [
            Deadline::Date(chrono::NaiveDate::from_ymd(2022, 1, 1)),
            Deadline::Date(chrono::NaiveDate::from_ymd(2022, 12, 31)),
            Deadline::DateTime(chrono::NaiveDateTime::from_timestamp(1672531199, 0)),
            Deadline::Date(chrono::NaiveDate::from_ymd(2023, 1, 1)),
            Deadline::DateTime(chrono::NaiveDateTime::from_timestamp(1672531200, 0)),
            Deadline::Uninterpretable(String::from("Hello")),
            Deadline::Uninterpretable(String::from("World")),
        ];

        values.sort();

        for (v, x) in values.iter().zip(gt.iter()) {
            assert_eq!(v, x);
        }
        Ok(())
    }
}
