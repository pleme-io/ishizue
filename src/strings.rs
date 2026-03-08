//! String utility functions for Neovim plugins.
//!
//! Pure Rust helpers for common string operations that come up repeatedly
//! in plugin development: truncation with ellipsis, padding, splitting.

/// Truncate `s` to at most `max_width` display characters. When truncated,
/// the last characters are replaced with `ellipsis` so the total length
/// (including ellipsis) does not exceed `max_width`.
///
/// If `max_width` is less than the ellipsis length, the string is truncated
/// to `max_width` characters with no ellipsis.
///
/// # Examples
///
/// ```
/// assert_eq!(ishizue::strings::truncate("hello world", 8, "..."), "hello...");
/// assert_eq!(ishizue::strings::truncate("hi", 10, "..."), "hi");
/// ```
#[must_use]
pub fn truncate(s: &str, max_width: usize, ellipsis: &str) -> String {
    let char_count = s.chars().count();

    if char_count <= max_width {
        return s.to_owned();
    }

    let ellipsis_len = ellipsis.chars().count();

    if max_width <= ellipsis_len {
        // Not enough room for ellipsis — just hard-truncate.
        return s.chars().take(max_width).collect();
    }

    let keep = max_width - ellipsis_len;
    let mut result: String = s.chars().take(keep).collect();
    result.push_str(ellipsis);
    result
}

/// Pad `s` on the right with spaces to reach `width`. If `s` is already
/// at least `width` characters, it is returned unchanged.
///
/// # Examples
///
/// ```
/// assert_eq!(ishizue::strings::pad_right("hi", 5), "hi   ");
/// assert_eq!(ishizue::strings::pad_right("hello", 3), "hello");
/// ```
#[must_use]
pub fn pad_right(s: &str, width: usize) -> String {
    let char_count = s.chars().count();
    if char_count >= width {
        return s.to_owned();
    }
    let padding = width - char_count;
    let mut result = s.to_owned();
    result.extend(std::iter::repeat_n(' ', padding));
    result
}

/// Pad `s` on the left with spaces to reach `width`. If `s` is already
/// at least `width` characters, it is returned unchanged.
///
/// # Examples
///
/// ```
/// assert_eq!(ishizue::strings::pad_left("hi", 5), "   hi");
/// assert_eq!(ishizue::strings::pad_left("hello", 3), "hello");
/// ```
#[must_use]
pub fn pad_left(s: &str, width: usize) -> String {
    let char_count = s.chars().count();
    if char_count >= width {
        return s.to_owned();
    }
    let padding = width - char_count;
    let mut result = String::with_capacity(s.len() + padding);
    result.extend(std::iter::repeat_n(' ', padding));
    result.push_str(s);
    result
}

/// Split `s` on the first occurrence of `delim` and return both halves.
/// Returns `None` if `delim` is not found.
///
/// # Examples
///
/// ```
/// assert_eq!(
///     ishizue::strings::split_first("key=value=extra", '='),
///     Some(("key", "value=extra")),
/// );
/// assert_eq!(ishizue::strings::split_first("no delimiter", '='), None);
/// ```
#[must_use]
pub fn split_first(s: &str, delim: char) -> Option<(&str, &str)> {
    s.find(delim).map(|idx| (&s[..idx], &s[idx + delim.len_utf8()..]))
}

/// Trim whitespace from both ends of `s`. Convenience wrapper around
/// [`str::trim`] that returns an owned `String`.
///
/// # Examples
///
/// ```
/// assert_eq!(ishizue::strings::trim("  hello  "), "hello");
/// ```
#[must_use]
pub fn trim(s: &str) -> String {
    s.trim().to_owned()
}

/// Trim whitespace from the start (left side) of `s`.
///
/// # Examples
///
/// ```
/// assert_eq!(ishizue::strings::trim_start("  hello  "), "hello  ");
/// ```
#[must_use]
pub fn trim_start(s: &str) -> String {
    s.trim_start().to_owned()
}

/// Trim whitespace from the end (right side) of `s`.
///
/// # Examples
///
/// ```
/// assert_eq!(ishizue::strings::trim_end("  hello  "), "  hello");
/// ```
#[must_use]
pub fn trim_end(s: &str) -> String {
    s.trim_end().to_owned()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- truncate -----------------------------------------------------------

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate("hi", 10, "..."), "hi");
    }

    #[test]
    fn truncate_exact_length_unchanged() {
        assert_eq!(truncate("hello", 5, "..."), "hello");
    }

    #[test]
    fn truncate_adds_ellipsis() {
        assert_eq!(truncate("hello world", 8, "..."), "hello...");
    }

    #[test]
    fn truncate_single_char_ellipsis() {
        assert_eq!(truncate("abcdef", 4, "~"), "abc~");
    }

    #[test]
    fn truncate_max_less_than_ellipsis() {
        // max_width=2, ellipsis="..." (3 chars) -> hard truncate to 2
        assert_eq!(truncate("abcdef", 2, "..."), "ab");
    }

    #[test]
    fn truncate_zero_width() {
        assert_eq!(truncate("hello", 0, "..."), "");
    }

    #[test]
    fn truncate_empty_string() {
        assert_eq!(truncate("", 5, "..."), "");
    }

    #[test]
    fn truncate_empty_ellipsis() {
        assert_eq!(truncate("abcdef", 3, ""), "abc");
    }

    #[test]
    fn truncate_unicode() {
        // Each emoji is 1 char
        assert_eq!(truncate("abcdef", 5, ".."), "abc..");
    }

    // -- pad_right ----------------------------------------------------------

    #[test]
    fn pad_right_adds_spaces() {
        assert_eq!(pad_right("hi", 5), "hi   ");
    }

    #[test]
    fn pad_right_exact_width() {
        assert_eq!(pad_right("hello", 5), "hello");
    }

    #[test]
    fn pad_right_longer_unchanged() {
        assert_eq!(pad_right("hello world", 5), "hello world");
    }

    #[test]
    fn pad_right_zero_width() {
        assert_eq!(pad_right("hi", 0), "hi");
    }

    #[test]
    fn pad_right_empty_string() {
        assert_eq!(pad_right("", 3), "   ");
    }

    // -- pad_left -----------------------------------------------------------

    #[test]
    fn pad_left_adds_spaces() {
        assert_eq!(pad_left("hi", 5), "   hi");
    }

    #[test]
    fn pad_left_exact_width() {
        assert_eq!(pad_left("hello", 5), "hello");
    }

    #[test]
    fn pad_left_longer_unchanged() {
        assert_eq!(pad_left("hello world", 5), "hello world");
    }

    #[test]
    fn pad_left_zero_width() {
        assert_eq!(pad_left("hi", 0), "hi");
    }

    #[test]
    fn pad_left_empty_string() {
        assert_eq!(pad_left("", 3), "   ");
    }

    // -- split_first --------------------------------------------------------

    #[test]
    fn split_first_basic() {
        assert_eq!(split_first("key=value", '='), Some(("key", "value")));
    }

    #[test]
    fn split_first_multiple_delimiters() {
        assert_eq!(
            split_first("key=value=extra", '='),
            Some(("key", "value=extra")),
        );
    }

    #[test]
    fn split_first_no_delimiter() {
        assert_eq!(split_first("no delimiter", '='), None);
    }

    #[test]
    fn split_first_delimiter_at_start() {
        assert_eq!(split_first("=value", '='), Some(("", "value")));
    }

    #[test]
    fn split_first_delimiter_at_end() {
        assert_eq!(split_first("key=", '='), Some(("key", "")));
    }

    #[test]
    fn split_first_only_delimiter() {
        assert_eq!(split_first("=", '='), Some(("", "")));
    }

    #[test]
    fn split_first_empty_string() {
        assert_eq!(split_first("", '='), None);
    }

    // -- trim ---------------------------------------------------------------

    #[test]
    fn trim_both_sides() {
        assert_eq!(trim("  hello  "), "hello");
    }

    #[test]
    fn trim_no_whitespace() {
        assert_eq!(trim("hello"), "hello");
    }

    #[test]
    fn trim_only_whitespace() {
        assert_eq!(trim("   "), "");
    }

    #[test]
    fn trim_tabs_and_newlines() {
        assert_eq!(trim("\t\nhello\n\t"), "hello");
    }

    // -- trim_start ---------------------------------------------------------

    #[test]
    fn trim_start_removes_leading() {
        assert_eq!(trim_start("  hello  "), "hello  ");
    }

    #[test]
    fn trim_start_no_leading_whitespace() {
        assert_eq!(trim_start("hello  "), "hello  ");
    }

    // -- trim_end -----------------------------------------------------------

    #[test]
    fn trim_end_removes_trailing() {
        assert_eq!(trim_end("  hello  "), "  hello");
    }

    #[test]
    fn trim_end_no_trailing_whitespace() {
        assert_eq!(trim_end("  hello"), "  hello");
    }
}
