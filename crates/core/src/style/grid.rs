use gpui::GridPlacement;

/// Parse a single CSS grid-line token into a [`GridPlacement`].
///
/// Accepted forms:
/// - `"auto"` → [`GridPlacement::Auto`]
/// - `"span N"` → [`GridPlacement::Span(N)`] (N is a positive integer)
/// - bare integer (e.g. `"3"`, `"-1"`) → [`GridPlacement::Line(n)`]
pub(super) fn parse_grid_line_token(s: &str) -> Option<GridPlacement> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("auto") {
        return Some(GridPlacement::Auto);
    }
    let lower = s.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("span") {
        if let Ok(n) = rest.trim().parse::<u16>() {
            if n > 0 {
                return Some(GridPlacement::Span(n));
            }
        }
        return None;
    }
    s.parse::<i16>().ok().map(GridPlacement::Line)
}

/// Parse a CSS `grid-column` / `grid-row` shorthand string into a
/// `(start, end)` pair of [`GridPlacement`]s.
///
/// Accepted forms:
/// - `"A / B"` → `(Some(A), Some(B))` where A and B are line tokens
/// - `"span N"` → `(Some(Span(N)), Some(Span(N)))` — mirrors GPUI's `col_span` semantics
/// - single token (`"auto"`, `"3"`, `"-1"`) → `(Some(token), None)`
pub(super) fn parse_grid_shorthand(s: &str) -> (Option<GridPlacement>, Option<GridPlacement>) {
    let s = s.trim();
    if let Some(slash) = s.find('/') {
        let start = parse_grid_line_token(&s[..slash]);
        let end = parse_grid_line_token(&s[slash + 1..]);
        return (start, end);
    }
    // A bare Span mirrors GPUI col_span semantics (both endpoints).
    match parse_grid_line_token(s) {
        Some(GridPlacement::Span(n)) => {
            (Some(GridPlacement::Span(n)), Some(GridPlacement::Span(n)))
        }
        other => (other, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- parse_grid_line_token ----

    #[test]
    fn auto_token() {
        assert!(matches!(
            parse_grid_line_token("auto"),
            Some(GridPlacement::Auto)
        ));
    }

    #[test]
    fn auto_token_case_insensitive() {
        assert!(matches!(
            parse_grid_line_token("AUTO"),
            Some(GridPlacement::Auto)
        ));
        assert!(matches!(
            parse_grid_line_token("Auto"),
            Some(GridPlacement::Auto)
        ));
    }

    #[test]
    fn span_token() {
        assert!(matches!(
            parse_grid_line_token("span 2"),
            Some(GridPlacement::Span(2))
        ));
        assert!(matches!(
            parse_grid_line_token("SPAN 3"),
            Some(GridPlacement::Span(3))
        ));
    }

    #[test]
    fn span_zero_is_invalid() {
        assert!(parse_grid_line_token("span 0").is_none());
    }

    #[test]
    fn positive_line_number() {
        assert!(matches!(
            parse_grid_line_token("3"),
            Some(GridPlacement::Line(3))
        ));
        assert!(matches!(
            parse_grid_line_token("1"),
            Some(GridPlacement::Line(1))
        ));
    }

    #[test]
    fn negative_line_number() {
        assert!(matches!(
            parse_grid_line_token("-1"),
            Some(GridPlacement::Line(-1))
        ));
    }

    #[test]
    fn invalid_token_returns_none() {
        assert!(parse_grid_line_token("").is_none());
        assert!(parse_grid_line_token("abc").is_none());
        assert!(parse_grid_line_token("span").is_none());
    }

    // ---- parse_grid_shorthand ----

    #[test]
    fn slash_form_start_and_end() {
        let (start, end) = parse_grid_shorthand("1 / 3");
        assert!(matches!(start, Some(GridPlacement::Line(1))));
        assert!(matches!(end, Some(GridPlacement::Line(3))));
    }

    #[test]
    fn slash_form_with_span() {
        let (start, end) = parse_grid_shorthand("1 / span 2");
        assert!(matches!(start, Some(GridPlacement::Line(1))));
        assert!(matches!(end, Some(GridPlacement::Span(2))));
    }

    #[test]
    fn bare_span_mirrors_both_ends() {
        let (start, end) = parse_grid_shorthand("span 2");
        assert!(matches!(start, Some(GridPlacement::Span(2))));
        assert!(matches!(end, Some(GridPlacement::Span(2))));
    }

    #[test]
    fn single_token_returns_start_only() {
        let (start, end) = parse_grid_shorthand("auto");
        assert!(matches!(start, Some(GridPlacement::Auto)));
        assert!(end.is_none());

        let (start, end) = parse_grid_shorthand("3");
        assert!(matches!(start, Some(GridPlacement::Line(3))));
        assert!(end.is_none());
    }

    #[test]
    fn slash_with_invalid_side_returns_none_for_that_side() {
        let (start, end) = parse_grid_shorthand("1 / abc");
        assert!(matches!(start, Some(GridPlacement::Line(1))));
        assert!(end.is_none());
    }
}
