//! Small text helpers shared by the printed report and the interactive viewer.

/// Shortens `s` to `max` characters, marking the cut with an ellipsis.
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('\u{2026}');
    out
}

/// Pads `s` with spaces to `width` characters, leaving longer strings alone.
pub fn pad(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - len))
    }
}

/// Trims to `width` and then pads, so the result always occupies `width` cells.
pub fn fit(s: &str, width: usize) -> String {
    pad(&truncate(s, width), width)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "panicking is the failure mode a test wants"
    )]
    use super::*;

    #[test]
    fn truncate_marks_the_cut() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello", 5), "hello");
        assert_eq!(truncate("hello", 4), "hel\u{2026}");
    }

    #[test]
    fn truncate_counts_characters_not_bytes() {
        assert_eq!(truncate("Åland Islands", 6).chars().count(), 6);
    }

    #[test]
    fn pad_and_fit_produce_fixed_width_cells() {
        assert_eq!(pad("ab", 4), "ab  ");
        assert_eq!(pad("abcdef", 4), "abcdef");
        assert_eq!(fit("abcdef", 4).chars().count(), 4);
        assert_eq!(fit("ab", 4), "ab  ");
    }
}
