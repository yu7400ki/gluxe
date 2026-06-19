//! Anchor positioning — the native side of gluxe's floating UI.
//!
//! A node declared with `anchorName` is wrapped in a layout-transparent
//! [`Measured`] element that records its last-painted window-space bounds into
//! `ANCHOR_BOUNDS` each prepaint. A `floating` element is wrapped (via
//! [`wrap_floating`]) in `deferred(anchored(...))` positioned against that
//! anchor's bounds, so GPUI handles overflow snapping and stacking.
//!
//! The two registries are owned here; `render` mutates them only through the
//! accessor functions below (`register_name` / `clear_name_for` / `drop_bounds` /
//! `evict` / `clear` / `has_bounds`), so all anchor bookkeeping stays in one place.

use std::cell::RefCell;

use gpui::{
    Anchor, AnyElement, App, Bounds, Element, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, Point, Window, anchored, deferred, point, prelude::*, px,
};
use rustc_hash::FxHashMap;

use crate::model::{ElementId, FloatingAlign, FloatingSide, FloatingSpec};

thread_local! {
    /// Maps each `anchorName` to its element id. Re-registered every render of the
    /// anchor node; on duplicate names the last writer wins.
    static ANCHOR_NAMES: RefCell<FxHashMap<String, ElementId>> = RefCell::new(FxHashMap::default());
    /// Last painted window-space bounds of each anchor node, recorded by `Measured`
    /// each prepaint and read (one frame later) by floating elements bound to it.
    static ANCHOR_BOUNDS: RefCell<FxHashMap<ElementId, Bounds<Pixels>>> =
        RefCell::new(FxHashMap::default());
}

/// Whether `id` currently has recorded anchor bounds (i.e. it was painted as an
/// anchor on a previous frame). Lets `render` keep ordinary nodes off the anchor
/// path while still catching a node that just stopped being an anchor.
pub(super) fn has_bounds(id: ElementId) -> bool {
    ANCHOR_BOUNDS.with(|m| m.borrow().contains_key(&id))
}

/// Register `id` as the anchor for `name` (last writer wins on duplicate names).
pub(super) fn register_name(name: String, id: ElementId) {
    ANCHOR_NAMES.with(|m| {
        m.borrow_mut().insert(name, id);
    });
}

/// Drop any name→id mapping pointing at `id` — a node that is no longer an anchor,
/// or whose `anchorName` changed. Called on every render of a (formerly) anchor node.
pub(super) fn clear_name_for(id: ElementId) {
    ANCHOR_NAMES.with(|m| m.borrow_mut().retain(|_, &mut v| v != id));
}

/// Drop `id`'s frozen bounds so nothing reads stale geometry.
pub(super) fn drop_bounds(id: ElementId) {
    ANCHOR_BOUNDS.with(|m| {
        m.borrow_mut().remove(&id);
    });
}

/// Evict all anchor state for a removed node (`removed_nodes` cleanup).
pub(super) fn evict(id: ElementId) {
    drop_bounds(id);
    clear_name_for(id);
}

/// Clear all anchor state (dev-mode full reload — old tree ids never reappear).
#[cfg(debug_assertions)]
pub(super) fn clear() {
    ANCHOR_NAMES.with(|m| m.borrow_mut().clear());
    ANCHOR_BOUNDS.with(|m| m.borrow_mut().clear());
}

/// A layout-transparent wrapper that records its child's painted bounds into
/// `ANCHOR_BOUNDS` each prepaint. Wraps any node declared with `anchorName` so
/// floating elements bound to that name can position against it (read one frame
/// later, like Zed's `PopoverMenu`). It forwards the child's `LayoutId` and paints
/// only the child, contributing nothing of its own to layout or paint.
pub(super) struct Measured {
    pub(super) id: ElementId,
    pub(super) child: AnyElement,
}

impl Element for Measured {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        // Floating elements bound to this anchor read its bounds one frame late (at their
        // own build time). When the bounds first appear or move, nudge a redraw so those
        // elements re-render at the new position. Guarded by an actual change so a stable
        // anchor never schedules a redraw (no repaint loop).
        let changed =
            ANCHOR_BOUNDS.with(|m| m.borrow_mut().insert(self.id, bounds) != Some(bounds));
        if changed {
            window.request_animation_frame();
        }
        self.child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        _prepaint: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}

impl IntoElement for Measured {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

/// Map a floating `side`/`align` to the (trigger-corner, floating-corner) pair:
/// the floating element's `floating` corner is placed at the anchor's `attach` corner.
fn floating_corners(side: FloatingSide, align: FloatingAlign) -> (Anchor, Anchor) {
    use Anchor::{
        BottomCenter, BottomLeft, BottomRight, LeftCenter, RightCenter, TopCenter, TopLeft,
        TopRight,
    };
    use FloatingAlign::{Center, End, Start};
    use FloatingSide::{Bottom, Left, Right, Top};
    match (side, align) {
        (Bottom, Start) => (BottomLeft, TopLeft),
        (Bottom, Center) => (BottomCenter, TopCenter),
        (Bottom, End) => (BottomRight, TopRight),
        (Top, Start) => (TopLeft, BottomLeft),
        (Top, Center) => (TopCenter, BottomCenter),
        (Top, End) => (TopRight, BottomRight),
        (Right, Start) => (TopRight, TopLeft),
        (Right, Center) => (RightCenter, LeftCenter),
        (Right, End) => (BottomRight, BottomLeft),
        (Left, Start) => (TopLeft, TopRight),
        (Left, Center) => (LeftCenter, RightCenter),
        (Left, End) => (BottomLeft, BottomRight),
    }
}

/// The gap vector applied along the `side` direction (pushes the floating element
/// away from the anchor).
fn side_offset(side: FloatingSide, offset: Pixels) -> Point<Pixels> {
    match side {
        FloatingSide::Top => point(px(0.0), -offset),
        FloatingSide::Bottom => point(px(0.0), offset),
        FloatingSide::Left => point(-offset, px(0.0)),
        FloatingSide::Right => point(offset, px(0.0)),
    }
}

/// Wrap a floating element in `deferred(anchored(...))`, positioned against the
/// named anchor's last-recorded bounds. `anchored` measures the floating element's
/// own size and snaps it inside the window on overflow; `deferred` lifts it above
/// in-flow content and outside overflow clipping. Until the anchor has been measured
/// (normally only the first frame, since the anchor pre-exists), it falls back to
/// `anchored`'s natural placement.
pub(super) fn wrap_floating(
    spec: &FloatingSpec,
    rem_size: Pixels,
    child: AnyElement,
) -> AnyElement {
    // `offset`/`margin` accept px/rem (resolved to absolute px here); `%`/`auto` are
    // meaningless for a gap and fall back to 0.
    let offset = spec
        .offset
        .to_absolute()
        .map_or(px(0.0), |a| a.to_pixels(rem_size));
    let margin = spec
        .margin
        .to_absolute()
        .map_or(px(0.0), |a| a.to_pixels(rem_size));

    let (attach, floating_anchor) = floating_corners(spec.side, spec.align);
    let position = ANCHOR_NAMES
        .with(|m| m.borrow().get(&spec.anchor).copied())
        .and_then(|anchor_id| ANCHOR_BOUNDS.with(|m| m.borrow().get(&anchor_id).copied()))
        .map(|bounds| bounds.corner(attach) + side_offset(spec.side, offset));

    let mut anchored_el = anchored().anchor(floating_anchor);
    if let Some(position) = position {
        anchored_el = anchored_el.position(position);
    }
    // Always snap inside the window on overflow, keeping `margin` from the edge.
    // We deliberately avoid anchored's `SwitchAnchor` (flip): it mirrors the floating
    // corner about the position point without knowing the anchor's size, so it would
    // overlap a sized anchor. `margin` 0 is equivalent to a plain `snap_to_window()`.
    anchored_el = anchored_el.snap_to_window_with_margin(margin);
    let anchored_el = anchored_el.child(child);

    let mut deferred_el = deferred(anchored_el);
    if let Some(priority) = spec.priority {
        deferred_el = deferred_el.with_priority(priority as usize);
    }
    deferred_el.into_any_element()
}

#[cfg(test)]
mod tests {
    use gpui::{Anchor, point, px};

    use super::{floating_corners, side_offset};
    use crate::model::{FloatingAlign, FloatingSide};

    #[test]
    fn floating_corners_all_combinations() {
        use Anchor::{
            BottomCenter, BottomLeft, BottomRight, LeftCenter, RightCenter, TopCenter, TopLeft,
            TopRight,
        };
        use FloatingAlign::{Center, End, Start};
        use FloatingSide::{Bottom, Left, Right, Top};

        // Characterization: exactly mirrors the current match arms.
        assert_eq!(floating_corners(Bottom, Start), (BottomLeft, TopLeft));
        assert_eq!(floating_corners(Bottom, Center), (BottomCenter, TopCenter));
        assert_eq!(floating_corners(Bottom, End), (BottomRight, TopRight));
        assert_eq!(floating_corners(Top, Start), (TopLeft, BottomLeft));
        assert_eq!(floating_corners(Top, Center), (TopCenter, BottomCenter));
        assert_eq!(floating_corners(Top, End), (TopRight, BottomRight));
        assert_eq!(floating_corners(Right, Start), (TopRight, TopLeft));
        assert_eq!(floating_corners(Right, Center), (RightCenter, LeftCenter));
        assert_eq!(floating_corners(Right, End), (BottomRight, BottomLeft));
        assert_eq!(floating_corners(Left, Start), (TopLeft, TopRight));
        assert_eq!(floating_corners(Left, Center), (LeftCenter, RightCenter));
        assert_eq!(floating_corners(Left, End), (BottomLeft, BottomRight));
    }

    #[test]
    fn side_offset_axis_and_sign() {
        let off = px(8.0);
        // Top pushes up (negative y); Bottom pushes down (positive y).
        assert_eq!(side_offset(FloatingSide::Top, off), point(px(0.0), -off));
        assert_eq!(side_offset(FloatingSide::Bottom, off), point(px(0.0), off));
        // Left pushes left (negative x); Right pushes right (positive x).
        assert_eq!(side_offset(FloatingSide::Left, off), point(-off, px(0.0)));
        assert_eq!(side_offset(FloatingSide::Right, off), point(off, px(0.0)));
    }
}
