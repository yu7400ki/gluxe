//! Native scrollbar element for `<ScrollArea>` (the built-in `__GluxeScrollbar`
//! host type). The thumb is sized, positioned, painted, and dragged in GPUI from
//! the target viewport's live `ScrollHandle` (cloned via the `target` prop), so
//! nothing round-trips through JS. Mirrors Zed's `crates/ui/src/components/scrollbar.rs`.

use std::cell::RefCell;

use gpui::{
    Along, AnyElement, Axis, BorderStyle, Bounds, ContentMask, Corners, CursorStyle, DispatchPhase,
    Edges, Hitbox, HitboxBehavior, Hsla, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, ScrollHandle, Styled, canvas, px, quad,
};
use rustc_hash::FxHashMap;
use serde_json::Value;

use crate::{model::ElementId, state::scroll_handle, style::parse_color};

const DEFAULT_MIN_THUMB: f32 = 20.0;
const DEFAULT_THUMB_RADIUS: f32 = 0.0;

fn default_thumb_color() -> Hsla {
    // Neutral translucent gray; headless consumers are expected to set `thumbColor`.
    Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.5,
        a: 0.5,
    }
}

// ---------------------------------------------------------------------------
// Pure geometry (unit-tested without a GPUI window)
// ---------------------------------------------------------------------------

/// Thumb length + leading-edge position along the axis (track-local px), or `None`
/// when not scrollable / the track can't fit a movable thumb. `max_offset` ≥ 0
/// (`content − viewport`); `current_offset` ≤ 0 (more negative = scrolled further).
fn thumb_geometry(
    track_len: f32,
    viewport_len: f32,
    max_offset: f32,
    current_offset: f32,
    min_thumb: f32,
) -> Option<(f32, f32)> {
    if max_offset <= 0.0 || viewport_len <= 0.0 || track_len <= 0.0 {
        return None;
    }
    let content_len = viewport_len + max_offset;
    let thumb_len = (track_len * (viewport_len / content_len)).max(min_thumb);
    if thumb_len >= track_len {
        // Track too short to show a movable thumb.
        return None;
    }
    let progress = (-current_offset / max_offset).clamp(0.0, 1.0);
    let thumb_pos = (track_len - thumb_len) * progress;
    Some((thumb_len, thumb_pos))
}

/// Pointer position along the track → scroll offset, clamped to `[-max_offset, 0]`
/// (Zed's `compute_click_offset`). `grab` = pointer's distance from the thumb's
/// leading edge: `thumb_len / 2` for a track-click center-jump, else the drag grab.
fn click_to_offset(
    track_len: f32,
    thumb_len: f32,
    track_origin: f32,
    event_pos: f32,
    max_offset: f32,
    grab: f32,
) -> f32 {
    let span = track_len - thumb_len;
    let thumb_start = (event_pos - track_origin - grab).clamp(0.0, span.max(0.0));
    let percentage = if span > 0.0 { thumb_start / span } else { 0.0 };
    -max_offset * percentage
}

/// Place the thumb in the track's `bounds`: length/position along `axis`, filling
/// the cross axis minus `inset` per side. The element's only axis mapping, kept
/// pure so both orientations are unit-tested.
fn thumb_rect(
    track: Bounds<Pixels>,
    axis: Axis,
    thumb_len: f32,
    thumb_pos: f32,
    inset: f32,
) -> Bounds<Pixels> {
    let cross = axis.invert();
    let origin = track
        .origin
        .apply_along(axis, |o| o + px(thumb_pos))
        .apply_along(cross, |o| o + px(inset));
    let size = track
        .size
        .apply_along(axis, |_| px(thumb_len))
        .apply_along(cross, |s| (s - px(inset * 2.0)).max(px(0.0)));
    Bounds::new(origin, size)
}

// ---------------------------------------------------------------------------
// Drag state (the element is stateless; keyed by target viewport + axis)
// ---------------------------------------------------------------------------

thread_local! {
    /// `(target, axis)` → grab offset, present only while dragging. Keyed by target
    /// so the element needs no id of its own (`NativeRenderContext` carries none).
    static SCROLLBAR_DRAG: RefCell<FxHashMap<(ElementId, u8), f32>> =
        RefCell::new(FxHashMap::default());
}

fn axis_key(axis: Axis) -> u8 {
    match axis {
        Axis::Horizontal => 0,
        Axis::Vertical => 1,
    }
}

fn drag_grab(target: ElementId, axis: Axis) -> Option<f32> {
    SCROLLBAR_DRAG.with(|m| m.borrow().get(&(target, axis_key(axis))).copied())
}

fn set_drag_grab(target: ElementId, axis: Axis, grab: f32) {
    SCROLLBAR_DRAG.with(|m| {
        m.borrow_mut().insert((target, axis_key(axis)), grab);
    });
}

fn clear_drag_grab(target: ElementId, axis: Axis) {
    SCROLLBAR_DRAG.with(|m| {
        m.borrow_mut().remove(&(target, axis_key(axis)));
    });
}

/// Drop `target`'s drag state when its viewport node is detached, so a drag can't
/// outlive the node (and misfire if the `ElementId` is later reused).
pub(crate) fn clear_scrollbar_drag_for(target: ElementId) {
    SCROLLBAR_DRAG.with(|m| {
        let mut m = m.borrow_mut();
        m.remove(&(target, axis_key(Axis::Horizontal)));
        m.remove(&(target, axis_key(Axis::Vertical)));
    });
}

/// Drop all drag state on dev-mode full reload (see [`crate::render::clear_node_views`]).
#[cfg(debug_assertions)]
pub(crate) fn clear_scrollbar_drag() {
    SCROLLBAR_DRAG.with(|m| m.borrow_mut().clear());
}

/// Register this module's node-lifecycle cleanup with the lifecycle seam.
pub(crate) fn register_lifecycle() {
    crate::lifecycle::on_detach(clear_scrollbar_drag_for);
    #[cfg(debug_assertions)]
    crate::lifecycle::on_reload(clear_scrollbar_drag);
}

// ---------------------------------------------------------------------------
// Element construction (registered render fn)
// ---------------------------------------------------------------------------

fn prop_f32(props: &Value, key: &str) -> Option<f32> {
    props.get(key).and_then(Value::as_f64).map(|v| v as f32)
}

fn prop_color(props: &Value, key: &str) -> Option<Hsla> {
    props
        .get(key)
        .and_then(Value::as_str)
        .and_then(parse_color)
        .map(Hsla::from)
}

/// Thumb fill per interaction state. `active` (while dragging) falls back to
/// `hover` then `base`; `hover` falls back to `base`.
struct ThumbColors {
    base: Hsla,
    hover: Option<Hsla>,
    active: Option<Hsla>,
}

impl ThumbColors {
    /// The fill to paint for the current interaction state.
    fn pick(&self, hovered: bool, dragging: bool) -> Hsla {
        if dragging {
            self.active.or(self.hover).unwrap_or(self.base)
        } else if hovered {
            self.hover.unwrap_or(self.base)
        } else {
            self.base
        }
    }
}

/// Render fn for `__GluxeScrollbar`: reads `target`/`orientation`/thumb props and
/// returns a `canvas` that paints + drives the thumb from the viewport handle.
pub(crate) fn build_scrollbar(props: &Value) -> Vec<AnyElement> {
    let Some(target) = props.get("target").and_then(Value::as_u64) else {
        // No viewport id yet (pre ref-commit) → render nothing.
        return Vec::new();
    };

    let axis = match props.get("orientation").and_then(Value::as_str) {
        Some("horizontal") => Axis::Horizontal,
        _ => Axis::Vertical,
    };
    let min_thumb = prop_f32(props, "minThumbLength").unwrap_or(DEFAULT_MIN_THUMB);
    let thumb_radius = prop_f32(props, "thumbRadius").unwrap_or(DEFAULT_THUMB_RADIUS);
    let thumb_inset = prop_f32(props, "thumbInset").unwrap_or(0.0);
    let colors = ThumbColors {
        base: prop_color(props, "thumbColor").unwrap_or_else(default_thumb_color),
        hover: prop_color(props, "thumbHoverColor"),
        active: prop_color(props, "thumbActiveColor"),
    };

    let handle = scroll_handle(target);

    vec![
        scrollbar_canvas(
            target,
            axis,
            handle,
            min_thumb,
            colors,
            thumb_radius,
            thumb_inset,
        )
        .into_any_element(),
    ]
}

/// Geometry computed in prepaint and consumed in paint.
struct ThumbLayout {
    thumb_bounds: Bounds<Pixels>,
    track_bounds: Bounds<Pixels>,
    /// Whole scrollbar: blocks clicks (not scroll), sets the cursor.
    track_hitbox: Hitbox,
    /// Just the thumb: drives the hover colour + its redraw.
    thumb_hitbox: Hitbox,
}

fn set_handle_offset(handle: &ScrollHandle, axis: Axis, value: f32) {
    let offset = handle.offset().apply_along(axis, |_| px(value));
    handle.set_offset(offset);
}

#[allow(clippy::too_many_arguments)]
fn scrollbar_canvas(
    target: ElementId,
    axis: Axis,
    handle: ScrollHandle,
    min_thumb: f32,
    colors: ThumbColors,
    thumb_radius: f32,
    thumb_inset: f32,
) -> impl IntoElement {
    let prepaint_handle = handle.clone();
    canvas(
        move |bounds, window, _cx| -> Option<ThumbLayout> {
            let track_len = f32::from(bounds.size.along(axis));
            let viewport_len = f32::from(prepaint_handle.bounds().size.along(axis));
            let max_offset = f32::from(prepaint_handle.max_offset().along(axis));
            let current = f32::from(prepaint_handle.offset().along(axis));

            let (thumb_len, thumb_pos) =
                thumb_geometry(track_len, viewport_len, max_offset, current, min_thumb)?;

            let thumb_bounds = thumb_rect(bounds, axis, thumb_len, thumb_pos, thumb_inset);

            // Block clicks on the scrollbar from reaching content beneath, but
            // pass the wheel through so wheel-over-scrollbar still scrolls.
            let track_hitbox = window.insert_hitbox(bounds, HitboxBehavior::BlockMouseExceptScroll);
            // Thumb hitbox (on top) drives the hover colour + hover-change redraw,
            // like gpui's own `.hover()`.
            let thumb_hitbox =
                window.insert_hitbox(thumb_bounds, HitboxBehavior::BlockMouseExceptScroll);

            Some(ThumbLayout {
                thumb_bounds,
                track_bounds: bounds,
                track_hitbox,
                thumb_hitbox,
            })
        },
        move |bounds, layout, window, _cx| {
            let Some(ThumbLayout {
                thumb_bounds,
                track_bounds,
                track_hitbox,
                thumb_hitbox,
            }) = layout
            else {
                return;
            };

            let dragging = drag_grab(target, axis).is_some();
            let hovered = thumb_hitbox.is_hovered(window);
            // While dragging, use the capture phase so the thumb keeps tracking
            // the pointer even off the thumb/track bounds.
            let capture_phase = if dragging {
                DispatchPhase::Capture
            } else {
                DispatchPhase::Bubble
            };

            window.with_content_mask(Some(ContentMask { bounds }), |window| {
                window.paint_quad(quad(
                    thumb_bounds,
                    Corners::all(px(thumb_radius)).clamp_radii_for_quad_size(thumb_bounds.size),
                    colors.pick(hovered, dragging),
                    Edges::default(),
                    Hsla::transparent_black(),
                    BorderStyle::default(),
                ));
                window.set_cursor_style(CursorStyle::Arrow, &track_hitbox);

                // Redraw when the pointer crosses the thumb edge so the hover
                // colour swaps (like gpui's `.hover()`).
                window.on_mouse_event({
                    let thumb_hitbox = thumb_hitbox.clone();
                    move |_event: &MouseMoveEvent, phase, window, _cx| {
                        if phase == DispatchPhase::Capture
                            && thumb_hitbox.is_hovered(window) != hovered
                        {
                            window.refresh();
                        }
                    }
                });

                window.on_mouse_event({
                    let handle = handle.clone();
                    move |event: &MouseDownEvent, phase, window, cx| {
                        if phase != capture_phase || event.button != MouseButton::Left {
                            return;
                        }
                        if !track_bounds.contains(&event.position) {
                            return;
                        }
                        if thumb_bounds.contains(&event.position) {
                            // Start dragging: record the grab offset within the thumb.
                            let grab = f32::from(event.position.along(axis))
                                - f32::from(thumb_bounds.origin.along(axis));
                            set_drag_grab(target, axis, grab);
                        } else {
                            // Track click: center the thumb on the pointer.
                            let max_offset = f32::from(handle.max_offset().along(axis));
                            let thumb_len = f32::from(thumb_bounds.size.along(axis));
                            let new = click_to_offset(
                                f32::from(track_bounds.size.along(axis)),
                                thumb_len,
                                f32::from(track_bounds.origin.along(axis)),
                                f32::from(event.position.along(axis)),
                                max_offset,
                                thumb_len / 2.0,
                            );
                            set_handle_offset(&handle, axis, new);
                        }
                        window.refresh();
                        cx.stop_propagation();
                    }
                });

                window.on_mouse_event({
                    let handle = handle.clone();
                    move |event: &MouseMoveEvent, phase, window, cx| {
                        if phase != capture_phase {
                            return;
                        }
                        let Some(grab) = drag_grab(target, axis) else {
                            return;
                        };
                        if event.pressed_button.is_none() {
                            // Button released off-window (no mouse-up): drop the drag.
                            clear_drag_grab(target, axis);
                            return;
                        }
                        let max_offset = f32::from(handle.max_offset().along(axis));
                        let new = click_to_offset(
                            f32::from(track_bounds.size.along(axis)),
                            f32::from(thumb_bounds.size.along(axis)),
                            f32::from(track_bounds.origin.along(axis)),
                            f32::from(event.position.along(axis)),
                            max_offset,
                            grab,
                        );
                        set_handle_offset(&handle, axis, new);
                        window.refresh();
                        cx.stop_propagation();
                    }
                });

                window.on_mouse_event(move |_event: &MouseUpEvent, phase, window, _cx| {
                    if phase != capture_phase {
                        return;
                    }
                    if drag_grab(target, axis).is_some() {
                        clear_drag_grab(target, axis);
                        window.refresh();
                    }
                });
            });
        },
    )
    .size_full()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 0.01;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    #[test]
    fn not_scrollable_when_max_offset_zero() {
        assert_eq!(thumb_geometry(100.0, 100.0, 0.0, 0.0, 20.0), None);
    }

    #[test]
    fn not_scrollable_when_content_fits() {
        // viewport >= content (max_offset 0) → no thumb.
        assert_eq!(thumb_geometry(200.0, 200.0, 0.0, 0.0, 20.0), None);
    }

    #[test]
    fn thumb_proportional_to_visible_fraction() {
        // viewport 100, content 200 (max_offset 100) → thumb is half the track.
        let (len, pos) = thumb_geometry(100.0, 100.0, 100.0, 0.0, 0.0).unwrap();
        assert!(approx(len, 50.0), "len = {len}");
        assert!(approx(pos, 0.0), "pos = {pos}");
    }

    #[test]
    fn thumb_at_bottom_when_fully_scrolled() {
        // Fully scrolled: current_offset == -max_offset → progress 1.
        let (len, pos) = thumb_geometry(100.0, 100.0, 100.0, -100.0, 0.0).unwrap();
        assert!(approx(len, 50.0));
        assert!(approx(pos, 50.0), "pos = {pos}"); // track_len - thumb_len
    }

    #[test]
    fn progress_clamped_to_range() {
        // Over-scrolled offset is clamped, never past the track end.
        let (_, pos) = thumb_geometry(100.0, 100.0, 100.0, -500.0, 0.0).unwrap();
        assert!(approx(pos, 50.0));
        let (_, pos_neg) = thumb_geometry(100.0, 100.0, 100.0, 50.0, 0.0).unwrap();
        assert!(approx(pos_neg, 0.0));
    }

    #[test]
    fn min_thumb_enforced() {
        // Tiny visible fraction would give a sub-min thumb; clamp up to min.
        let (len, _) = thumb_geometry(100.0, 10.0, 990.0, 0.0, 20.0).unwrap();
        assert!(approx(len, 20.0), "len = {len}");
    }

    #[test]
    fn none_when_min_thumb_exceeds_track() {
        // min_thumb bigger than the track can't produce a movable thumb.
        assert_eq!(thumb_geometry(15.0, 10.0, 100.0, 0.0, 20.0), None);
    }

    #[test]
    fn track_click_center_jump() {
        // Click at the far end of a 100px track, thumb 50px, max_offset 100.
        // grab = thumb_len/2 = 25; thumb_start = (100 - 0 - 25) clamped to [0,50] = 50;
        // percentage = 1 → offset = -100.
        let off = click_to_offset(100.0, 50.0, 0.0, 100.0, 100.0, 25.0);
        assert!(approx(off, -100.0), "off = {off}");
    }

    #[test]
    fn track_click_clamped_both_ends() {
        // Click before the track start clamps to 0 offset.
        let off = click_to_offset(100.0, 50.0, 0.0, -50.0, 100.0, 25.0);
        assert!(approx(off, 0.0), "off = {off}");
        // Click way past the end clamps to -max_offset.
        let off2 = click_to_offset(100.0, 50.0, 0.0, 999.0, 100.0, 25.0);
        assert!(approx(off2, -100.0), "off2 = {off2}");
    }

    #[test]
    fn drag_preserves_grab_point() {
        // Dragging with grab=10: pointer at 60 in a 100px track, thumb 50px.
        // thumb_start = (60 - 0 - 10) = 50 (clamped to [0,50]); percentage 1 → -100.
        let off = click_to_offset(100.0, 50.0, 0.0, 60.0, 100.0, 10.0);
        assert!(approx(off, -100.0), "off = {off}");
        // Mid-drag: pointer at 35, grab 10 → thumb_start 25 → 0.5 → -50.
        let mid = click_to_offset(100.0, 50.0, 0.0, 35.0, 100.0, 10.0);
        assert!(approx(mid, -50.0), "mid = {mid}");
    }

    #[test]
    fn click_respects_track_origin() {
        // Track starting at window x=200; pointer at 250, thumb 50, grab 25.
        // thumb_start = (250 - 200 - 25) = 25 → 0.5 → -50.
        let off = click_to_offset(100.0, 50.0, 200.0, 250.0, 100.0, 25.0);
        assert!(approx(off, -50.0), "off = {off}");
    }

    #[test]
    fn thumb_rect_runs_along_axis_and_insets_cross() {
        use gpui::{point, size};
        // Vertical track: 8 wide, 200 tall, at (10, 20).
        let track = Bounds::new(point(px(10.0), px(20.0)), size(px(8.0), px(200.0)));
        let r = thumb_rect(track, Axis::Vertical, 50.0, 30.0, 1.0);
        // length/position run along Y; X is inset by `inset` on each side.
        assert_eq!(r.origin, point(px(11.0), px(50.0)));
        assert_eq!(r.size, size(px(6.0), px(50.0)));
    }

    #[test]
    fn thumb_rect_horizontal_is_vertical_transposed() {
        use gpui::{point, size};
        // A square track so the two axes are directly comparable by transposition.
        let track = Bounds::new(point(px(10.0), px(10.0)), size(px(100.0), px(100.0)));
        let v = thumb_rect(track, Axis::Vertical, 40.0, 25.0, 2.0);
        let h = thumb_rect(track, Axis::Horizontal, 40.0, 25.0, 2.0);
        // Horizontal is the vertical rect with X/Y (and width/height) swapped —
        // this is what actually exercises the axis mapping.
        assert_eq!(h.origin.x, v.origin.y);
        assert_eq!(h.origin.y, v.origin.x);
        assert_eq!(h.size.width, v.size.height);
        assert_eq!(h.size.height, v.size.width);
    }
}
