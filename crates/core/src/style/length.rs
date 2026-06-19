use gpui::{AbsoluteLength, DefiniteLength, Length, px, relative, rems};

use crate::style::fields::LengthValue;

/// Parse a CSS-style length string into a [`LengthValue`].
///
/// Accepted forms:
/// - `"auto"`         → `Auto`
/// - `"<n>%"`         → `Percent(n)` (whole percent, e.g. 50.0 = 50 %)
/// - `"<n>px"`        → `Px(n)`
/// - `"<n>rem"`       → `Rem(n)`
/// - `"<n>"` (bare)   → `Px(n)` (numeric string without unit, same as a number)
pub(super) fn parse_length_str(s: &str) -> Option<LengthValue> {
    let s = s.trim();
    if s == "auto" {
        return Some(LengthValue::Auto);
    }
    if let Some(p) = s.strip_suffix('%') {
        return p.trim().parse::<f32>().ok().map(LengthValue::Percent);
    }
    if let Some(p) = s.strip_suffix("px") {
        return p.trim().parse::<f32>().ok().map(LengthValue::Px);
    }
    if let Some(p) = s.strip_suffix("rem") {
        return p.trim().parse::<f32>().ok().map(LengthValue::Rem);
    }
    s.parse::<f32>().ok().map(LengthValue::Px)
}

impl LengthValue {
    /// All units including `auto` and `%`. Used for `width`/`height`.
    pub(crate) fn to_length(self) -> Length {
        match self {
            LengthValue::Px(n) => px(n).into(),
            LengthValue::Rem(n) => rems(n).into(),
            LengthValue::Percent(p) => relative(p / 100.0).into(),
            LengthValue::Auto => Length::Auto,
        }
    }

    /// px/rem/% only; `auto` → `None`. Used for `padding`/`gap`.
    pub(crate) fn to_definite(self) -> Option<DefiniteLength> {
        match self {
            LengthValue::Px(n) => Some(px(n).into()),
            LengthValue::Rem(n) => Some(rems(n).into()),
            LengthValue::Percent(p) => Some(relative(p / 100.0)),
            LengthValue::Auto => None,
        }
    }

    /// px/rem only; `%`/`auto` → `None`. Used for `border_radius`/`font_size`.
    pub(crate) fn to_absolute(self) -> Option<AbsoluteLength> {
        match self {
            LengthValue::Px(n) => Some(px(n).into()),
            LengthValue::Rem(n) => Some(rems(n).into()),
            LengthValue::Percent(_) | LengthValue::Auto => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::fields::LengthValue;

    // ---- parse_length_str ----

    #[test]
    fn auto_keyword() {
        assert_eq!(parse_length_str("auto"), Some(LengthValue::Auto));
    }

    #[test]
    fn percent_value() {
        assert_eq!(parse_length_str("50%"), Some(LengthValue::Percent(50.0)));
        assert_eq!(parse_length_str("100%"), Some(LengthValue::Percent(100.0)));
        assert_eq!(parse_length_str("0%"), Some(LengthValue::Percent(0.0)));
    }

    #[test]
    fn px_value() {
        assert_eq!(parse_length_str("10px"), Some(LengthValue::Px(10.0)));
        assert_eq!(parse_length_str("0px"), Some(LengthValue::Px(0.0)));
        assert_eq!(parse_length_str("1.5px"), Some(LengthValue::Px(1.5)));
    }

    #[test]
    fn rem_value() {
        assert_eq!(parse_length_str("1rem"), Some(LengthValue::Rem(1.0)));
        assert_eq!(parse_length_str("1.5rem"), Some(LengthValue::Rem(1.5)));
    }

    #[test]
    fn bare_number_becomes_px() {
        assert_eq!(parse_length_str("12"), Some(LengthValue::Px(12.0)));
        assert_eq!(parse_length_str("0"), Some(LengthValue::Px(0.0)));
    }

    #[test]
    fn whitespace_is_trimmed() {
        assert_eq!(parse_length_str("  auto  "), Some(LengthValue::Auto));
        assert_eq!(parse_length_str(" 10px "), Some(LengthValue::Px(10.0)));
    }

    #[test]
    fn invalid_string_returns_none() {
        assert!(parse_length_str("").is_none());
        assert!(parse_length_str("abc").is_none());
        assert!(parse_length_str("px").is_none());
    }

    // ---- LengthValue::to_definite ----

    #[test]
    fn to_definite_auto_is_none() {
        assert!(LengthValue::Auto.to_definite().is_none());
    }

    #[test]
    fn to_definite_px_is_some() {
        assert!(LengthValue::Px(10.0).to_definite().is_some());
    }

    #[test]
    fn to_definite_rem_is_some() {
        assert!(LengthValue::Rem(1.0).to_definite().is_some());
    }

    #[test]
    fn to_definite_percent_is_some() {
        assert!(LengthValue::Percent(50.0).to_definite().is_some());
    }

    // ---- LengthValue::to_absolute ----

    #[test]
    fn to_absolute_auto_is_none() {
        assert!(LengthValue::Auto.to_absolute().is_none());
    }

    #[test]
    fn to_absolute_percent_is_none() {
        assert!(LengthValue::Percent(50.0).to_absolute().is_none());
    }

    #[test]
    fn to_absolute_px_is_some() {
        assert!(LengthValue::Px(10.0).to_absolute().is_some());
    }

    #[test]
    fn to_absolute_rem_is_some() {
        assert!(LengthValue::Rem(1.0).to_absolute().is_some());
    }

    // ---- LengthValue::to_length ----

    #[test]
    fn to_length_auto_is_auto() {
        let l = LengthValue::Auto.to_length();
        assert!(matches!(l, gpui::Length::Auto));
    }

    #[test]
    fn to_length_px_is_definite() {
        let l = LengthValue::Px(10.0).to_length();
        assert!(matches!(l, gpui::Length::Definite(_)));
    }

    #[test]
    fn to_length_percent_is_definite() {
        let l = LengthValue::Percent(50.0).to_length();
        assert!(matches!(l, gpui::Length::Definite(_)));
    }
}
