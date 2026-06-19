// Animatable-field registry.
//
// The `animatable_fields!` macro declares the full set of interpolatable
// `StyleFields` entries — `(rust_ident, "jsCamelName", Kind)` — and generates
// the id enum, name lookup, typed read/write accessors, and change diff.
// Fields absent from the list (strings, enums, grid placement, box shadows, …)
// always apply instantly.

use gpui::Rgba;

use crate::model::{LengthValue, StyleFields};

/// A style value that can be interpolated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum AnimValue {
    Length(LengthValue),
    Color(Rgba),
    F32(f32),
}

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

impl AnimValue {
    /// Interpolate between two values. Returns `None` when not interpolable
    /// (unit mismatch, `auto`, or kind mismatch) — caller applies target instantly.
    pub(crate) fn lerp(from: Self, to: Self, t: f32) -> Option<Self> {
        use LengthValue::*;
        match (from, to) {
            (Self::Length(a), Self::Length(b)) => {
                let v = match (a, b) {
                    (Px(x), Px(y)) => Px(lerp_f32(x, y, t)),
                    (Rem(x), Rem(y)) => Rem(lerp_f32(x, y, t)),
                    (Percent(x), Percent(y)) => Percent(lerp_f32(x, y, t)),
                    _ => return None,
                };
                Some(Self::Length(v))
            }
            (Self::Color(a), Self::Color(b)) => Some(Self::Color(Rgba {
                // Component-wise lerp (not premultiplied-alpha, but close enough).
                r: lerp_f32(a.r, b.r, t),
                g: lerp_f32(a.g, b.g, t),
                b: lerp_f32(a.b, b.b, t),
                a: lerp_f32(a.a, b.a, t),
            })),
            (Self::F32(a), Self::F32(b)) => Some(Self::F32(lerp_f32(a, b, t))),
            _ => None,
        }
    }
}

macro_rules! animatable_fields {
    ($(($field:ident, $name:literal, $kind:ident)),* $(,)?) => {
        /// Identifies one animatable `StyleFields` field (variant names are macro-generated snake_case idents).
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub(crate) enum FieldId {
            $($field,)*
        }

        /// JS camelCase style-prop name → [`FieldId`]. `None` for names that
        /// are unknown or not animatable.
        pub(crate) fn field_id_from_name(name: &str) -> Option<FieldId> {
            match name {
                $($name => Some(FieldId::$field),)*
                _ => None,
            }
        }

        /// Read a field's current value out of a [`StyleFields`].
        pub(crate) fn read_field(style: &StyleFields, id: FieldId) -> Option<AnimValue> {
            match id {
                $(FieldId::$field => style.$field.map(AnimValue::$kind),)*
            }
        }

        /// Write an interpolated value into the matching field.
        /// Kind mismatch is impossible by construction and is silently ignored.
        pub(crate) fn write_field(style: &mut StyleFields, id: FieldId, v: AnimValue) {
            match id {
                $(FieldId::$field => {
                    if let AnimValue::$kind(inner) = v {
                        style.$field = Some(inner);
                    }
                })*
            }
        }

        /// All animatable fields whose value differs between `old` and `new`.
        pub(crate) fn diff_animatable(old: &StyleFields, new: &StyleFields) -> Vec<FieldId> {
            let mut out = Vec::new();
            $(if old.$field != new.$field {
                out.push(FieldId::$field);
            })*
            out
        }
    };
}

animatable_fields![
    // ---- Lengths ----
    (width, "width", Length),
    (height, "height", Length),
    (min_width, "minWidth", Length),
    (min_height, "minHeight", Length),
    (max_width, "maxWidth", Length),
    (max_height, "maxHeight", Length),
    (flex_basis, "flexBasis", Length),
    (padding, "padding", Length),
    (padding_x, "paddingX", Length),
    (padding_y, "paddingY", Length),
    (padding_top, "paddingTop", Length),
    (padding_right, "paddingRight", Length),
    (padding_bottom, "paddingBottom", Length),
    (padding_left, "paddingLeft", Length),
    (margin, "margin", Length),
    (margin_x, "marginX", Length),
    (margin_y, "marginY", Length),
    (margin_top, "marginTop", Length),
    (margin_right, "marginRight", Length),
    (margin_bottom, "marginBottom", Length),
    (margin_left, "marginLeft", Length),
    (gap, "gap", Length),
    (gap_x, "gapX", Length),
    (gap_y, "gapY", Length),
    (inset, "inset", Length),
    (top, "top", Length),
    (right, "right", Length),
    (bottom, "bottom", Length),
    (left, "left", Length),
    (border_radius, "borderRadius", Length),
    (border_top_left_radius, "borderTopLeftRadius", Length),
    (border_top_right_radius, "borderTopRightRadius", Length),
    (
        border_bottom_right_radius,
        "borderBottomRightRadius",
        Length
    ),
    (border_bottom_left_radius, "borderBottomLeftRadius", Length),
    (border_width, "borderWidth", Length),
    (border_top_width, "borderTopWidth", Length),
    (border_right_width, "borderRightWidth", Length),
    (border_bottom_width, "borderBottomWidth", Length),
    (border_left_width, "borderLeftWidth", Length),
    (font_size, "fontSize", Length),
    // ---- Colors ----
    (background_color, "backgroundColor", Color),
    (color, "color", Color),
    (border_color, "borderColor", Color),
    (text_decoration_color, "textDecorationColor", Color),
    (text_background_color, "textBackgroundColor", Color),
    // ---- Scalars ----
    (opacity, "opacity", F32),
    (flex_grow, "flexGrow", F32),
    (flex_shrink, "flexShrink", F32),
    (font_weight, "fontWeight", F32),
    (aspect_ratio, "aspectRatio", F32),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_lookup() {
        assert_eq!(field_id_from_name("width"), Some(FieldId::width));
        assert_eq!(
            field_id_from_name("backgroundColor"),
            Some(FieldId::background_color)
        );
        assert_eq!(field_id_from_name("opacity"), Some(FieldId::opacity));
        // Known style props that are not animatable:
        assert_eq!(field_id_from_name("display"), None);
        assert_eq!(field_id_from_name("boxShadow"), None);
        assert_eq!(field_id_from_name("nonsense"), None);
    }

    #[test]
    fn read_write_roundtrip() {
        let mut style = StyleFields::default();
        assert_eq!(read_field(&style, FieldId::width), None);
        write_field(
            &mut style,
            FieldId::width,
            AnimValue::Length(LengthValue::Px(42.0)),
        );
        assert_eq!(
            read_field(&style, FieldId::width),
            Some(AnimValue::Length(LengthValue::Px(42.0)))
        );
    }

    #[test]
    fn diff_detects_changed_fields_only() {
        let old = StyleFields {
            width: Some(LengthValue::Px(10.0)),
            opacity: Some(1.0),
            ..Default::default()
        };
        let mut new = old.clone();
        new.width = Some(LengthValue::Px(20.0));
        new.background_color = Some(Rgba {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        });
        let diff = diff_animatable(&old, &new);
        assert!(diff.contains(&FieldId::width));
        assert!(diff.contains(&FieldId::background_color));
        assert!(!diff.contains(&FieldId::opacity));
        assert_eq!(diff.len(), 2);
    }

    #[test]
    fn lerp_same_unit_lengths() {
        let a = AnimValue::Length(LengthValue::Px(0.0));
        let b = AnimValue::Length(LengthValue::Px(100.0));
        assert_eq!(
            AnimValue::lerp(a, b, 0.5),
            Some(AnimValue::Length(LengthValue::Px(50.0)))
        );
        let a = AnimValue::Length(LengthValue::Percent(0.0));
        let b = AnimValue::Length(LengthValue::Percent(50.0));
        assert_eq!(
            AnimValue::lerp(a, b, 0.5),
            Some(AnimValue::Length(LengthValue::Percent(25.0)))
        );
    }

    #[test]
    fn lerp_mismatched_units_is_none() {
        let px = AnimValue::Length(LengthValue::Px(10.0));
        let pct = AnimValue::Length(LengthValue::Percent(50.0));
        let auto = AnimValue::Length(LengthValue::Auto);
        assert_eq!(AnimValue::lerp(px, pct, 0.5), None);
        assert_eq!(AnimValue::lerp(px, auto, 0.5), None);
        assert_eq!(AnimValue::lerp(auto, px, 0.5), None);
        // Kind mismatch
        assert_eq!(AnimValue::lerp(px, AnimValue::F32(1.0), 0.5), None);
    }

    #[test]
    fn lerp_colors_componentwise() {
        let black = AnimValue::Color(Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        });
        let white = AnimValue::Color(Rgba {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        });
        let Some(AnimValue::Color(mid)) = AnimValue::lerp(black, white, 0.5) else {
            panic!("expected color");
        };
        assert!((mid.r - 0.5).abs() < 1e-6);
        assert!((mid.g - 0.5).abs() < 1e-6);
        assert!((mid.a - 1.0).abs() < 1e-6);
    }
}
