//! Cursor movement over lists that mix headers with selectable rows.

use tui_widget_list::ListState;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Step {
    Next,
    Prev,
}

/// Index of the next (or previous) selectable entry after `selected`,
/// wrapping around; `None` when nothing is selectable.
pub(crate) fn step_selectable(
    len: usize,
    selected: Option<usize>,
    step: Step,
    is_selectable: impl Fn(usize) -> bool,
) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let advance = |i: usize| match step {
        Step::Next => (i + 1) % len,
        Step::Prev => (i + len - 1) % len,
    };
    let mut idx = advance(selected.unwrap_or(0));
    for _ in 0..len {
        if is_selectable(idx) {
            return Some(idx);
        }
        idx = advance(idx);
    }
    None
}

pub(crate) fn step_list<T>(
    state: &mut ListState,
    entries: &[T],
    is_selectable: impl Fn(&T) -> bool,
    step: Step,
) {
    if let Some(idx) = step_selectable(entries.len(), state.selected, step, |i| {
        is_selectable(&entries[i])
    }) {
        state.select(Some(idx));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SELECTABLE: [bool; 5] = [false, true, false, true, false];

    #[test]
    fn next_skips_unselectable_entries() {
        let next = step_selectable(5, Some(1), Step::Next, |i| SELECTABLE[i]);
        assert_eq!(next, Some(3));
    }

    #[test]
    fn next_wraps_to_the_first_selectable_entry() {
        let next = step_selectable(5, Some(3), Step::Next, |i| SELECTABLE[i]);
        assert_eq!(next, Some(1));
    }

    #[test]
    fn prev_wraps_to_the_last_selectable_entry() {
        let prev = step_selectable(5, Some(1), Step::Prev, |i| SELECTABLE[i]);
        assert_eq!(prev, Some(3));
    }

    #[test]
    fn without_a_selection_the_search_starts_after_the_first_entry() {
        let next = step_selectable(5, None, Step::Next, |i| SELECTABLE[i]);
        assert_eq!(next, Some(1));
    }

    #[test]
    fn a_list_with_nothing_selectable_yields_none() {
        assert_eq!(step_selectable(3, Some(0), Step::Next, |_| false), None);
        assert_eq!(step_selectable(0, None, Step::Prev, |_| true), None);
    }
}
