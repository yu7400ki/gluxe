// JS↔Rust value coercion — the single home for `&JsValue` → Rust-scalar
// conversions, the JS-array walk, and the recursive `js_to_json` bridge.
//
// The style reader/parser and the bridge each used to re-derive these
// (`get_f32`/`str_of`/inline `as_number`…); they now share this module so the
// JS→Rust narrowing rules live in exactly one place.

use boa_engine::{Context as JsContext, JsObject, JsValue, js_string, property::PropertyKey};
use gpui::Rgba;

use crate::style::parse_color;

/// Coercion conveniences on a `&JsValue`.
///
/// These mirror boa's own `as_number` / `as_string` (and complement
/// `as_boolean`, which already returns `Option<bool>` and needs no wrapper), but
/// land in the Rust types the rest of the crate wants — `f32`, an owned
/// `String`, an `Rgba` — so call sites read uniformly as `value.as_f32()` etc.
pub(crate) trait JsValueExt {
    /// Number → `f32` (lossy `f64`→`f32` narrowing).
    fn as_f32(&self) -> Option<f32>;
    /// String → owned UTF-8 `String`; a lossy-decode failure drops the value.
    fn as_str(&self) -> Option<String>;
    /// CSS color string → `Rgba`, via [`parse_color`].
    fn as_color(&self) -> Option<Rgba>;
}

impl JsValueExt for JsValue {
    fn as_f32(&self) -> Option<f32> {
        self.as_number().map(|n| n as f32)
    }

    fn as_str(&self) -> Option<String> {
        self.as_string().and_then(|s| s.to_std_string().ok())
    }

    fn as_color(&self) -> Option<Rgba> {
        self.as_str().and_then(|s| parse_color(&s))
    }
}

/// Collect the elements of a JS array object by walking `length` + indexed gets.
///
/// Callers must have checked `is_array()` first (a non-array object yields an
/// empty `Vec` because it has no numeric `length`). A failed indexed get yields
/// `undefined` for that slot, matching boa's sparse-array read semantics.
pub(crate) fn js_array_values(arr: &JsObject, ctx: &mut JsContext) -> Vec<JsValue> {
    let len = arr
        .get(js_string!("length"), ctx)
        .ok()
        .and_then(|v| v.as_number())
        .unwrap_or(0.0) as u32;
    (0..len)
        .map(|i| arr.get(js_string!(format!("{i}")), ctx).unwrap_or_default())
        .collect()
}

/// Recursively convert a JS value into a `serde_json::Value`.
///
/// Used to pass raw props to native component render functions. Skips callable
/// values defensively (JS already strips handlers in `extractHandlers`).
/// Implemented by hand rather than boa's `to_json` to avoid a feature flag and
/// to control how non-JSON values (NaN, ±Inf, symbols) map.
pub(crate) fn js_to_json(value: &JsValue, ctx: &mut JsContext) -> serde_json::Value {
    use serde_json::Value;

    if value.is_null_or_undefined() {
        return Value::Null;
    }
    if let Some(b) = value.as_boolean() {
        return Value::Bool(b);
    }
    if let Some(n) = value.as_number() {
        // Store integral values as JSON integers: JS has only f64, so `5`
        // arrives as `5.0`; a float-backed serde number would make `as_u64()` return `None`.
        if n.is_finite() && n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
            return Value::Number((n as i64).into());
        }
        // NaN / ±Inf are not representable in JSON → Null.
        return serde_json::Number::from_f64(n)
            .map(Value::Number)
            .unwrap_or(Value::Null);
    }
    if let Some(s) = value.as_string() {
        return Value::String(s.to_std_string().unwrap_or_default());
    }
    if let Some(obj) = value.as_object() {
        if obj.is_array() {
            let values = js_array_values(&obj, ctx);
            let mut arr = Vec::with_capacity(values.len());
            for item in &values {
                arr.push(js_to_json(item, ctx));
            }
            return Value::Array(arr);
        }
        // Own string + integer-index keys only (symbols skipped).
        // Integer-index keys arrive as `PropertyKey::Index` and are stringified
        // so they aren't silently dropped.
        let mut map = serde_json::Map::new();
        if let Ok(keys) = obj.own_property_keys(ctx) {
            for key in keys {
                let key_str = match &key {
                    PropertyKey::String(k) => k.to_std_string().unwrap_or_default(),
                    PropertyKey::Index(i) => i.get().to_string(),
                    PropertyKey::Symbol(_) => continue,
                };
                let v = obj.get(key, ctx).unwrap_or_default();
                if v.is_callable() {
                    continue;
                }
                map.insert(key_str, js_to_json(&v, ctx));
            }
        }
        return Value::Object(map);
    }
    Value::Null
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack::eval_on_parser_stack;

    /// Evaluate a JS expression on a fresh context, returning the resulting value.
    fn eval(src: &str) -> (JsContext, JsValue) {
        let mut ctx = JsContext::builder().build().expect("ctx");
        let value = eval_on_parser_stack(&mut ctx, src.as_bytes()).expect("eval");
        (ctx, value)
    }

    #[test]
    fn as_f32_narrows_numbers_only() {
        assert_eq!(JsValue::from(2.5_f64).as_f32(), Some(2.5));
        assert_eq!(JsValue::from(js_string!("3")).as_f32(), None);
        assert_eq!(JsValue::from(true).as_f32(), None);
    }

    #[test]
    fn as_str_decodes_strings_only() {
        assert_eq!(
            JsValue::from(js_string!("hi")).as_str().as_deref(),
            Some("hi")
        );
        assert_eq!(JsValue::from(7.0_f64).as_str(), None);
    }

    #[test]
    fn as_color_parses_css_strings() {
        assert!(JsValue::from(js_string!("#ff0000")).as_color().is_some());
        assert!(
            JsValue::from(js_string!("not-a-color"))
                .as_color()
                .is_none()
        );
        assert!(JsValue::from(1.0_f64).as_color().is_none());
    }

    #[test]
    fn js_array_values_walks_length_and_indices() {
        let (mut ctx, value) = eval("([10, 20, 30])");
        let arr = value.as_object().expect("array");
        let values = js_array_values(&arr, &mut ctx);
        let nums: Vec<f32> = values.iter().filter_map(|v| v.as_f32()).collect();
        assert_eq!(nums, vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn js_to_json_integers_stay_integers() {
        let (mut ctx, value) = eval("(5)");
        assert_eq!(js_to_json(&value, &mut ctx), serde_json::json!(5));
    }

    #[test]
    fn js_to_json_non_finite_becomes_null() {
        let (mut ctx, value) = eval("(1/0)"); // Infinity
        assert_eq!(js_to_json(&value, &mut ctx), serde_json::Value::Null);
    }

    #[test]
    fn js_to_json_nested_array_and_object_skip_callables() {
        let (mut ctx, value) = eval("({ a: [1, 2], b: 'x', fn: () => {}, n: true })");
        let json = js_to_json(&value, &mut ctx);
        assert_eq!(
            json,
            serde_json::json!({ "a": [1, 2], "b": "x", "n": true })
        );
    }
}
