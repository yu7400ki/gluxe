use std::sync::Arc;

use gpui::{
    BorderStyle, BoxShadow, EdgesRefinement, FontFeatures, FontWeight, Overflow, SharedString,
    StrikethroughStyle, Styled, UnderlineStyle, Visibility, point, px,
};

use crate::model::{BoxShadowSpec, LengthValue, OverflowMode, StyleFields};

/// Apply `StyleFields` to any element that implements GPUI's `Styled` trait.
///
/// Works for both `Div` (base styles) and `StyleRefinement` (pseudo-selector overlays
/// passed into `.hover()` / `.active()` closures).
pub(crate) fn apply_style_props<T: Styled>(mut element: T, props: &StyleFields) -> T {
    match props.display.as_deref() {
        Some("flex") => element = element.flex(),
        Some("block") => element = element.block(),
        Some("grid") => element = element.grid(),
        Some("none") => element = element.hidden(),
        _ => {}
    }

    // Keyword `flex` takes priority; numeric flex uses React Native semantics
    // (positive n → grow=n, shrink=1, basis=0; zero → flex_initial).
    match props.flex_keyword.as_deref() {
        Some("auto") => element = element.flex_auto(),
        Some("initial") => element = element.flex_initial(),
        Some("none") => element = element.flex_none(),
        _ => {
            if let Some(n) = props.flex {
                if n > 0.0 {
                    element.style().flex_grow = Some(n);
                    element.style().flex_shrink = Some(1.0);
                    element.style().flex_basis = Some(gpui::relative(0.).into());
                } else {
                    element = element.flex_initial();
                }
            }
        }
    }
    // Individual flex-grow / flex-shrink / flex-basis override the shorthand.
    if let Some(n) = props.flex_grow {
        element.style().flex_grow = Some(n);
    }
    if let Some(n) = props.flex_shrink {
        element.style().flex_shrink = Some(n);
    }
    if let Some(v) = props.flex_basis {
        element = element.flex_basis(v.to_length());
    }

    match props.flex_direction.as_deref() {
        Some("row") => element = element.flex_row(),
        Some("column") => element = element.flex_col(),
        Some("row-reverse") | Some("row_reverse") => element = element.flex_row_reverse(),
        Some("column-reverse") | Some("column_reverse") => element = element.flex_col_reverse(),
        _ => {}
    }

    match props.flex_wrap.as_deref() {
        Some("wrap") => element = element.flex_wrap(),
        Some("nowrap") => element = element.flex_nowrap(),
        Some("wrap-reverse") | Some("wrap_reverse") => element = element.flex_wrap_reverse(),
        _ => {}
    }
    if let Some(v) = props.width {
        element = element.w(v.to_length());
    }
    if let Some(v) = props.height {
        element = element.h(v.to_length());
    }
    // Padding / gap / margin: uniform → per-axis → per-side (most specific wins).
    if let Some(v) = props.padding {
        if let Some(d) = v.to_definite() {
            element = element.p(d);
        }
    }
    if let Some(v) = props.padding_x {
        if let Some(d) = v.to_definite() {
            element = element.px(d);
        }
    }
    if let Some(v) = props.padding_y {
        if let Some(d) = v.to_definite() {
            element = element.py(d);
        }
    }
    if let Some(v) = props.padding_top {
        if let Some(d) = v.to_definite() {
            element = element.pt(d);
        }
    }
    if let Some(v) = props.padding_right {
        if let Some(d) = v.to_definite() {
            element = element.pr(d);
        }
    }
    if let Some(v) = props.padding_bottom {
        if let Some(d) = v.to_definite() {
            element = element.pb(d);
        }
    }
    if let Some(v) = props.padding_left {
        if let Some(d) = v.to_definite() {
            element = element.pl(d);
        }
    }
    if let Some(v) = props.gap {
        if let Some(d) = v.to_definite() {
            element = element.gap(d);
        }
    }
    if let Some(v) = props.gap_x {
        if let Some(d) = v.to_definite() {
            element = element.gap_x(d);
        }
    }
    if let Some(v) = props.gap_y {
        if let Some(d) = v.to_definite() {
            element = element.gap_y(d);
        }
    }
    if let Some(n) = props.grid_template_columns {
        element = element.grid_cols(n);
    }
    if let Some(n) = props.grid_template_rows {
        element = element.grid_rows(n);
    }
    if props.grid_column_start.is_some()
        || props.grid_column_end.is_some()
        || props.grid_row_start.is_some()
        || props.grid_row_end.is_some()
    {
        let loc = element.style().grid_location_mut();
        if let Some(p) = props.grid_column_start {
            loc.column.start = p;
        }
        if let Some(p) = props.grid_column_end {
            loc.column.end = p;
        }
        if let Some(p) = props.grid_row_start {
            loc.row.start = p;
        }
        if let Some(p) = props.grid_row_end {
            loc.row.end = p;
        }
    }
    match props.align_items.as_deref() {
        Some("flex-start") | Some("flex_start") => element = element.items_start(),
        Some("center") => element = element.items_center(),
        Some("flex-end") | Some("flex_end") => element = element.items_end(),
        Some("baseline") => element = element.items_baseline(),
        Some("stretch") => element = element.items_stretch(),
        _ => {}
    }
    match props.align_self.as_deref() {
        Some("flex-start") | Some("flex_start") | Some("start") => {
            element = element.self_flex_start()
        }
        Some("flex-end") | Some("flex_end") | Some("end") => element = element.self_flex_end(),
        Some("center") => element = element.self_center(),
        Some("baseline") => element = element.self_baseline(),
        Some("stretch") => element = element.self_stretch(),
        _ => {}
    }
    match props.align_content.as_deref() {
        Some("normal") => element = element.content_normal(),
        Some("center") => element = element.content_center(),
        Some("flex-start") | Some("flex_start") | Some("start") => {
            element = element.content_start()
        }
        Some("flex-end") | Some("flex_end") | Some("end") => element = element.content_end(),
        Some("space-between") | Some("space_between") => element = element.content_between(),
        Some("space-around") | Some("space_around") => element = element.content_around(),
        Some("space-evenly") | Some("space_evenly") => element = element.content_evenly(),
        Some("stretch") => element = element.content_stretch(),
        _ => {}
    }
    match props.justify_content.as_deref() {
        Some("flex-start") | Some("flex_start") => element = element.justify_start(),
        Some("center") => element = element.justify_center(),
        Some("flex-end") | Some("flex_end") => element = element.justify_end(),
        Some("space-between") | Some("space_between") => element = element.justify_between(),
        Some("space-around") | Some("space_around") => element = element.justify_around(),
        Some("space-evenly") | Some("space_evenly") => element = element.justify_evenly(),
        _ => {}
    }
    if let Some(background_color) = props.background_color {
        element = element.bg(background_color);
    }
    if let Some(v) = props.border_radius {
        if let Some(a) = v.to_absolute() {
            element = element.rounded(a);
        }
    }
    if let Some(color) = props.color {
        element = element.text_color(color);
    }
    if let Some(v) = props.font_size {
        if let Some(a) = v.to_absolute() {
            element = element.text_size(a);
        }
    }
    if let Some(w) = props.font_weight {
        element = element.font_weight(FontWeight(w));
    }
    // GPUI renders a border only when both border_widths > 0 and border_color are set.
    if let Some(c) = props.border_color {
        element = element.border_color(c);
    }
    if let Some(v) = props.border_width {
        if let Some(a) = v.to_absolute() {
            element.style().border_widths = EdgesRefinement {
                top: Some(a),
                left: Some(a),
                right: Some(a),
                bottom: Some(a),
            };
        }
    }
    match props.cursor.as_deref() {
        Some("pointer") => element = element.cursor_pointer(),
        Some("default") => element = element.cursor_default(),
        Some("text") => element = element.cursor_text(),
        Some("move") => element = element.cursor_move(),
        Some("grab") => element = element.cursor_grab(),
        Some("grabbing") => element = element.cursor_grabbing(),
        Some("crosshair") => element = element.cursor_crosshair(),
        Some("not-allowed") => element = element.cursor_not_allowed(),
        Some("no-drop") => element = element.cursor_no_drop(),
        Some("context-menu") => element = element.cursor_context_menu(),
        Some("copy") => element = element.cursor_copy(),
        Some("alias") => element = element.cursor_alias(),
        Some("vertical-text") => element = element.cursor_vertical_text(),
        Some("ew-resize") => element = element.cursor_ew_resize(),
        Some("ns-resize") => element = element.cursor_ns_resize(),
        Some("nesw-resize") => element = element.cursor_nesw_resize(),
        Some("nwse-resize") => element = element.cursor_nwse_resize(),
        Some("col-resize") => element = element.cursor_col_resize(),
        Some("row-resize") => element = element.cursor_row_resize(),
        Some("n-resize") => element = element.cursor_n_resize(),
        Some("e-resize") => element = element.cursor_e_resize(),
        Some("s-resize") => element = element.cursor_s_resize(),
        Some("w-resize") => element = element.cursor_w_resize(),
        _ => {}
    }
    if props.white_space.as_deref() == Some("nowrap") {
        element = element.whitespace_nowrap();
    }
    match props.text_overflow.as_deref() {
        Some("ellipsis") => element = element.text_ellipsis(),
        Some("ellipsis-start") => element = element.text_ellipsis_start(),
        _ => {}
    }
    if let Some(n) = props.line_clamp {
        element = element.line_clamp(n as usize);
    }
    // Scroll overflow is handled separately in the stateful render branch
    // (requires StatefulInteractiveElement::overflow_*_scroll).
    match props.overflow_x {
        Some(OverflowMode::Hidden) => element.style().overflow.x = Some(Overflow::Hidden),
        Some(OverflowMode::Visible) => element.style().overflow.x = Some(Overflow::Visible),
        _ => {}
    }
    match props.overflow_y {
        Some(OverflowMode::Hidden) => element.style().overflow.y = Some(Overflow::Hidden),
        Some(OverflowMode::Visible) => element.style().overflow.y = Some(Overflow::Visible),
        _ => {}
    }
    if let Some(v) = props.margin {
        element = element.m(v.to_length());
    }
    if let Some(v) = props.margin_x {
        element = element.mx(v.to_length());
    }
    if let Some(v) = props.margin_y {
        element = element.my(v.to_length());
    }
    if let Some(v) = props.margin_top {
        element = element.mt(v.to_length());
    }
    if let Some(v) = props.margin_right {
        element = element.mr(v.to_length());
    }
    if let Some(v) = props.margin_bottom {
        element = element.mb(v.to_length());
    }
    if let Some(v) = props.margin_left {
        element = element.ml(v.to_length());
    }
    if let Some(v) = props.min_width {
        element = element.min_w(v.to_length());
    }
    if let Some(v) = props.min_height {
        element = element.min_h(v.to_length());
    }
    if let Some(v) = props.max_width {
        element = element.max_w(v.to_length());
    }
    if let Some(v) = props.max_height {
        element = element.max_h(v.to_length());
    }
    if let Some(ratio) = props.aspect_ratio {
        element = element.aspect_ratio(ratio);
    }
    match props.position.as_deref() {
        Some("relative") => element = element.relative(),
        Some("absolute") => element = element.absolute(),
        _ => {}
    }
    if let Some(v) = props.inset {
        element = element.inset(v.to_length());
    }
    if let Some(v) = props.top {
        element = element.top(v.to_length());
    }
    if let Some(v) = props.right {
        element = element.right(v.to_length());
    }
    if let Some(v) = props.bottom {
        element = element.bottom(v.to_length());
    }
    if let Some(v) = props.left {
        element = element.left(v.to_length());
    }
    if let Some(o) = props.opacity {
        element = element.opacity(o);
    }
    match props.visibility.as_deref() {
        Some("hidden") => element.style().visibility = Some(Visibility::Hidden),
        Some("visible") => element.style().visibility = Some(Visibility::Visible),
        _ => {}
    }
    match props.border_style.as_deref() {
        Some("dashed") => element = element.border_dashed(),
        Some("solid") => element.style().border_style = Some(BorderStyle::Solid),
        _ => {}
    }
    if let Some(a) = props.border_top_width.and_then(|v| v.to_absolute()) {
        element.style().border_widths.top = Some(a);
    }
    if let Some(a) = props.border_right_width.and_then(|v| v.to_absolute()) {
        element.style().border_widths.right = Some(a);
    }
    if let Some(a) = props.border_bottom_width.and_then(|v| v.to_absolute()) {
        element.style().border_widths.bottom = Some(a);
    }
    if let Some(a) = props.border_left_width.and_then(|v| v.to_absolute()) {
        element.style().border_widths.left = Some(a);
    }
    if let Some(a) = props.border_top_left_radius.and_then(|v| v.to_absolute()) {
        element.style().corner_radii.top_left = Some(a);
    }
    if let Some(a) = props.border_top_right_radius.and_then(|v| v.to_absolute()) {
        element.style().corner_radii.top_right = Some(a);
    }
    if let Some(a) = props
        .border_bottom_right_radius
        .and_then(|v| v.to_absolute())
    {
        element.style().corner_radii.bottom_right = Some(a);
    }
    if let Some(a) = props
        .border_bottom_left_radius
        .and_then(|v| v.to_absolute())
    {
        element.style().corner_radii.bottom_left = Some(a);
    }
    if let Some(a) = props.scrollbar_width.and_then(|v| v.to_absolute()) {
        element = element.scrollbar_width(a);
    }
    match &props.box_shadow {
        Some(BoxShadowSpec::Preset(p)) => {
            element = match p.as_str() {
                "2xs" => element.shadow_2xs(),
                "xs" => element.shadow_xs(),
                "sm" => element.shadow_sm(),
                "md" => element.shadow_md(),
                "lg" => element.shadow_lg(),
                "xl" => element.shadow_xl(),
                "2xl" => element.shadow_2xl(),
                "none" => element.shadow_none(),
                _ => element,
            };
        }
        Some(BoxShadowSpec::Custom(list)) => {
            let shadows: Vec<BoxShadow> = list
                .iter()
                .map(|s| BoxShadow {
                    color: s.color.into(),
                    offset: point(px(s.offset_x), px(s.offset_y)),
                    blur_radius: px(s.blur_radius),
                    spread_radius: px(s.spread_radius),
                    inset: s.inset,
                })
                .collect();
            element = element.shadow(shadows);
        }
        None => {}
    }

    match props.text_align.as_deref() {
        Some("left") => element = element.text_left(),
        Some("center") => element = element.text_center(),
        Some("right") => element = element.text_right(),
        _ => {}
    }
    match props.font_style.as_deref() {
        Some("italic") => element = element.italic(),
        Some("normal") => element = element.not_italic(),
        _ => {}
    }
    if let Some(f) = &props.font_family {
        element = element.font_family(SharedString::from(f.clone()));
    }
    if let Some(lh) = props.line_height {
        element = element.line_height(lh);
    }
    if let Some(bg) = props.text_background_color {
        element = element.text_bg(bg);
    }
    if let Some(list) = &props.font_features {
        element = element.font_features(FontFeatures(Arc::new(list.clone())));
    }
    // Use text_style() directly so color and wavy apply to both decoration types;
    // the Styled helper methods only target underline.
    let (want_underline, want_strikethrough) = match props.text_decoration_line.as_deref() {
        Some("underline") => (true, false),
        Some("line-through") => (false, true),
        Some("underline line-through") | Some("line-through underline") => (true, true),
        Some("none") => {
            // Explicitly clear any inherited decoration.
            let ts = element.text_style();
            ts.underline = None;
            ts.strikethrough = None;
            (false, false)
        }
        _ => (false, false),
    };
    if want_underline || want_strikethrough {
        let thickness = props
            .text_decoration_thickness
            .and_then(|v| match v {
                LengthValue::Px(n) => Some(px(n)),
                _ => None,
            })
            .unwrap_or(px(1.));
        let color: Option<gpui::Hsla> = props.text_decoration_color.map(Into::into);
        let wavy = props.text_decoration_style.as_deref() == Some("wavy");
        let ts = element.text_style();
        if want_underline {
            ts.underline = Some(UnderlineStyle {
                thickness,
                color,
                wavy,
            });
        }
        if want_strikethrough {
            ts.strikethrough = Some(StrikethroughStyle { thickness, color });
        }
    }

    element
}
