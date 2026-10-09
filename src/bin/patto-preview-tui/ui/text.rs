/// `text` cut to `max_chars` characters, the last one replaced by `…` when
/// anything was cut.
pub(super) fn truncate_with_ellipsis(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let kept: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{kept}…")
}

#[cfg(test)]
mod tests {
    use super::truncate_with_ellipsis;

    #[test]
    fn text_that_fits_is_returned_unchanged() {
        assert_eq!(truncate_with_ellipsis("hello", 5), "hello");
    }

    #[test]
    fn longer_text_ends_with_an_ellipsis_within_the_limit() {
        assert_eq!(truncate_with_ellipsis("hello world", 5), "hell…");
    }

    #[test]
    fn a_zero_limit_leaves_only_the_ellipsis() {
        assert_eq!(truncate_with_ellipsis("hello", 0), "…");
    }
}
