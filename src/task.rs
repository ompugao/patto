/// Task time tracking types and transition detection helpers.
///
/// This module centralises all domain types that describe the *state* of a task
/// and the *transitions* between states.  Edit-generation lives in
/// `crate::lsp::task_edits` so that LSP-specific types (TextEdit, Position, …)
/// are not pulled into the core domain layer.
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::parser::{Deadline, TaskStatus};

// ─── Duration ────────────────────────────────────────────────────────────────

/// A human-readable duration stored as whole hours + minutes.
///
/// Canonical serialised form: `1h30m`, `45m`, `2h`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Duration {
    pub hours: u32,
    pub minutes: u32,
}

impl Duration {
    pub fn new(hours: u32, minutes: u32) -> Self {
        let total_minutes = hours * 60 + minutes;
        Self {
            hours: total_minutes / 60,
            minutes: total_minutes % 60,
        }
    }

    pub fn from_minutes(total: u32) -> Self {
        Self {
            hours: total / 60,
            minutes: total % 60,
        }
    }

    pub fn total_minutes(&self) -> u32 {
        self.hours * 60 + self.minutes
    }

    pub fn is_zero(&self) -> bool {
        self.hours == 0 && self.minutes == 0
    }
}

impl std::ops::Add for Duration {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::from_minutes(self.total_minutes() + rhs.total_minutes())
    }
}

impl std::ops::AddAssign for Duration {
    fn add_assign(&mut self, rhs: Self) {
        *self = Self::from_minutes(self.total_minutes() + rhs.total_minutes());
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.hours, self.minutes) {
            (h, 0) => write!(f, "{}h", h),
            (0, m) => write!(f, "{}m", m),
            (h, m) => write!(f, "{}h{}m", h, m),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseDurationError;

impl fmt::Display for ParseDurationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid duration string (expected NhMm / Nh / Mm)")
    }
}

impl FromStr for Duration {
    type Err = ParseDurationError;

    /// Accepts `"1h30m"`, `"2h"`, `"45m"`, `"0h"`, `"0m"`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ParseDurationError);
        }

        let mut hours: u32 = 0;
        let mut minutes: u32 = 0;
        let mut rest = s;

        if let Some(h_pos) = rest.find('h') {
            hours = rest[..h_pos].parse().map_err(|_| ParseDurationError)?;
            rest = &rest[h_pos + 1..];
        }

        if let Some(m_pos) = rest.find('m') {
            minutes = rest[..m_pos].parse().map_err(|_| ParseDurationError)?;
            rest = &rest[m_pos + 1..];
        }

        // Trailing garbage → error
        if !rest.is_empty() {
            return Err(ParseDurationError);
        }

        // Must have consumed at least one unit
        if s.find('h').is_none() && s.find('m').is_none() {
            return Err(ParseDurationError);
        }

        Ok(Self::new(hours, minutes))
    }
}

// ─── TaskSnapshot ─────────────────────────────────────────────────────────────

/// A complete snapshot of one task line's state, captured from the AST.
///
/// Both the old and new AST are converted to `HashMap<usize, TaskSnapshot>`
/// (keyed by line row) so that the diff logic only deals with `TaskSnapshot`
/// values instead of raw `Property` pattern-matches.
#[derive(Debug, Clone)]
pub struct TaskSnapshot {
    pub row: usize,
    pub status: TaskStatus,
    pub due: Deadline,
    pub scheduled: Option<Deadline>,
    pub completed_at: Option<Deadline>,
    pub started_at: Option<Deadline>,
    pub time_spent: Option<Duration>,
    /// Byte span of the entire task property token within the line string.
    /// Used by edit generators to avoid re-scanning raw text.
    pub prop_span: crate::parser::Span,
    /// Is the status value a recognised canonical keyword (`todo`/`doing`/`done`
    /// and aliases)?  `false` when the parser fell back to the default because
    /// the value was unrecognised (e.g. `status=doin` during a mid-edit keystroke).
    /// Transitions from/to non-canonical states are ignored by the diff logic.
    pub status_is_canonical: bool,
    /// Is the on-disk form a shorthand token (`-YYYY-MM-DD`) rather than a
    /// full `{@task …}` block?
    pub is_shorthand: bool,
    /// Full raw text of the line that owns this task (needed for UTF-16 offset
    /// conversion when generating edits).
    pub line_text: String,
}

// ─── TaskTransition ───────────────────────────────────────────────────────────

/// A detected state change for a single task line between two AST snapshots.
#[derive(Debug)]
pub enum TaskTransition {
    /// Any status → Done (no `completed_at` present yet in the new snapshot).
    BecameDone {
        old: TaskSnapshot,
        new: TaskSnapshot,
    },
    /// Any status → Doing (no `started_at` present yet in the new snapshot).
    BecameDoing {
        old: TaskSnapshot,
        new: TaskSnapshot,
    },
    /// Doing → Todo (clock-out without completing; `time_spent` should be updated).
    BecameTodo {
        old: TaskSnapshot,
        new: TaskSnapshot,
    },
}

impl TaskTransition {
    /// Row number of the affected line (from the *new* snapshot).
    pub fn row(&self) -> usize {
        match self {
            TaskTransition::BecameDone { new, .. } => new.row,
            TaskTransition::BecameDoing { new, .. } => new.row,
            TaskTransition::BecameTodo { new, .. } => new.row,
        }
    }

    pub fn new_snapshot(&self) -> &TaskSnapshot {
        match self {
            TaskTransition::BecameDone { new, .. } => new,
            TaskTransition::BecameDoing { new, .. } => new,
            TaskTransition::BecameTodo { new, .. } => new,
        }
    }

    pub fn old_snapshot(&self) -> &TaskSnapshot {
        match self {
            TaskTransition::BecameDone { old, .. } => old,
            TaskTransition::BecameDoing { old, .. } => old,
            TaskTransition::BecameTodo { old, .. } => old,
        }
    }
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_roundtrip() {
        // Canonical form: hours-only, minutes-only, or both.
        // "0m" and "0h" both normalise to zero; "0h" is the canonical display.
        for (input, expected) in &[
            ("1h30m", "1h30m"),
            ("2h", "2h"),
            ("45m", "45m"),
            ("0h", "0h"),
            ("0m", "0h"), // normalises to hours form
            ("10h5m", "10h5m"),
        ] {
            let d: Duration = input.parse().unwrap();
            assert_eq!(d.to_string(), *expected, "input={}", input);
        }
    }

    #[test]
    fn duration_add() {
        let a = Duration::new(1, 45);
        let b = Duration::new(0, 30);
        assert_eq!(a + b, Duration::new(2, 15));
    }

    #[test]
    fn duration_parse_errors() {
        assert!("".parse::<Duration>().is_err());
        assert!("abc".parse::<Duration>().is_err());
        assert!("1h30".parse::<Duration>().is_err()); // trailing digits, no unit
    }
}
