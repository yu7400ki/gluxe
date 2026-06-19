use boa_engine::{Context as JsContext, JsObject, JsValue, js_string, property::PropertyKey};

use super::reader::{PropReader, length_from_value};
use crate::anim::{Easing, TransitionProperty, TransitionSpec, field_id_from_name};
use crate::coerce::{JsValueExt, js_array_values};
use crate::model::Props;
use crate::style::fields::{
    FloatingSpec, LengthValue, StyleFields, parse_floating_area, parse_window_control_area,
};

// ---------------------------------------------------------------------------
// Composite-field presence tracking
// ---------------------------------------------------------------------------

/// Tracks which composite style fields appeared in any source object.
///
/// Composite fields have multi-key precedence rules that must be resolved
/// independently of JS key-iteration order (e.g. `overflowX` always wins over
/// `overflow`). We flag their presence here and resolve them afterwards via the
/// order-correct [`PropReader`] getters, so absent composites cost nothing.
#[derive(Default)]
struct CompositeSeen {
    flex: bool,
    overflow: bool,
    grid_col: bool,
    grid_row: bool,
    box_shadow: bool,
    font_weight: bool,
    line_height: bool,
    font_features: bool,
}

/// Records composite-field presence; returns `true` so the caller can skip the simple-field path.
fn mark_composite(seen: &mut CompositeSeen, key: &str) -> bool {
    match key {
        "flex" => seen.flex = true,
        "overflow" | "overflowX" | "overflowY" => seen.overflow = true,
        "gridColumn" | "gridColumnStart" | "gridColumnEnd" | "gridColumnSpan" => {
            seen.grid_col = true
        }
        "gridRow" | "gridRowStart" | "gridRowEnd" | "gridRowSpan" => seen.grid_row = true,
        "boxShadow" => seen.box_shadow = true,
        "fontWeight" => seen.font_weight = true,
        "lineHeight" => seen.line_height = true,
        "fontFeatures" => seen.font_features = true,
        _ => return false,
    }
    true
}

// ---------------------------------------------------------------------------
// Simple (1:1 key → field) assignment
// ---------------------------------------------------------------------------

/// Table of simple (1:1 key → field) style props: `(rust_field, "jsName", Kind)`.
/// Generates [`apply_simple_field`]. `Kind` picks the JS→Rust converter, whose
/// return type must match the `StyleFields` field type — a `Kind`/field mismatch
/// is a compile error, so the table can never silently mis-convert a prop. (The
/// animatable subset is declared separately in `anim/fields.rs`; merging the two
/// tables behind a shared `animatable` flag is a possible future consolidation.)
macro_rules! simple_style_fields {
    (@convert Length, $value:expr) => { length_from_value($value) };
    (@convert F32, $value:expr) => { $value.as_f32() };
    (@convert Str, $value:expr) => { $value.as_str() };
    (@convert Color, $value:expr) => { $value.as_color() };
    // Grid track counts: number → u16, min 1.
    (@convert GridTrack, $value:expr) => { $value.as_f32().map(|n| (n as u16).max(1)) };
    ($(($field:ident, $name:literal, $kind:ident)),* $(,)?) => {
        /// Assign a single non-composite style field from an already-fetched JS `value`.
        /// Only assigns when conversion yields `Some`, so an unparseable high-priority value
        /// does not clobber a valid low-priority one. Generated from the field table below.
        fn apply_simple_field(fields: &mut StyleFields, key: &str, value: &JsValue) {
            match key {
                $(
                    $name => {
                        if let Some(v) = simple_style_fields!(@convert $kind, value) {
                            fields.$field = Some(v);
                        }
                    }
                )*
                // Unknown / non-style / composite keys: ignored here.
                _ => {}
            }
        }
    };
}

simple_style_fields![
    // ---- Lengths ----
    (width, "width", Length),
    (height, "height", Length),
    (flex_basis, "flexBasis", Length),
    (padding, "padding", Length),
    (padding_x, "paddingX", Length),
    (padding_y, "paddingY", Length),
    (padding_top, "paddingTop", Length),
    (padding_right, "paddingRight", Length),
    (padding_bottom, "paddingBottom", Length),
    (padding_left, "paddingLeft", Length),
    (gap, "gap", Length),
    (gap_x, "gapX", Length),
    (gap_y, "gapY", Length),
    (border_radius, "borderRadius", Length),
    (border_width, "borderWidth", Length),
    (font_size, "fontSize", Length),
    (margin, "margin", Length),
    (margin_x, "marginX", Length),
    (margin_y, "marginY", Length),
    (margin_top, "marginTop", Length),
    (margin_right, "marginRight", Length),
    (margin_bottom, "marginBottom", Length),
    (margin_left, "marginLeft", Length),
    (min_width, "minWidth", Length),
    (min_height, "minHeight", Length),
    (max_width, "maxWidth", Length),
    (max_height, "maxHeight", Length),
    (inset, "inset", Length),
    (top, "top", Length),
    (right, "right", Length),
    (bottom, "bottom", Length),
    (left, "left", Length),
    (border_top_width, "borderTopWidth", Length),
    (border_right_width, "borderRightWidth", Length),
    (border_bottom_width, "borderBottomWidth", Length),
    (border_left_width, "borderLeftWidth", Length),
    (border_top_left_radius, "borderTopLeftRadius", Length),
    (border_top_right_radius, "borderTopRightRadius", Length),
    (
        border_bottom_right_radius,
        "borderBottomRightRadius",
        Length
    ),
    (border_bottom_left_radius, "borderBottomLeftRadius", Length),
    (scrollbar_width, "scrollbarWidth", Length),
    (text_decoration_thickness, "textDecorationThickness", Length),
    (caret_width, "caretWidth", Length),
    // ---- Numbers ----
    (flex_grow, "flexGrow", F32),
    (flex_shrink, "flexShrink", F32),
    (line_clamp, "lineClamp", F32),
    (aspect_ratio, "aspectRatio", F32),
    (opacity, "opacity", F32),
    // ---- Grid track counts (number → u16, min 1) ----
    (grid_template_columns, "gridTemplateColumns", GridTrack),
    (grid_template_rows, "gridTemplateRows", GridTrack),
    // ---- Strings ----
    (display, "display", Str),
    (flex_direction, "flexDirection", Str),
    (align_items, "alignItems", Str),
    (justify_content, "justifyContent", Str),
    (flex_wrap, "flexWrap", Str),
    (align_self, "alignSelf", Str),
    (align_content, "alignContent", Str),
    (cursor, "cursor", Str),
    (white_space, "whiteSpace", Str),
    (text_overflow, "textOverflow", Str),
    (position, "position", Str),
    (visibility, "visibility", Str),
    (border_style, "borderStyle", Str),
    (text_align, "textAlign", Str),
    (font_style, "fontStyle", Str),
    (font_family, "fontFamily", Str),
    (text_decoration_line, "textDecorationLine", Str),
    (text_decoration_style, "textDecorationStyle", Str),
    // ---- Colors ----
    (background_color, "backgroundColor", Color),
    (color, "color", Color),
    (border_color, "borderColor", Color),
    (text_decoration_color, "textDecorationColor", Color),
    (text_background_color, "textBackgroundColor", Color),
    (caret_color, "caretColor", Color),
    (selection_color, "selectionColor", Color),
    (placeholder_color, "placeholderColor", Color),
];

/// Walk one source object's own string keys, applying simple fields and recording
/// composite-field presence. Array-index and symbol keys are skipped.
fn apply_object_keys(
    fields: &mut StyleFields,
    seen: &mut CompositeSeen,
    obj: &JsObject,
    ctx: &mut JsContext,
) {
    let Ok(keys) = obj.own_property_keys(ctx) else {
        return;
    };
    for key in keys {
        let PropertyKey::String(ref name_js) = key else {
            continue;
        };
        let Ok(name) = name_js.to_std_string() else {
            continue;
        };
        if mark_composite(seen, &name) {
            continue;
        }
        let Ok(value) = obj.get(key, ctx) else {
            continue;
        };
        apply_simple_field(fields, &name, &value);
    }
}

// ---------------------------------------------------------------------------
// StyleFields assembly
// ---------------------------------------------------------------------------

/// Build a [`StyleFields`] from an ordered list of source objects (highest-priority first).
///
/// Simple fields are applied in reverse (lowest → highest priority) so later writes win.
/// Composite fields are resolved afterwards via [`PropReader`], which also uses
/// highest-priority-first order.
fn parse_style_fields(objs: &[&JsObject], ctx: &mut JsContext) -> StyleFields {
    let mut fields = StyleFields::default();
    let mut seen = CompositeSeen::default();

    for obj in objs.iter().rev() {
        apply_object_keys(&mut fields, &mut seen, obj, ctx);
    }

    // Resolve composite fields only when a relevant key appeared.
    if seen.flex
        || seen.overflow
        || seen.grid_col
        || seen.grid_row
        || seen.box_shadow
        || seen.font_weight
        || seen.line_height
        || seen.font_features
    {
        let r = PropReader::new(objs.to_vec());
        if seen.flex {
            let (flex_num, flex_kw) = r.flex(ctx);
            fields.flex = flex_num;
            fields.flex_keyword = flex_kw;
        }
        if seen.overflow {
            let (x, y) = r.overflow(ctx);
            fields.overflow_x = x;
            fields.overflow_y = y;
        }
        if seen.grid_col {
            let (start, end) = r.grid_axis(
                "gridColumn",
                "gridColumnStart",
                "gridColumnEnd",
                "gridColumnSpan",
                ctx,
            );
            fields.grid_column_start = start;
            fields.grid_column_end = end;
        }
        if seen.grid_row {
            let (start, end) =
                r.grid_axis("gridRow", "gridRowStart", "gridRowEnd", "gridRowSpan", ctx);
            fields.grid_row_start = start;
            fields.grid_row_end = end;
        }
        if seen.box_shadow {
            fields.box_shadow = r.box_shadow(ctx);
        }
        if seen.font_weight {
            fields.font_weight = r.font_weight(ctx);
        }
        if seen.line_height {
            fields.line_height = r.line_height(ctx);
        }
        if seen.font_features {
            fields.font_features = r.font_features(ctx);
        }
    }

    fields
}

// ---------------------------------------------------------------------------
// Transition parsing
// ---------------------------------------------------------------------------

/// Parse one `{ property, duration, easing?, delay? }` declaration.
/// Returns `None` when `property` is unknown or non-animatable (spec is dropped).
fn parse_transition_item(obj: &JsObject, ctx: &mut JsContext) -> Option<TransitionSpec> {
    let property = match obj.get(js_string!("property"), ctx).ok() {
        Some(v) if !v.is_undefined() => {
            let s = v.as_str()?;
            if s == "all" {
                TransitionProperty::All
            } else {
                TransitionProperty::Field(field_id_from_name(&s)?)
            }
        }
        _ => TransitionProperty::All,
    };
    let duration_ms = obj
        .get(js_string!("duration"), ctx)
        .ok()
        .and_then(|v| v.as_f32())
        .map(|n| n.max(0.0))
        .unwrap_or(0.0);
    let delay_ms = obj
        .get(js_string!("delay"), ctx)
        .ok()
        .and_then(|v| v.as_f32())
        .map(|n| n.max(0.0))
        .unwrap_or(0.0);
    let easing = obj
        .get(js_string!("easing"), ctx)
        .ok()
        .and_then(|v| v.as_str())
        .and_then(|s| Easing::parse(&s))
        .unwrap_or(Easing::Ease);
    Some(TransitionSpec {
        property,
        duration_ms,
        delay_ms,
        easing,
    })
}

/// Parse the `transition` style key (single object or array). Unparseable → empty list.
fn parse_transitions(value: &JsValue, ctx: &mut JsContext) -> Vec<TransitionSpec> {
    let Some(obj) = value.as_object() else {
        return Vec::new();
    };
    if !obj.is_array() {
        return parse_transition_item(&obj, ctx).into_iter().collect();
    }
    let mut specs = Vec::new();
    for item in js_array_values(&obj, ctx) {
        if let Some(item_obj) = item.as_object() {
            specs.extend(parse_transition_item(&item_obj, ctx));
        }
    }
    specs
}

// ---------------------------------------------------------------------------
// Floating prop parsing
// ---------------------------------------------------------------------------

/// Parse the top-level `floating` prop object into a [`FloatingSpec`].
/// Returns `None` when the value is not an object or when the required `anchor`
/// sub-key is absent or not a string.
fn parse_floating(obj: &JsObject, ctx: &mut JsContext) -> Option<FloatingSpec> {
    let floating_val = obj.get(js_string!("floating"), ctx).ok()?;
    let floating_obj = floating_val.as_object()?;

    let anchor = floating_obj.get(js_string!("anchor"), ctx).ok()?.as_str()?;

    let (side, align) = floating_obj
        .get(js_string!("area"), ctx)
        .ok()
        .and_then(|v| v.as_str())
        .map(|s| parse_floating_area(&s))
        .unwrap_or_else(|| parse_floating_area(""));

    let offset = floating_obj
        .get(js_string!("offset"), ctx)
        .ok()
        .and_then(|v| length_from_value(&v))
        .unwrap_or(LengthValue::Px(0.0));

    let margin = floating_obj
        .get(js_string!("margin"), ctx)
        .ok()
        .and_then(|v| length_from_value(&v))
        .unwrap_or(LengthValue::Px(0.0));

    let priority = floating_obj
        .get(js_string!("priority"), ctx)
        .ok()
        .and_then(|v| v.as_f32())
        .map(|n| n as u16);

    Some(FloatingSpec {
        anchor,
        side,
        align,
        offset,
        margin,
        priority,
    })
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Parse a nested pseudo-selector style block (`_hover` / `_active` / `_focus` /
/// `_focusVisible`) from the `style` object, when `key` is present and an object.
fn nested_style(
    style_obj: Option<&JsObject>,
    key: &str,
    ctx: &mut JsContext,
) -> Option<Box<StyleFields>> {
    let obj = style_obj?.get(js_string!(key), ctx).ok()?.as_object()?;
    Some(Box::new(parse_style_fields(&[&obj], ctx)))
}

pub(crate) fn parse_props(obj: &JsObject, ctx: &mut JsContext) -> Props {
    // `style` sub-object takes precedence over top-level prop keys.
    let style_obj: Option<JsObject> = obj
        .get(js_string!("style"), ctx)
        .ok()
        .and_then(|value| value.as_object());

    let mut objs: Vec<&JsObject> = Vec::with_capacity(2);
    if let Some(s) = style_obj.as_ref() {
        objs.push(s);
    }
    objs.push(obj);
    let style = parse_style_fields(&objs, ctx);

    // Pseudo-selector overrides are parsed from nested objects inside `style` only.
    let hover = nested_style(style_obj.as_ref(), "_hover", ctx);
    let active = nested_style(style_obj.as_ref(), "_active", ctx);
    let focus_style = nested_style(style_obj.as_ref(), "_focus", ctx);
    let focus_visible_style = nested_style(style_obj.as_ref(), "_focusVisible", ctx);

    // Transitions come from `style` only (never top-level, never `_hover`/`_active`).
    let transitions = style_obj
        .as_ref()
        .and_then(|s| s.get(js_string!("transition"), ctx).ok())
        .map(|v| parse_transitions(&v, ctx))
        .unwrap_or_default();

    // Non-style props are read from the top-level object only.
    let obj_reader = PropReader::new(vec![obj]);
    Props {
        hover,
        active,
        focus_style,
        focus_visible_style,
        tab_index: obj_reader.i32_val("tabIndex", ctx),
        tab_stop: obj_reader.bool_val("tabStop", ctx),
        style,
        transitions,
        src: obj_reader.str_val("src", ctx),
        value: obj_reader.str_val("value", ctx),
        placeholder: obj_reader.str_val("placeholder", ctx),
        multiline: obj_reader.bool_val("multiline", ctx).unwrap_or(false),
        // Rows must be positive; non-positive values are ignored (fall back to defaults).
        min_rows: obj_reader
            .i32_val("minRows", ctx)
            .filter(|n| *n >= 1)
            .map(|n| n as u32),
        max_rows: obj_reader
            .i32_val("maxRows", ctx)
            .filter(|n| *n >= 1)
            .map(|n| n as u32),
        autofocus: obj_reader.bool_val("autoFocus", ctx).unwrap_or(false),
        window_control_area: obj_reader
            .str_val("windowControlArea", ctx)
            .as_deref()
            .and_then(parse_window_control_area),
        anchor_name: obj_reader.str_val("anchorName", ctx),
        floating: parse_floating(obj, ctx),
        occlude: obj_reader.bool_val("occlude", ctx),
        // Events are populated separately from the third bridge argument.
        ..Props::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::fields::{
        FloatingAlign, FloatingSide, FloatingSpec, LengthValue, OverflowMode,
    };

    /// Evaluate a JS object-literal expression and run it through `parse_props`.
    fn props_from_js(src: &str) -> Props {
        let mut ctx = JsContext::builder().build().expect("ctx");
        let value = crate::stack::eval_on_parser_stack(&mut ctx, src.as_bytes()).expect("eval");
        let obj = value.as_object().expect("object literal");
        parse_props(&obj, &mut ctx)
    }

    #[test]
    fn style_overrides_top_level() {
        let p = props_from_js("({ width: 5, style: { width: 10 } })");
        assert_eq!(p.style.width, Some(LengthValue::Px(10.0)));
    }

    #[test]
    fn multiline_props_parse() {
        let p = props_from_js("({ multiline: true, minRows: 3, maxRows: 8 })");
        assert!(p.multiline);
        assert_eq!(p.min_rows, Some(3));
        assert_eq!(p.max_rows, Some(8));
    }

    #[test]
    fn multiline_defaults_and_nonpositive_rows_ignored() {
        // multiline defaults to false; rows < 1 fall back to the engine defaults.
        let p = props_from_js("({ minRows: 0, maxRows: -2 })");
        assert!(!p.multiline);
        assert_eq!(p.min_rows, None);
        assert_eq!(p.max_rows, None);
    }

    #[test]
    fn top_level_used_when_style_absent() {
        let p = props_from_js("({ width: 5, style: { height: 20 } })");
        assert_eq!(p.style.width, Some(LengthValue::Px(5.0)));
        assert_eq!(p.style.height, Some(LengthValue::Px(20.0)));
    }

    #[test]
    fn unparseable_high_priority_falls_back_to_low() {
        // style.width (boolean) is not a valid length → keep the top-level value.
        let p = props_from_js("({ width: 5, style: { width: true } })");
        assert_eq!(p.style.width, Some(LengthValue::Px(5.0)));
    }

    #[test]
    fn overflow_x_overrides_overflow_independent_of_key_order() {
        // overflowX must win over overflow for the x axis regardless of which key
        // appears first in the object literal.
        let a = props_from_js("({ style: { overflowX: 'scroll', overflow: 'hidden' } })");
        assert_eq!(a.style.overflow_x, Some(OverflowMode::Scroll));
        assert_eq!(a.style.overflow_y, Some(OverflowMode::Hidden));
        let b = props_from_js("({ style: { overflow: 'hidden', overflowX: 'scroll' } })");
        assert_eq!(b.style.overflow_x, Some(OverflowMode::Scroll));
        assert_eq!(b.style.overflow_y, Some(OverflowMode::Hidden));
    }

    #[test]
    fn parses_strings_colors_and_flex() {
        let p =
            props_from_js("({ style: { display: 'flex', backgroundColor: '#ff0000', flex: 1 } })");
        assert_eq!(p.style.display.as_deref(), Some("flex"));
        assert!(p.style.background_color.is_some());
        assert_eq!(p.style.flex, Some(1.0));
    }

    #[test]
    fn simple_field_kinds_convert_correctly() {
        // One representative per `simple_style_fields!` Kind. The converter return
        // type already pins the field type at compile time; this guards the value
        // shape (and the GridTrack `(n as u16).max(1)` quirk) against a table edit.
        let p = props_from_js(
            "({ style: { width: '2rem', opacity: 0.5, color: '#00ff00', display: 'flex', gridTemplateColumns: 3 } })",
        );
        assert_eq!(p.style.width, Some(LengthValue::Rem(2.0))); // Length
        assert_eq!(p.style.opacity, Some(0.5)); // F32
        assert!(p.style.color.is_some()); // Color
        assert_eq!(p.style.display.as_deref(), Some("flex")); // Str
        assert_eq!(p.style.grid_template_columns, Some(3)); // GridTrack
    }

    #[test]
    fn grid_track_count_clamps_to_min_one() {
        // GridTrack converter floors at 1 even when JS passes 0/negative.
        let p = props_from_js("({ style: { gridTemplateColumns: 0, gridTemplateRows: -4 } })");
        assert_eq!(p.style.grid_template_columns, Some(1));
        assert_eq!(p.style.grid_template_rows, Some(1));
    }

    #[test]
    fn transition_single_object() {
        let p = props_from_js(
            "({ style: { transition: { property: 'width', duration: 300, easing: 'ease-out', delay: 50 } } })",
        );
        assert_eq!(p.transitions.len(), 1);
        let t = &p.transitions[0];
        assert_eq!(
            t.property,
            TransitionProperty::Field(crate::anim::FieldId::width)
        );
        assert_eq!(t.duration_ms, 300.0);
        assert_eq!(t.delay_ms, 50.0);
        assert_eq!(t.easing, Easing::EaseOut);
    }

    #[test]
    fn transition_array_and_defaults() {
        let p = props_from_js(
            "({ style: { transition: [ { property: 'all', duration: 200 }, { duration: 100, easing: 'linear' } ] } })",
        );
        assert_eq!(p.transitions.len(), 2);
        assert_eq!(p.transitions[0].property, TransitionProperty::All);
        assert_eq!(p.transitions[0].easing, Easing::Ease); // default
        assert_eq!(p.transitions[0].delay_ms, 0.0); // default
        assert_eq!(p.transitions[1].property, TransitionProperty::All); // property defaults to all
        assert_eq!(p.transitions[1].easing, Easing::Linear);
    }

    #[test]
    fn transition_unknown_property_drops_spec() {
        let p = props_from_js(
            "({ style: { transition: [ { property: 'display', duration: 100 }, { property: 'opacity', duration: 100 } ] } })",
        );
        assert_eq!(p.transitions.len(), 1);
        assert_eq!(
            p.transitions[0].property,
            TransitionProperty::Field(crate::anim::FieldId::opacity)
        );
    }

    #[test]
    fn transition_unknown_easing_falls_back_to_ease() {
        let p = props_from_js("({ style: { transition: { duration: 100, easing: 'bounce' } } })");
        assert_eq!(p.transitions[0].easing, Easing::Ease);
    }

    #[test]
    fn transition_invalid_or_absent_is_empty() {
        assert!(props_from_js("({ style: {} })").transitions.is_empty());
        assert!(
            props_from_js("({ style: { transition: 'all 200ms' } })")
                .transitions
                .is_empty()
        );
        // Negative values clamp to zero.
        let p = props_from_js("({ style: { transition: { duration: -5, delay: -3 } } })");
        assert_eq!(p.transitions[0].duration_ms, 0.0);
        assert_eq!(p.transitions[0].delay_ms, 0.0);
    }

    #[test]
    fn pseudo_selectors_parse_independently() {
        let p = props_from_js("({ style: { _hover: { width: 7 }, _active: { width: 9 } } })");
        assert_eq!(
            p.hover.as_ref().map(|h| h.width),
            Some(Some(LengthValue::Px(7.0)))
        );
        assert_eq!(
            p.active.as_ref().map(|a| a.width),
            Some(Some(LengthValue::Px(9.0)))
        );
    }

    #[test]
    fn focus_pseudo_selectors_parse_independently() {
        let p = props_from_js("({ style: { _focus: { width: 3 }, _focusVisible: { width: 5 } } })");
        assert_eq!(
            p.focus_style.as_ref().map(|f| f.width),
            Some(Some(LengthValue::Px(3.0)))
        );
        assert_eq!(
            p.focus_visible_style.as_ref().map(|f| f.width),
            Some(Some(LengthValue::Px(5.0)))
        );
        assert!(p.is_focusable());
    }

    // ---- tabIndex / tabStop props ----

    #[test]
    fn tab_index_parses_and_implies_focusable() {
        let p = props_from_js("({ tabIndex: 2 })");
        assert_eq!(p.tab_index, Some(2));
        assert!(p.is_focusable());
    }

    #[test]
    fn negative_tab_index_parses() {
        let p = props_from_js("({ tabIndex: -1 })");
        assert_eq!(p.tab_index, Some(-1));
        assert!(p.is_focusable());
    }

    #[test]
    fn tab_stop_parses() {
        let p = props_from_js("({ tabIndex: 0, tabStop: false })");
        assert_eq!(p.tab_stop, Some(false));
    }

    #[test]
    fn no_focus_props_is_not_focusable() {
        let p = props_from_js("({ style: { width: 10 } })");
        assert!(!p.is_focusable());
    }

    // ---- occlude prop ----

    #[test]
    fn occlude_absent_is_none_and_follows_overlay_default() {
        let p = props_from_js("({ style: { width: 10 } })");
        assert_eq!(p.occlude, None);
        assert!(!p.should_occlude());
    }

    #[test]
    fn occlude_true_forces_occlusion_on_in_flow_node() {
        let p = props_from_js("({ occlude: true })");
        assert_eq!(p.occlude, Some(true));
        assert!(p.should_occlude());
    }

    #[test]
    fn occlude_false_opts_out_of_overlay_occlusion() {
        let p = props_from_js("({ occlude: false, style: { position: 'absolute' } })");
        assert_eq!(p.occlude, Some(false));
        assert!(!p.should_occlude());
    }

    // ---- windowControlArea prop ----

    #[test]
    fn window_control_area_drag() {
        use gpui::WindowControlArea;
        let p = props_from_js("({ windowControlArea: 'drag' })");
        assert_eq!(p.window_control_area, Some(WindowControlArea::Drag));
    }

    #[test]
    fn window_control_area_close() {
        use gpui::WindowControlArea;
        let p = props_from_js("({ windowControlArea: 'close' })");
        assert_eq!(p.window_control_area, Some(WindowControlArea::Close));
    }

    #[test]
    fn window_control_area_max() {
        use gpui::WindowControlArea;
        let p = props_from_js("({ windowControlArea: 'max' })");
        assert_eq!(p.window_control_area, Some(WindowControlArea::Max));
    }

    #[test]
    fn window_control_area_min() {
        use gpui::WindowControlArea;
        let p = props_from_js("({ windowControlArea: 'min' })");
        assert_eq!(p.window_control_area, Some(WindowControlArea::Min));
    }

    #[test]
    fn window_control_area_bogus_gives_none() {
        let p = props_from_js("({ windowControlArea: 'bogus' })");
        assert!(p.window_control_area.is_none());
    }

    #[test]
    fn window_control_area_number_gives_none() {
        let p = props_from_js("({ windowControlArea: 42 })");
        assert!(p.window_control_area.is_none());
    }

    #[test]
    fn window_control_area_absent_gives_none() {
        let p = props_from_js("({ width: 10 })");
        assert!(p.window_control_area.is_none());
    }

    // ---- anchorName prop ----

    #[test]
    fn anchor_name_parsed_from_top_level() {
        let p = props_from_js("({ anchorName: 'trigger-1' })");
        assert_eq!(p.anchor_name, Some("trigger-1".to_string()));
        assert!(p.floating.is_none());
    }

    // ---- floating prop ----

    #[test]
    fn floating_all_fields_parsed() {
        let p = props_from_js(
            "({ floating: { anchor: 'a', area: 'top end', offset: 8, margin: 12, priority: 3 } })",
        );
        assert_eq!(
            p.floating,
            Some(FloatingSpec {
                anchor: "a".to_string(),
                side: FloatingSide::Top,
                align: FloatingAlign::End,
                offset: LengthValue::Px(8.0),
                margin: LengthValue::Px(12.0),
                priority: Some(3),
            })
        );
    }

    #[test]
    fn floating_offset_margin_accept_units() {
        // Strings carry units; `%`/`auto` parse but are ignored at render time.
        let p = props_from_js("({ floating: { anchor: 'a', offset: '0.5rem', margin: '8px' } })");
        let f = p.floating.unwrap();
        assert_eq!(f.offset, LengthValue::Rem(0.5));
        assert_eq!(f.margin, LengthValue::Px(8.0));
    }

    #[test]
    fn floating_center_align_parsed() {
        let p = props_from_js("({ floating: { anchor: 'a', area: 'right center' } })");
        let f = p.floating.unwrap();
        assert_eq!(f.side, FloatingSide::Right);
        assert_eq!(f.align, FloatingAlign::Center);
    }

    #[test]
    fn floating_defaults_when_only_anchor_given() {
        let p = props_from_js("({ floating: { anchor: 'a' } })");
        assert_eq!(
            p.floating,
            Some(FloatingSpec {
                anchor: "a".to_string(),
                side: FloatingSide::Bottom,
                align: FloatingAlign::Start,
                offset: LengthValue::Px(0.0),
                margin: LengthValue::Px(0.0),
                priority: None,
            })
        );
    }

    #[test]
    fn floating_without_anchor_gives_none() {
        // No `anchor` key → the whole floating spec is dropped.
        let p = props_from_js("({ floating: { area: 'bottom' } })");
        assert!(p.floating.is_none());
    }

    #[test]
    fn floating_area_bottom_only_gives_bottom_start() {
        let p = props_from_js("({ floating: { anchor: 'a', area: 'bottom' } })");
        let f = p.floating.unwrap();
        assert_eq!(f.side, FloatingSide::Bottom);
        assert_eq!(f.align, FloatingAlign::Start);
    }

    /// Fixture is generated from `packages/react/js/style-prop-samples.ts` via
    /// `pnpm -C packages/react test:run -- -u`. Catches TS props silently ignored
    /// by the Rust parser; the reverse (dead Rust arm) is not detected here.
    #[test]
    fn every_ts_style_prop_is_recognized() {
        let fixture = include_str!("../../tests/fixtures/style_prop_samples.json");
        let samples: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(fixture).expect("valid fixture JSON");
        assert!(!samples.is_empty(), "fixture must not be empty");
        let baseline = props_from_js("({})");
        for (key, value) in &samples {
            let src = format!("({{ style: {{ {key:?}: {value} }} }})");
            let parsed = props_from_js(&src);
            assert_ne!(
                parsed, baseline,
                "TS style key '{key}' (sample {value}) was not recognized by the \
                 Rust parser — add it to apply_simple_field / the composite keys \
                 in style/parse.rs, or fix the sample in style-prop-samples.ts"
            );
        }
    }
}
