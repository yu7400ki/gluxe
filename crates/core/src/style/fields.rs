//! Style data vocabulary — the value types parsed from JS `style` props (and the
//! `floating` / `windowControlArea` element props), together with their string
//! parsers.
//!
//! These are the *inputs* to the style system: `style/parse.rs` builds them from
//! JS, while `style/apply.rs` and `style/reader.rs` consume them. `model.rs`
//! composes [`StyleFields`] and [`FloatingSpec`] into its `Props` and re-exports
//! this vocabulary for callers that reach it through `crate::model`.

use gpui::{DefiniteLength, GridPlacement, Rgba, WindowControlArea};

/// Box shadow specification — either a named Tailwind-style preset or one or
/// more custom shadow layers.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BoxShadowSpec {
    /// A named preset matching Tailwind's shadow scale:
    /// `"none"` | `"2xs"` | `"xs"` | `"sm"` | `"md"` | `"lg"` | `"xl"` | `"2xl"`.
    Preset(String),
    /// One or more custom shadow layers.
    Custom(Vec<ShadowValue>),
}

/// A single CSS box-shadow layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ShadowValue {
    pub(crate) offset_x: f32,
    pub(crate) offset_y: f32,
    pub(crate) blur_radius: f32,
    pub(crate) spread_radius: f32,
    /// RGBA colour (alpha included).
    pub(crate) color: Rgba,
    pub(crate) inset: bool,
}

/// Overflow mode for a single axis.
///
/// Maps to CSS `overflow-x` / `overflow-y`: `visible` (default), `hidden` (clip),
/// or `scroll` (clip + enable scrolling).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum OverflowMode {
    Visible,
    Hidden,
    Scroll,
}

impl OverflowMode {
    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "visible" => Some(Self::Visible),
            "hidden" => Some(Self::Hidden),
            "scroll" => Some(Self::Scroll),
            _ => None,
        }
    }
}

/// Which side of the anchor the floating element is placed on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum FloatingSide {
    Top,
    Bottom,
    Left,
    Right,
}

impl FloatingSide {
    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "top" => Some(Self::Top),
            "bottom" => Some(Self::Bottom),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            _ => None,
        }
    }
}

/// Alignment of the floating element along the anchor's cross axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum FloatingAlign {
    Start,
    Center,
    End,
}

impl FloatingAlign {
    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "start" => Some(Self::Start),
            "center" => Some(Self::Center),
            "end" => Some(Self::End),
            _ => None,
        }
    }
}

/// Parse a floating `area` string: `"<side>"` or `"<side> <align>"`.
/// First whitespace-separated token = side, optional second = align.
/// Missing/unknown side → default `Bottom`; missing/unknown align → default `Start`.
pub(crate) fn parse_floating_area(s: &str) -> (FloatingSide, FloatingAlign) {
    let mut tokens = s.split_whitespace();
    let side = tokens
        .next()
        .and_then(FloatingSide::parse)
        .unwrap_or(FloatingSide::Bottom);
    let align = tokens
        .next()
        .and_then(FloatingAlign::parse)
        .unwrap_or(FloatingAlign::Start);
    (side, align)
}

/// Positioning spec for a floating element bound to a named anchor.
///
/// Placement is resolved against the anchor's last-painted bounds and clamped to
/// the window (no opposite-side flip — overflow is handled by snapping, matching
/// GPUI's `anchored`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FloatingSpec {
    /// The `anchorName` this floating element binds to.
    pub(crate) anchor: String,
    pub(crate) side: FloatingSide,
    pub(crate) align: FloatingAlign,
    /// Gap from the anchor along the `side` direction (px/rem; `%`/`auto` ignored).
    pub(crate) offset: LengthValue,
    /// Minimum gap kept from the window edge when snapping on overflow (px/rem).
    pub(crate) margin: LengthValue,
    /// Draw-order priority among floating layers. `None` = leave GPUI default.
    pub(crate) priority: Option<u16>,
}

/// Parse the `windowControlArea` prop value into a GPUI [`WindowControlArea`].
///
/// Maps the four JS string values used in `<View windowControlArea="…">` to their
/// GPUI equivalents. Any unrecognized value yields `None`.
pub(crate) fn parse_window_control_area(s: &str) -> Option<WindowControlArea> {
    match s {
        "drag" => Some(WindowControlArea::Drag),
        "close" => Some(WindowControlArea::Close),
        "max" => Some(WindowControlArea::Max),
        "min" => Some(WindowControlArea::Min),
        _ => None,
    }
}

/// A length value that can be expressed in different units.
///
/// Bare JS numbers become `Px`. Strings are parsed as `{number}px`, `{number}%`,
/// `{number}rem`, or the keyword `"auto"`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum LengthValue {
    Px(f32),
    Rem(f32),
    /// Whole percentage, e.g. `50.0` represents 50 % of the parent's size.
    Percent(f32),
    Auto,
}

/// Visual-only style fields shared between base style and pseudo-selector overlays.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct StyleFields {
    pub(crate) display: Option<String>,
    /// CSS `flex` shorthand as a number (e.g. `flex: 1` → grow=1, shrink=1, basis=0).
    pub(crate) flex: Option<f32>,
    /// CSS `flex` shorthand as a keyword: `"auto"` | `"initial"` | `"none"`.
    pub(crate) flex_keyword: Option<String>,
    /// Individual flex-grow factor (overrides the shorthand if set).
    pub(crate) flex_grow: Option<f32>,
    /// Individual flex-shrink factor (overrides the shorthand if set).
    pub(crate) flex_shrink: Option<f32>,
    /// Initial main-size of a flex item (`flex-basis`).
    pub(crate) flex_basis: Option<LengthValue>,
    /// Whether flex items wrap onto multiple lines (`flex-wrap`).
    pub(crate) flex_wrap: Option<String>,
    pub(crate) flex_direction: Option<String>,
    /// How this specific item is aligned along the container's cross axis (`align-self`).
    pub(crate) align_self: Option<String>,
    /// Alignment of multiple lines within a flex container (`align-content`).
    pub(crate) align_content: Option<String>,
    pub(crate) width: Option<LengthValue>,
    pub(crate) height: Option<LengthValue>,
    // Padding: uniform → per-axis → per-side (CSS cascade order; auto ignored).
    pub(crate) padding: Option<LengthValue>,
    pub(crate) padding_x: Option<LengthValue>,
    pub(crate) padding_y: Option<LengthValue>,
    pub(crate) padding_top: Option<LengthValue>,
    pub(crate) padding_right: Option<LengthValue>,
    pub(crate) padding_bottom: Option<LengthValue>,
    pub(crate) padding_left: Option<LengthValue>,
    // Gap: uniform → per-axis.
    pub(crate) gap: Option<LengthValue>,
    pub(crate) gap_x: Option<LengthValue>,
    pub(crate) gap_y: Option<LengthValue>,
    pub(crate) align_items: Option<String>,
    pub(crate) justify_content: Option<String>,
    pub(crate) background_color: Option<Rgba>,
    pub(crate) border_radius: Option<LengthValue>,
    pub(crate) border_width: Option<LengthValue>,
    pub(crate) border_color: Option<Rgba>,
    pub(crate) color: Option<Rgba>,
    pub(crate) font_size: Option<LengthValue>,
    pub(crate) font_weight: Option<f32>, // 400 = normal, 700 = bold
    pub(crate) cursor: Option<String>, // CSS cursor value: "pointer" | "default" | "text" | "move" | "grab" | "grabbing" | "crosshair" | "not-allowed" | "no-drop" | "context-menu" | "copy" | "alias" | "vertical-text" | "ew-resize" | "ns-resize" | "nesw-resize" | "nwse-resize" | "col-resize" | "row-resize" | "n-resize" | "e-resize" | "s-resize" | "w-resize"
    pub(crate) white_space: Option<String>, // "nowrap" | "normal"
    pub(crate) text_overflow: Option<String>, // "ellipsis" | "clip"
    pub(crate) line_clamp: Option<f32>, // max number of visible lines
    pub(crate) overflow_x: Option<OverflowMode>,
    pub(crate) overflow_y: Option<OverflowMode>,
    // Margin: uniform → per-axis → per-side (auto allowed).
    pub(crate) margin: Option<LengthValue>,
    pub(crate) margin_x: Option<LengthValue>,
    pub(crate) margin_y: Option<LengthValue>,
    pub(crate) margin_top: Option<LengthValue>,
    pub(crate) margin_right: Option<LengthValue>,
    pub(crate) margin_bottom: Option<LengthValue>,
    pub(crate) margin_left: Option<LengthValue>,
    // Min / max size (auto allowed).
    pub(crate) min_width: Option<LengthValue>,
    pub(crate) min_height: Option<LengthValue>,
    pub(crate) max_width: Option<LengthValue>,
    pub(crate) max_height: Option<LengthValue>,
    pub(crate) aspect_ratio: Option<f32>,
    // Position (auto allowed for inset/sides).
    pub(crate) position: Option<String>, // "relative" | "absolute"
    pub(crate) inset: Option<LengthValue>,
    pub(crate) top: Option<LengthValue>,
    pub(crate) right: Option<LengthValue>,
    pub(crate) bottom: Option<LengthValue>,
    pub(crate) left: Option<LengthValue>,
    // ---- Visual effects ----
    /// Element opacity (0.0 = fully transparent, 1.0 = fully opaque).
    pub(crate) opacity: Option<f32>,
    /// CSS `visibility`: `"visible"` | `"hidden"` (hidden preserves layout space).
    pub(crate) visibility: Option<String>,
    /// CSS `border-style`: `"solid"` (default) | `"dashed"`.
    pub(crate) border_style: Option<String>,
    // Per-side border widths (override the uniform `border_width`).
    pub(crate) border_top_width: Option<LengthValue>,
    pub(crate) border_right_width: Option<LengthValue>,
    pub(crate) border_bottom_width: Option<LengthValue>,
    pub(crate) border_left_width: Option<LengthValue>,
    // Per-corner border radii (override the uniform `border_radius`).
    pub(crate) border_top_left_radius: Option<LengthValue>,
    pub(crate) border_top_right_radius: Option<LengthValue>,
    pub(crate) border_bottom_right_radius: Option<LengthValue>,
    pub(crate) border_bottom_left_radius: Option<LengthValue>,
    /// Width reserved for the scrollbar (only meaningful when overflow is `Scroll`).
    pub(crate) scrollbar_width: Option<LengthValue>,
    /// Box shadow — preset name or one or more custom layers.
    pub(crate) box_shadow: Option<BoxShadowSpec>,
    // ---- Text styling ----
    /// CSS `text-align`: `"left"` | `"center"` | `"right"`.
    pub(crate) text_align: Option<String>,
    /// CSS `font-style`: `"normal"` | `"italic"`.
    pub(crate) font_style: Option<String>,
    /// CSS `font-family` as an ordered token list (primary first, then fallbacks).
    /// Parsed from a string (CSS comma syntax) or an array (one token per element).
    /// Merge semantics are replace-not-concat: a higher-priority source's list
    /// wins wholesale (handled by the `PropReader::first` resolution in `parse.rs`).
    pub(crate) font_family: Option<Vec<String>>,
    /// CSS `line-height`. A bare number becomes `relative(n)` (font-size multiplier).
    pub(crate) line_height: Option<DefiniteLength>,
    /// CSS `text-decoration-line`: `"none"` | `"underline"` | `"line-through"` | combined.
    pub(crate) text_decoration_line: Option<String>,
    /// Decoration colour. Applied to underline and/or strikethrough.
    pub(crate) text_decoration_color: Option<Rgba>,
    /// CSS `text-decoration-style`: `"solid"` | `"wavy"` (underline only).
    pub(crate) text_decoration_style: Option<String>,
    /// CSS `text-decoration-thickness` (px only; other length units are ignored).
    pub(crate) text_decoration_thickness: Option<LengthValue>,
    /// Text highlight background colour (`text_bg` in GPUI).
    pub(crate) text_background_color: Option<Rgba>,
    /// OpenType font features: `Vec<(tag, value)>` where value 1 = on, 0 = off.
    pub(crate) font_features: Option<Vec<(String, u32)>>,
    // ---- TextInput caret / selection (read by text_input.rs; not applied to a div) ----
    /// Caret colour. `None` → falls back to the text `color`.
    pub(crate) caret_color: Option<Rgba>,
    /// Caret width (px/rem only; `%`/`auto` ignored). `None` → 1px default.
    pub(crate) caret_width: Option<LengthValue>,
    /// Selection-highlight background colour. `None` → built-in translucent blue.
    pub(crate) selection_color: Option<Rgba>,
    /// Placeholder text colour. `None` → built-in translucent black.
    pub(crate) placeholder_color: Option<Rgba>,
    // ---- Grid ----
    /// Number of equal-width columns (`repeat(N, minmax(0, 1fr))`).
    /// GPUI only supports uniform tracks; arbitrary track lists are unavailable.
    pub(crate) grid_template_columns: Option<u16>,
    /// Number of equal-height rows (`repeat(N, minmax(0, 1fr))`).
    pub(crate) grid_template_rows: Option<u16>,
    /// Column placement: start line or span for a grid item.
    pub(crate) grid_column_start: Option<GridPlacement>,
    /// Column placement: end line or span for a grid item.
    pub(crate) grid_column_end: Option<GridPlacement>,
    /// Row placement: start line or span for a grid item.
    pub(crate) grid_row_start: Option<GridPlacement>,
    /// Row placement: end line or span for a grid item.
    pub(crate) grid_row_end: Option<GridPlacement>,
}

impl StyleFields {
    /// Returns true when either axis requests scrolling.
    ///
    /// `render.rs` uses this to decide whether to force an element onto the
    /// stateful `.id()` path (required by `StatefulInteractiveElement::overflow_*_scroll`).
    pub(crate) fn scrolls(&self) -> bool {
        matches!(self.overflow_x, Some(OverflowMode::Scroll))
            || matches!(self.overflow_y, Some(OverflowMode::Scroll))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- OverflowMode::parse ----

    #[test]
    fn overflow_mode_known_values() {
        assert_eq!(OverflowMode::parse("visible"), Some(OverflowMode::Visible));
        assert_eq!(OverflowMode::parse("hidden"), Some(OverflowMode::Hidden));
        assert_eq!(OverflowMode::parse("scroll"), Some(OverflowMode::Scroll));
    }

    #[test]
    fn overflow_mode_unknown_returns_none() {
        assert!(OverflowMode::parse("auto").is_none());
        assert!(OverflowMode::parse("clip").is_none());
        assert!(OverflowMode::parse("").is_none());
    }

    // ---- parse_window_control_area ----

    #[test]
    fn window_control_area_known_values() {
        assert_eq!(
            parse_window_control_area("drag"),
            Some(WindowControlArea::Drag)
        );
        assert_eq!(
            parse_window_control_area("close"),
            Some(WindowControlArea::Close)
        );
        assert_eq!(
            parse_window_control_area("max"),
            Some(WindowControlArea::Max)
        );
        assert_eq!(
            parse_window_control_area("min"),
            Some(WindowControlArea::Min)
        );
    }

    #[test]
    fn window_control_area_unknown_returns_none() {
        assert!(parse_window_control_area("").is_none());
        assert!(parse_window_control_area("Drag").is_none());
        assert!(parse_window_control_area("maximize").is_none());
        assert!(parse_window_control_area("bogus").is_none());
    }

    // ---- StyleFields::scrolls ----

    #[test]
    fn scrolls_false_by_default() {
        let s = StyleFields::default();
        assert!(!s.scrolls());
    }

    #[test]
    fn scrolls_true_when_overflow_x_scroll() {
        let s = StyleFields {
            overflow_x: Some(OverflowMode::Scroll),
            ..Default::default()
        };
        assert!(s.scrolls());
    }

    #[test]
    fn scrolls_true_when_overflow_y_scroll() {
        let s = StyleFields {
            overflow_y: Some(OverflowMode::Scroll),
            ..Default::default()
        };
        assert!(s.scrolls());
    }

    #[test]
    fn scrolls_false_when_hidden() {
        let s = StyleFields {
            overflow_x: Some(OverflowMode::Hidden),
            overflow_y: Some(OverflowMode::Hidden),
            ..Default::default()
        };
        assert!(!s.scrolls());
    }

    // ---- FloatingSide::parse ----

    #[test]
    fn floating_side_known_values() {
        assert_eq!(FloatingSide::parse("top"), Some(FloatingSide::Top));
        assert_eq!(FloatingSide::parse("bottom"), Some(FloatingSide::Bottom));
        assert_eq!(FloatingSide::parse("left"), Some(FloatingSide::Left));
        assert_eq!(FloatingSide::parse("right"), Some(FloatingSide::Right));
    }

    #[test]
    fn floating_side_unknown_returns_none() {
        assert!(FloatingSide::parse("center").is_none());
        assert!(FloatingSide::parse("Top").is_none());
        assert!(FloatingSide::parse("").is_none());
    }

    // ---- FloatingAlign::parse ----

    #[test]
    fn floating_align_known_values() {
        assert_eq!(FloatingAlign::parse("start"), Some(FloatingAlign::Start));
        assert_eq!(FloatingAlign::parse("center"), Some(FloatingAlign::Center));
        assert_eq!(FloatingAlign::parse("end"), Some(FloatingAlign::End));
    }

    #[test]
    fn floating_align_unknown_returns_none() {
        assert!(FloatingAlign::parse("middle").is_none());
        assert!(FloatingAlign::parse("Start").is_none());
        assert!(FloatingAlign::parse("").is_none());
    }

    // ---- parse_floating_area ----

    #[test]
    fn floating_area_bottom_start() {
        assert_eq!(
            parse_floating_area("bottom start"),
            (FloatingSide::Bottom, FloatingAlign::Start)
        );
    }

    #[test]
    fn floating_area_side_only_defaults_align_to_start() {
        assert_eq!(
            parse_floating_area("bottom"),
            (FloatingSide::Bottom, FloatingAlign::Start)
        );
    }

    #[test]
    fn floating_area_top_end() {
        assert_eq!(
            parse_floating_area("top end"),
            (FloatingSide::Top, FloatingAlign::End)
        );
    }

    #[test]
    fn floating_area_empty_defaults_to_bottom_start() {
        assert_eq!(
            parse_floating_area(""),
            (FloatingSide::Bottom, FloatingAlign::Start)
        );
    }

    #[test]
    fn floating_area_garbage_defaults_to_bottom_start() {
        assert_eq!(
            parse_floating_area("garbage"),
            (FloatingSide::Bottom, FloatingAlign::Start)
        );
    }

    #[test]
    fn floating_area_left_center() {
        assert_eq!(
            parse_floating_area("left center"),
            (FloatingSide::Left, FloatingAlign::Center)
        );
    }

    #[test]
    fn floating_area_unknown_align_falls_back_to_start() {
        // An unrecognized align token → defaults to Start.
        assert_eq!(
            parse_floating_area("left middle"),
            (FloatingSide::Left, FloatingAlign::Start)
        );
    }
}
