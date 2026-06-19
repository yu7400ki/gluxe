//! Generic `font-family` expansion.
//!
//! gluxe stores `fontFamily` as an ordered token list (see `parse.rs`). The CSS
//! generic families (`sans-serif`, `serif`, `monospace`, `system-ui`) carry no
//! concrete font on their own, so [`expand_chain`] rewrites them into the
//! platform's default fonts before the list is handed to GPUI as
//! `font_family` (primary) + `font_fallbacks` (the rest).
//!
//! Windows/macOS concrete names are transcribed from browser-default-fonts
//! (Firefox `x-western` records): <https://github.com/yu7400ki/browser-default-fonts>.
//! Linux uses the common DejaVu → Liberation → Noto stack instead (its
//! `x-western` data is only fontconfig aliases, which GPUI cannot resolve — see
//! [`GenericFamily::concrete`]). Only the Western (Latin) defaults are modelled
//! here; CJK and other scripts are left to GPUI's per-glyph fallback within the
//! font stack.

use gpui::SharedString;

/// The CSS generic font families gluxe expands to platform fonts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GenericFamily {
    SansSerif,
    Serif,
    Monospace,
    SystemUi,
}

impl GenericFamily {
    /// Match a token against the four supported generics, case-insensitively.
    /// Returns `None` for any concrete (non-generic) family name; the caller
    /// keeps such names verbatim (preserving their original casing).
    fn parse(token: &str) -> Option<Self> {
        match token.to_ascii_lowercase().as_str() {
            "sans-serif" => Some(Self::SansSerif),
            "serif" => Some(Self::Serif),
            "monospace" => Some(Self::Monospace),
            "system-ui" => Some(Self::SystemUi),
            _ => None,
        }
    }

    /// Concrete font names for this generic, ordered best-first.
    ///
    /// `system-ui` maps to GPUI's special `.SystemUIFont` sentinel on every
    /// platform. The Windows/macOS text generics come from browser-default-fonts'
    /// `x-western` data and are real DB family names that resolve by exact match.
    ///
    /// Linux (and any non-Windows/macOS target) must list concrete families too:
    /// GPUI's cosmic-text backend does an exact-name DB lookup and never maps the
    /// fontconfig aliases (`"sans-serif"` etc.) to a generic family, so passing
    /// the aliases through would fail to resolve and fall back to GPUI's global
    /// stack (rendering serif/monospace as sans). We use the common Linux stack
    /// (DejaVu → Liberation → Noto) instead.
    fn concrete(self) -> &'static [&'static str] {
        match self {
            Self::SystemUi => &[".SystemUIFont"],
            #[cfg(target_os = "windows")]
            Self::SansSerif => &["Arial"],
            #[cfg(target_os = "windows")]
            Self::Serif => &["Times New Roman"],
            #[cfg(target_os = "windows")]
            Self::Monospace => &["Consolas"],
            #[cfg(target_os = "macos")]
            Self::SansSerif => &["Helvetica", "Arial"],
            #[cfg(target_os = "macos")]
            Self::Serif => &["Times", "Times New Roman"],
            #[cfg(target_os = "macos")]
            Self::Monospace => &["Menlo"],
            #[cfg(not(any(target_os = "windows", target_os = "macos")))]
            Self::SansSerif => &["DejaVu Sans", "Liberation Sans", "Noto Sans"],
            #[cfg(not(any(target_os = "windows", target_os = "macos")))]
            Self::Serif => &["DejaVu Serif", "Liberation Serif", "Noto Serif"],
            #[cfg(not(any(target_os = "windows", target_os = "macos")))]
            Self::Monospace => &["DejaVu Sans Mono", "Liberation Mono", "Noto Sans Mono"],
        }
    }
}

/// Expand a parsed token list into a concrete font stack.
///
/// Generic families are replaced by their platform fonts ([`GenericFamily::concrete`]);
/// concrete names pass through unchanged (casing preserved). Duplicates are
/// removed while preserving first-occurrence order — e.g. `["Arial", "sans-serif"]`
/// on Windows yields just `["Arial"]` because `sans-serif` also expands to `Arial`.
///
/// The caller splits the result into `font_family` (first) + `font_fallbacks`
/// (rest). GPUI only panics on a missing font when the primary, *every* fallback,
/// and its own global font stack all fail to resolve — effectively impossible
/// here since the chain ends in a platform default — so no extra hardening is
/// needed.
pub(crate) fn expand_chain(tokens: &[String]) -> Vec<SharedString> {
    let mut out: Vec<SharedString> = Vec::with_capacity(tokens.len());
    let push = |name: SharedString, out: &mut Vec<SharedString>| {
        if !out.contains(&name) {
            out.push(name);
        }
    };
    for token in tokens {
        match GenericFamily::parse(token) {
            Some(generic) => {
                for &concrete in generic.concrete() {
                    push(SharedString::from(concrete), &mut out);
                }
            }
            None => push(SharedString::from(token.clone()), &mut out),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expand(tokens: &[&str]) -> Vec<String> {
        let owned: Vec<String> = tokens.iter().map(|s| s.to_string()).collect();
        expand_chain(&owned).iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn concrete_names_pass_through_with_casing_preserved() {
        assert_eq!(expand(&["Inter", "MyFont"]), vec!["Inter", "MyFont"]);
    }

    #[test]
    fn system_ui_maps_to_gpui_sentinel_case_insensitively() {
        assert_eq!(expand(&["system-ui"]), vec![".SystemUIFont"]);
        assert_eq!(expand(&["SYSTEM-UI"]), vec![".SystemUIFont"]);
        assert_eq!(expand(&["System-Ui"]), vec![".SystemUIFont"]);
    }

    #[test]
    fn dedup_preserves_first_occurrence_order() {
        // Concrete-only, so OS-independent.
        assert_eq!(
            expand(&["Inter", "Arial", "Inter", "Arial"]),
            vec!["Inter", "Arial"]
        );
        assert_eq!(expand(&["system-ui", "system-ui"]), vec![".SystemUIFont"]);
    }

    #[test]
    fn empty_input_yields_empty() {
        assert!(expand(&[]).is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_generics_expand_to_x_western_defaults() {
        assert_eq!(expand(&["sans-serif"]), vec!["Helvetica", "Arial"]);
        assert_eq!(expand(&["serif"]), vec!["Times", "Times New Roman"]);
        assert_eq!(expand(&["monospace"]), vec!["Menlo"]);
        // Generic after a concrete dup-of-its-expansion collapses (order kept).
        assert_eq!(expand(&["Arial", "sans-serif"]), vec!["Arial", "Helvetica"]);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_generics_expand_to_x_western_defaults() {
        assert_eq!(expand(&["sans-serif"]), vec!["Arial"]);
        assert_eq!(expand(&["serif"]), vec!["Times New Roman"]);
        assert_eq!(expand(&["monospace"]), vec!["Consolas"]);
        assert_eq!(expand(&["Arial", "sans-serif"]), vec!["Arial"]);
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    #[test]
    fn linux_generics_expand_to_concrete_stack() {
        // cosmic-text resolves by exact DB name, so generics must expand to real
        // families rather than passing the fontconfig aliases through.
        assert_eq!(
            expand(&["sans-serif"]),
            vec!["DejaVu Sans", "Liberation Sans", "Noto Sans"]
        );
        assert_eq!(
            expand(&["serif"]),
            vec!["DejaVu Serif", "Liberation Serif", "Noto Serif"]
        );
        assert_eq!(
            expand(&["monospace"]),
            vec!["DejaVu Sans Mono", "Liberation Mono", "Noto Sans Mono"]
        );
        // "Inter" then sans-serif → concrete stack appended (no overlap).
        assert_eq!(
            expand(&["Inter", "sans-serif"]),
            vec!["Inter", "DejaVu Sans", "Liberation Sans", "Noto Sans"]
        );
    }
}
