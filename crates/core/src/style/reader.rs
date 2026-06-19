use boa_engine::{Context as JsContext, JsObject, JsValue, js_string};
use gpui::{DefiniteLength, GridPlacement, Rgba, px, relative, rems};

use super::color::parse_color;
use super::grid::parse_grid_shorthand;
use super::length::parse_length_str;
use crate::coerce::{JsValueExt, js_array_values};
use crate::style::fields::{BoxShadowSpec, LengthValue, OverflowMode, ShadowValue};

fn get_f32(obj: &JsObject, key: &str, ctx: &mut JsContext) -> Option<f32> {
    obj.get(js_string!(key), ctx).ok().and_then(|v| v.as_f32())
}

fn get_str(obj: &JsObject, key: &str, ctx: &mut JsContext) -> Option<String> {
    obj.get(js_string!(key), ctx).ok().and_then(|v| v.as_str())
}

fn get_bool(obj: &JsObject, key: &str, ctx: &mut JsContext) -> Option<bool> {
    obj.get(js_string!(key), ctx)
        .ok()
        .and_then(|value| value.as_boolean())
}

/// Convert an already-read JS value into a [`LengthValue`].
/// Bare number → `Px(n)`; string → [`parse_length_str`].
pub(crate) fn length_from_value(value: &JsValue) -> Option<LengthValue> {
    if let Some(n) = value.as_f32() {
        return Some(LengthValue::Px(n));
    }
    if let Some(s) = value.as_str() {
        return parse_length_str(&s);
    }
    None
}

/// Number → as-is; `"bold"` → 700; `"normal"` → 400.
fn get_font_weight(obj: &JsObject, ctx: &mut JsContext) -> Option<f32> {
    let val = obj.get(js_string!("fontWeight"), ctx).ok()?;
    if let Some(n) = val.as_number() {
        return Some(n as f32);
    }
    if let Some(s) = val.as_string() {
        return match s.to_std_string().as_deref() {
            Ok("bold") => Some(700.0),
            Ok("normal") => Some(400.0),
            _ => None,
        };
    }
    None
}

/// Parse `lineHeight`: number or bare numeric string → `relative(n)`;
/// `"Npx"` → `px(N)`; `"Nrem"` → `rems(N)`; `"N%"` → `relative(N/100)`.
fn get_line_height(obj: &JsObject, ctx: &mut JsContext) -> Option<DefiniteLength> {
    let val = obj.get(js_string!("lineHeight"), ctx).ok()?;
    if let Some(n) = val.as_number() {
        return Some(relative(n as f32));
    }
    if let Some(s) = val.as_string().and_then(|s| s.to_std_string().ok()) {
        let s = s.trim();
        if let Some(p) = s.strip_suffix("px") {
            return p.trim().parse::<f32>().ok().map(|n| px(n).into());
        }
        if let Some(p) = s.strip_suffix("rem") {
            return p.trim().parse::<f32>().ok().map(|n| rems(n).into());
        }
        if let Some(p) = s.strip_suffix('%') {
            return p.trim().parse::<f32>().ok().map(|n| relative(n / 100.0));
        }
        if let Ok(n) = s.parse::<f32>() {
            return Some(relative(n));
        }
    }
    None
}

/// Parse `fontFeatures`: keys are 4-char OpenType tags; values are booleans or
/// non-negative integers. Invalid tags and unsupported types are silently skipped.
fn get_font_features(obj: &JsObject, ctx: &mut JsContext) -> Option<Vec<(String, u32)>> {
    let val = obj.get(js_string!("fontFeatures"), ctx).ok()?;
    let features_obj = val.as_object()?;

    let keys = features_obj.own_property_keys(ctx).ok()?;
    let mut result = Vec::new();

    for key in keys {
        let boa_engine::property::PropertyKey::String(tag_js) = key else {
            continue;
        };
        let tag = tag_js.to_std_string().unwrap_or_default();
        if tag.len() != 4 || !tag.chars().all(|c| c.is_ascii_alphanumeric()) {
            continue;
        }
        let val = features_obj
            .get(js_string!(tag.as_str()), ctx)
            .unwrap_or_default();
        if let Some(b) = val.as_boolean() {
            result.push((tag, if b { 1 } else { 0 }));
        } else if let Some(n) = val.as_number()
            && n >= 0.0
            && n.fract() == 0.0
        {
            result.push((tag, n as u32));
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Trim surrounding whitespace and a single layer of matching outer quotes
/// (`'…'` or `"…"`) from one `font-family` token.
fn clean_font_token(raw: &str) -> String {
    let t = raw.trim();
    let unquoted = if t.len() >= 2
        && ((t.starts_with('"') && t.ends_with('"'))
            || (t.starts_with('\'') && t.ends_with('\'')))
    {
        &t[1..t.len() - 1]
    } else {
        t
    };
    unquoted.trim().to_string()
}

/// Parse `fontFamily` into an ordered token list (primary first, then fallbacks).
///
/// - **String** — split on commas (CSS `font-family` syntax).
/// - **Array** — one token per element; commas inside an element are *not* split.
///
/// Every token is trimmed and unquoted via [`clean_font_token`]; empty tokens are
/// dropped. An all-empty (or non-string/array) value yields `None`.
fn get_font_family(obj: &JsObject, ctx: &mut JsContext) -> Option<Vec<String>> {
    let val = obj.get(js_string!("fontFamily"), ctx).ok()?;
    let tokens: Vec<String> = if let Some(arr) = val.as_object().filter(|o| o.is_array()) {
        js_array_values(&arr, ctx)
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| clean_font_token(&s))
            .filter(|s| !s.is_empty())
            .collect()
    } else if let Some(s) = val.as_str() {
        s.split(',')
            .map(clean_font_token)
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        return None;
    };
    if tokens.is_empty() {
        None
    } else {
        Some(tokens)
    }
}

/// Returns `(flex_number, flex_keyword)` — at most one is `Some`.
fn get_flex(obj: &JsObject, ctx: &mut JsContext) -> (Option<f32>, Option<String>) {
    let val = match obj.get(js_string!("flex"), ctx).ok() {
        Some(v) => v,
        None => return (None, None),
    };
    if val.is_undefined() || val.is_null() {
        return (None, None);
    }
    if let Some(n) = val.as_number() {
        return (Some(n as f32), None);
    }
    if let Some(s) = val.as_string().and_then(|s| s.to_std_string().ok()) {
        match s.as_str() {
            "auto" | "initial" | "none" => return (None, Some(s)),
            _ => {}
        }
    }
    (None, None)
}

/// Missing fields default to zero offsets/radii and black at 10% opacity.
fn parse_shadow_value(obj: &JsObject, ctx: &mut JsContext) -> ShadowValue {
    let color = get_str(obj, "color", ctx)
        .and_then(|s| parse_color(&s))
        .unwrap_or(Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.1,
        });
    ShadowValue {
        offset_x: get_f32(obj, "offsetX", ctx).unwrap_or(0.0),
        offset_y: get_f32(obj, "offsetY", ctx).unwrap_or(0.0),
        blur_radius: get_f32(obj, "blurRadius", ctx).unwrap_or(0.0),
        spread_radius: get_f32(obj, "spreadRadius", ctx).unwrap_or(0.0),
        color,
        inset: obj
            .get(js_string!("inset"), ctx)
            .ok()
            .and_then(|v| v.as_boolean())
            .unwrap_or(false),
    }
}

/// Parse `boxShadow`: preset string, single layer object, or array of layers.
fn get_box_shadow(obj: &JsObject, ctx: &mut JsContext) -> Option<BoxShadowSpec> {
    let val = obj.get(js_string!("boxShadow"), ctx).ok()?;
    if val.is_undefined() || val.is_null() {
        return None;
    }
    if let Some(s) = val.as_string().and_then(|s| s.to_std_string().ok()) {
        let valid = matches!(
            s.as_str(),
            "none" | "2xs" | "xs" | "sm" | "md" | "lg" | "xl" | "2xl"
        );
        return if valid {
            Some(BoxShadowSpec::Preset(s))
        } else {
            None
        };
    }
    if let Some(obj_val) = val.as_object() {
        if obj_val.is_array() {
            let mut layers = Vec::new();
            for item_val in js_array_values(&obj_val, ctx) {
                if let Some(item) = item_val.as_object() {
                    layers.push(parse_shadow_value(&item, ctx));
                }
            }
            if layers.is_empty() {
                return None;
            }
            return Some(BoxShadowSpec::Custom(layers));
        }
        return Some(BoxShadowSpec::Custom(vec![parse_shadow_value(
            &obj_val, ctx,
        )]));
    }
    None
}

/// `overflow` sets both axes; `overflowX`/`overflowY` override per axis.
fn parse_overflow(
    obj: &JsObject,
    ctx: &mut JsContext,
) -> (Option<OverflowMode>, Option<OverflowMode>) {
    let both = get_str(obj, "overflow", ctx).and_then(|s| OverflowMode::parse(&s));
    let x = get_str(obj, "overflowX", ctx)
        .and_then(|s| OverflowMode::parse(&s))
        .or(both);
    let y = get_str(obj, "overflowY", ctx)
        .and_then(|s| OverflowMode::parse(&s))
        .or(both);
    (x, y)
}

/// Priority: shorthand → span (overrides both endpoints) → start → end.
fn parse_grid_axis(
    obj: &JsObject,
    shorthand_key: &str,
    start_key: &str,
    end_key: &str,
    span_key: &str,
    ctx: &mut JsContext,
) -> (Option<GridPlacement>, Option<GridPlacement>) {
    let mut start: Option<GridPlacement> = None;
    let mut end: Option<GridPlacement> = None;

    // Shorthand: number form takes priority over string form.
    if let Some(n) = get_f32(obj, shorthand_key, ctx) {
        start = Some(GridPlacement::Line(n as i16));
    } else if let Some(s) = get_str(obj, shorthand_key, ctx) {
        let (sh_start, sh_end) = parse_grid_shorthand(&s);
        start = sh_start;
        end = sh_end;
    }

    // *Span overrides both endpoints.
    if let Some(n) = get_f32(obj, span_key, ctx) {
        let n = (n as u16).max(1);
        start = Some(GridPlacement::Span(n));
        end = Some(GridPlacement::Span(n));
    }

    if let Some(n) = get_f32(obj, start_key, ctx) {
        start = Some(GridPlacement::Line(n as i16));
    }

    if let Some(n) = get_f32(obj, end_key, ctx) {
        end = Some(GridPlacement::Line(n as i16));
    }

    (start, end)
}

// ---------------------------------------------------------------------------
// PropReader — ordered-list lookup abstraction
// ---------------------------------------------------------------------------

/// Reads style fields from an ordered list of JS objects; the first object
/// that yields a value wins. `[style_obj, obj]` for top-level props;
/// `[sub_obj]` for `_hover`/`_active`.
pub(crate) struct PropReader<'a> {
    objs: Vec<&'a JsObject>,
}

impl<'a> PropReader<'a> {
    pub(crate) fn new(objs: Vec<&'a JsObject>) -> Self {
        Self { objs }
    }

    fn first<T>(
        &self,
        ctx: &mut JsContext,
        f: impl Fn(&JsObject, &mut JsContext) -> Option<T>,
    ) -> Option<T> {
        self.objs.iter().find_map(|obj| f(obj, ctx))
    }

    /// The first object where either component is `Some` wins for both.
    fn first_pair<A, B>(
        &self,
        ctx: &mut JsContext,
        f: impl Fn(&JsObject, &mut JsContext) -> (Option<A>, Option<B>),
    ) -> (Option<A>, Option<B>) {
        for obj in &self.objs {
            let (a, b) = f(obj, ctx);
            if a.is_some() || b.is_some() {
                return (a, b);
            }
        }
        (None, None)
    }

    pub(crate) fn str_val(&self, key: &str, ctx: &mut JsContext) -> Option<String> {
        self.first(ctx, |o, c| get_str(o, key, c))
    }

    pub(crate) fn bool_val(&self, key: &str, ctx: &mut JsContext) -> Option<bool> {
        self.first(ctx, |o, c| get_bool(o, key, c))
    }

    /// Reads a numeric prop, truncating to `i32` (e.g. `tabIndex`).
    pub(crate) fn i32_val(&self, key: &str, ctx: &mut JsContext) -> Option<i32> {
        self.first(ctx, |o, c| get_f32(o, key, c)).map(|n| n as i32)
    }

    pub(crate) fn font_weight(&self, ctx: &mut JsContext) -> Option<f32> {
        self.first(ctx, get_font_weight)
    }

    pub(crate) fn line_height(&self, ctx: &mut JsContext) -> Option<DefiniteLength> {
        self.first(ctx, get_line_height)
    }

    pub(crate) fn font_features(&self, ctx: &mut JsContext) -> Option<Vec<(String, u32)>> {
        self.first(ctx, get_font_features)
    }

    pub(crate) fn font_family(&self, ctx: &mut JsContext) -> Option<Vec<String>> {
        self.first(ctx, get_font_family)
    }

    pub(crate) fn flex(&self, ctx: &mut JsContext) -> (Option<f32>, Option<String>) {
        self.first_pair(ctx, get_flex)
    }

    pub(crate) fn overflow(
        &self,
        ctx: &mut JsContext,
    ) -> (Option<OverflowMode>, Option<OverflowMode>) {
        self.first_pair(ctx, parse_overflow)
    }

    pub(crate) fn grid_axis(
        &self,
        shorthand_key: &str,
        start_key: &str,
        end_key: &str,
        span_key: &str,
        ctx: &mut JsContext,
    ) -> (Option<GridPlacement>, Option<GridPlacement>) {
        self.first_pair(ctx, |o, c| {
            parse_grid_axis(o, shorthand_key, start_key, end_key, span_key, c)
        })
    }

    pub(crate) fn box_shadow(&self, ctx: &mut JsContext) -> Option<BoxShadowSpec> {
        self.first(ctx, get_box_shadow)
    }
}
