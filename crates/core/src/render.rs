use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
};

use gpui::{
    Anchor, AnyElement, App, Bounds, Context as GpuiContext, Element, Entity, GlobalElementId,
    InspectorElementId, IntoElement, KeyDownEvent, LayoutId, MouseButton, Pixels, Point, Render,
    Subscription, Window, WindowControlArea, anchored, deferred, div, img, point, prelude::*, px,
};
use rustc_hash::FxHashMap;
use url::Url;

use crate::{
    component::{self, NativeRenderContext},
    model::{
        ApplyOutcome, ElementId, ElementKind, Events, FloatingAlign, FloatingSide, FloatingSpec,
        OverflowMode, Props, StyleFields,
    },
    state::{
        dispatch_key_event, dispatch_mouse_event, dispatch_simple_event, focus_handle,
        notify_text_input_entity, scroll_handle, text_input_entity, try_autofocus, with_tree,
    },
    style::apply_style_props,
};

// ---------------------------------------------------------------------------
// Per-node renderer
// ---------------------------------------------------------------------------

thread_local! {
    static NODE_VIEWS: RefCell<FxHashMap<ElementId, Entity<NodeView>>> =
        RefCell::new(FxHashMap::default());
    /// Whether a left-button press started inside a `WindowControlArea::Drag` region
    /// (non-Windows). Set on mouse-down, cleared when drag starts or button releases.
    static DRAG_SHOULD_MOVE: Cell<bool> = Cell::new(false);
    /// Set when `start_window_move()` is actually called; prevents a drag-initiated
    /// click from also triggering the double-click zoom action.
    static WINDOW_MOVE_STARTED: Cell<bool> = Cell::new(false);
    /// Maps each `anchorName` to its element id. Re-registered every render of the
    /// anchor node; on duplicate names the last writer wins.
    static ANCHOR_NAMES: RefCell<FxHashMap<String, ElementId>> = RefCell::new(FxHashMap::default());
    /// Last painted window-space bounds of each anchor node, recorded by `Measured`
    /// each prepaint and read (one frame later) by floating elements bound to it.
    static ANCHOR_BOUNDS: RefCell<FxHashMap<ElementId, Bounds<Pixels>>> =
        RefCell::new(FxHashMap::default());
    /// `onFocus`/`onBlur` subscriptions per element. Rebuilt when the handler set
    /// changes; dropped on `DetachDeleted` / dev reload.
    static FOCUS_SUBSCRIPTIONS: RefCell<FxHashMap<ElementId, FocusSubs>> =
        RefCell::new(FxHashMap::default());
}

/// `on_focus`/`on_blur` subscriptions tagged with the `(focus, blur)` flags they
/// were built from, so `attach_focus!` can rebuild on change instead of freezing.
struct FocusSubs {
    focus: bool,
    blur: bool,
    _subs: Vec<Subscription>, // dropping cancels
}

/// Drop the focus subscriptions for `id` (called when its node is detached).
pub(crate) fn drop_focus_subscriptions(id: ElementId) {
    FOCUS_SUBSCRIPTIONS.with(|subs| {
        subs.borrow_mut().remove(&id);
    });
}

/// Drop all focus subscriptions (dev-mode full reload — old ids never reappear).
#[cfg(debug_assertions)]
pub(crate) fn clear_focus_subscriptions() {
    FOCUS_SUBSCRIPTIONS.with(|subs| subs.borrow_mut().clear());
}

/// Attach JS event listeners to any `Stateful<T>` (`T: StatefulInteractiveElement`).
///
/// A macro rather than a generic helper because `Stateful<Div>` and `Stateful<Img>`
/// share trait impls but have no common nameable ancestor in stable Rust.
macro_rules! attach_events {
    ($s:expr, $eid:expr, $ev:expr) => {{
        let mut s = $s;
        let eid: ElementId = $eid;
        let ev: Events = $ev;
        if ev.click {
            s = s.on_click(move |e, _, _| {
                let p = e.position();
                dispatch_mouse_event(eid, "click", f32::from(p.x), f32::from(p.y));
            });
        }
        if ev.mousedown {
            s = s.on_mouse_down(MouseButton::Left, move |e, _, _| {
                dispatch_mouse_event(
                    eid,
                    "mousedown",
                    f32::from(e.position.x),
                    f32::from(e.position.y),
                );
            });
        }
        if ev.mouseup {
            s = s.on_mouse_up(MouseButton::Left, move |e, _, _| {
                dispatch_mouse_event(
                    eid,
                    "mouseup",
                    f32::from(e.position.x),
                    f32::from(e.position.y),
                );
            });
        }
        if ev.mousemove {
            s = s.on_mouse_move(move |e, _, _| {
                dispatch_mouse_event(
                    eid,
                    "mousemove",
                    f32::from(e.position.x),
                    f32::from(e.position.y),
                );
            });
        }
        if ev.mouseenter || ev.mouseleave {
            let (want_enter, want_leave) = (ev.mouseenter, ev.mouseleave);
            s = s.on_hover(move |is_hovered, window, _| {
                let p = window.mouse_position();
                if *is_hovered {
                    if want_enter {
                        dispatch_mouse_event(eid, "mouseenter", f32::from(p.x), f32::from(p.y));
                    }
                } else if want_leave {
                    dispatch_mouse_event(eid, "mouseleave", f32::from(p.x), f32::from(p.y));
                }
            });
        }
        if ev.keydown {
            s = s.on_key_down(move |e: &KeyDownEvent, _, _| {
                let ks = &e.keystroke;
                let m = ks.modifiers;
                dispatch_key_event(eid, &ks.key, m.shift, m.control, m.alt, m.platform);
            });
        }
        s
    }};
}

/// Attach window control area behaviour to a `Stateful<T>` (`Div` or `Img`).
///
/// Same macro-not-generic rationale as `attach_events!`.
///
/// On **Windows**, `window_control_area(area)` is sufficient — the OS handles
/// drag-move, Snap Layouts, and button actions natively via NCHITTEST.
/// On macOS/Linux, the hit-test registration is still applied, but the actual
/// actions are driven by mouse-event handlers in the `#[cfg(not(target_os = "windows"))]`
/// block below.
///
/// For button areas (Close / Max / Min), `.occlude()` is applied so the button
/// wins the hit-test over any containing `Drag` region (mirrors Zed's titlebar impl).
macro_rules! attach_window_control {
    ($s:expr, $area:expr) => {{
        let mut s = $s;
        let area: WindowControlArea = $area;
        // Register the GPUI hit-test region (drives NCHITTEST on Windows;
        // marks compositor/WM regions on other platforms).
        s = s.window_control_area(area);
        // Explicit arms: a new WindowControlArea variant upstream → compile error here.
        match area {
            WindowControlArea::Drag => {
                // Non-Windows: implement drag-move by setting a flag on mouse-down
                // and calling `start_window_move()` on the first mouse-move.
                // The flag is cleared on mouse-up/out so a click without a move is a no-op.
                // Double-click maximises — only if no window move occurred (`WINDOW_MOVE_STARTED`).
                #[cfg(not(target_os = "windows"))]
                {
                    s = s
                        .on_mouse_down(MouseButton::Left, |_, _, _| {
                            DRAG_SHOULD_MOVE.set(true);
                            WINDOW_MOVE_STARTED.set(false);
                        })
                        .on_mouse_up(MouseButton::Left, |_, _, _| {
                            DRAG_SHOULD_MOVE.set(false);
                        })
                        .on_mouse_up_out(MouseButton::Left, |_, _, _| {
                            DRAG_SHOULD_MOVE.set(false);
                        })
                        .on_mouse_down_out(|_, _, _| {
                            DRAG_SHOULD_MOVE.set(false);
                        })
                        .on_mouse_move(|_, window, _| {
                            if DRAG_SHOULD_MOVE.replace(false) {
                                window.start_window_move();
                                WINDOW_MOVE_STARTED.set(true);
                            }
                        })
                        .on_click(|e, window, _| {
                            if e.click_count() == 2 && !WINDOW_MOVE_STARTED.get() {
                                // macOS: honour System Preferences "titlebar double-click" setting.
                                // Linux/other: zoom (maximise/restore).
                                #[cfg(target_os = "macos")]
                                window.titlebar_double_click();
                                #[cfg(not(target_os = "macos"))]
                                window.zoom_window();
                            }
                        });
                }
            }
            WindowControlArea::Close | WindowControlArea::Max | WindowControlArea::Min => {
                // `.occlude()` makes this button win the hit-test over any enclosing Drag region.
                s = s.occlude();
                #[cfg(not(target_os = "windows"))]
                {
                    // Stop propagation so button events don't trigger drag logic on a parent Drag.
                    s = s
                        .on_mouse_down(MouseButton::Left, |_, _, cx| {
                            cx.stop_propagation();
                        })
                        .on_mouse_move(|_, _, cx| {
                            cx.stop_propagation();
                        })
                        .on_click(move |_, window, cx| {
                            cx.stop_propagation();
                            match area {
                                WindowControlArea::Close => window.remove_window(),
                                WindowControlArea::Max => window.zoom_window(),
                                WindowControlArea::Min => window.minimize_window(),
                                WindowControlArea::Drag => unreachable!(),
                            }
                        });
                }
            }
        }
        s
    }};
}

/// Apply focus management to a `Stateful<T>` (`Div`/`Img`): `track_focus`,
/// `tabIndex`/`tabStop`, `_focus`/`_focusVisible`, `autoFocus`, `onFocus`/`onBlur`.
///
/// Macro (not generic) for the same reason as `attach_events!`. `$cx` must be a
/// `Context<NodeView>`. Call only when `props.is_focusable()`.
macro_rules! attach_focus {
    ($s:expr, $eid:expr, $props:expr, $window:expr, $cx:expr) => {{
        let mut s = $s;
        let eid: ElementId = $eid;
        let props: &Props = $props;
        // Tab order goes ON THE FocusHandle: with `track_focus`, gpui reads
        // tab_index/tab_stop from the handle and ignores element-level setters.
        // HTML semantics: tabIndex >= 0 is a tab stop; < 0 is focusable but skipped.
        let mut handle = focus_handle(eid, $cx);
        if let Some(idx) = props.tab_index {
            handle = handle.tab_index(idx as isize);
        }
        let tab_stop = props
            .tab_stop
            .unwrap_or_else(|| props.tab_index.is_some_and(|i| i >= 0));
        handle = handle.tab_stop(tab_stop);
        s = s.track_focus(&handle);
        if let Some(fs) = &props.focus_style {
            let fs: &StyleFields = fs.as_ref();
            s = s.focus(|style| apply_style_props(style, fs));
        }
        if let Some(fvs) = &props.focus_visible_style {
            let fvs: &StyleFields = fvs.as_ref();
            s = s.focus_visible(|style| apply_style_props(style, fvs));
        }
        if props.autofocus {
            try_autofocus(eid, $window, $cx);
        }
        // Rebuild on_focus/on_blur only when the (focus, blur) set changes, so a
        // handler added/removed later takes effect (no frozen set, no leak).
        let want_focus = props.events.focus;
        let want_blur = props.events.blur;
        let changed = FOCUS_SUBSCRIPTIONS.with(|subs| {
            subs.borrow()
                .get(&eid)
                .map_or(want_focus || want_blur, |s| {
                    s.focus != want_focus || s.blur != want_blur
                })
        });
        if changed {
            if want_focus || want_blur {
                let mut list: Vec<Subscription> = Vec::new();
                if want_focus {
                    list.push($cx.on_focus(&handle, $window, move |_, _, _| {
                        dispatch_simple_event(eid, "focus");
                    }));
                }
                if want_blur {
                    list.push($cx.on_blur(&handle, $window, move |_, _, _| {
                        dispatch_simple_event(eid, "blur");
                    }));
                }
                FOCUS_SUBSCRIPTIONS.with(|subs| {
                    subs.borrow_mut().insert(
                        eid,
                        FocusSubs {
                            focus: want_focus,
                            blur: want_blur,
                            _subs: list,
                        },
                    );
                });
            } else {
                // Both handlers removed — drop the subscriptions.
                FOCUS_SUBSCRIPTIONS.with(|subs| {
                    subs.borrow_mut().remove(&eid);
                });
            }
        }
        s
    }};
}

/// Build a styled `Div` (or `Stateful<Div>` when needed) from `props`.
///
/// Same macro-not-generic rationale as `attach_events!`. A `Stateful<Div>`
/// (via `.id()`) is used when any of: hover/active pseudo-selectors, JS event
/// handlers, scroll, focus (any focus-related prop), or windowControlArea is present.
macro_rules! build_div_with_pseudo {
    ($id:expr, $props:expr, $children:expr, $window:expr, $cx:expr) => {{
        let mut div = apply_style_props(div(), &$props.style);
        if let Some(hover_props) = &$props.hover {
            let hover_props: &StyleFields = hover_props.as_ref();
            div = div.hover(|style| apply_style_props(style, hover_props));
        }
        if $props.hover.is_some()
            || $props.active.is_some()
            || $props.events.any()
            || $props.style.scrolls()
            || $props.is_focusable()
            || $props.window_control_area.is_some()
        {
            let eid: ElementId = $id;
            let mut stateful = div.id(eid as usize);
            if let Some(active_props) = &$props.active {
                let active_props: &StyleFields = active_props.as_ref();
                stateful = stateful.active(|style| apply_style_props(style, active_props));
            }
            // Focus management (track_focus/tabIndex/_focus/autoFocus/onFocus/onBlur).
            if $props.is_focusable() {
                stateful = attach_focus!(stateful, eid, &$props, $window, $cx);
            }
            // overflow_*_scroll requires `.id()` (StatefulInteractiveElement).
            if matches!($props.style.overflow_x, Some(OverflowMode::Scroll)) {
                stateful = stateful.overflow_x_scroll();
            }
            if matches!($props.style.overflow_y, Some(OverflowMode::Scroll)) {
                stateful = stateful.overflow_y_scroll();
            }
            if $props.style.scrolls() {
                stateful = stateful.track_scroll(&scroll_handle(eid));
            }
            // Must come after `.id()`: the non-Windows Drag handler uses `on_click`
            // which requires Stateful.
            if let Some(area) = $props.window_control_area {
                stateful = attach_window_control!(stateful, area);
            }
            let stateful = attach_events!(stateful, eid, $props.events);
            stateful.children($children).into_any_element()
        } else {
            div.children($children).into_any_element()
        }
    }};
}

fn node_view_entity(id: ElementId, cx: &mut App) -> Entity<NodeView> {
    NODE_VIEWS.with(|views| {
        let existing = views.borrow().get(&id).cloned();
        if let Some(entity) = existing {
            return entity;
        }
        let entity = cx.new(|_| NodeView { id });
        views.borrow_mut().insert(id, entity.clone());
        entity
    })
}

fn get_node_view_entity(id: ElementId) -> Option<Entity<NodeView>> {
    NODE_VIEWS.with(|views| views.borrow().get(&id).cloned())
}

fn remove_node_view(id: ElementId) {
    NODE_VIEWS.with(|views| {
        views.borrow_mut().remove(&id);
    });
}

/// Drop all cached `NodeView` entities and reset drag flags (dev-mode full reload:
/// old tree ids never reappear so entities would leak; an in-progress drag must
/// not carry over to the new tree).
#[cfg(debug_assertions)]
pub(crate) fn clear_node_views() {
    NODE_VIEWS.with(|views| views.borrow_mut().clear());
    DRAG_SHOULD_MOVE.set(false);
    WINDOW_MOVE_STARTED.set(false);
    ANCHOR_NAMES.with(|m| m.borrow_mut().clear());
    ANCHOR_BOUNDS.with(|m| m.borrow_mut().clear());
}

fn render_child(id: ElementId, cx: &mut App) -> Option<AnyElement> {
    let raw_text = with_tree(|tree| {
        tree.nodes.get(&id).and_then(|element| {
            if matches!(&element.kind, ElementKind::RawText) {
                Some(element.text.clone().unwrap_or_default())
            } else {
                None
            }
        })
    });
    if let Some(text) = raw_text {
        return Some(text.into_any_element());
    }

    let exists = with_tree(|tree| tree.nodes.contains_key(&id));
    exists.then(|| node_view_entity(id, cx).into_any_element())
}

fn is_text_input_node(id: ElementId) -> bool {
    with_tree(|tree| {
        tree.nodes
            .get(&id)
            .is_some_and(|element| matches!(&element.kind, ElementKind::TextInput))
    })
}

// ---------------------------------------------------------------------------
// Anchor positioning
// ---------------------------------------------------------------------------

/// A layout-transparent wrapper that records its child's painted bounds into
/// `ANCHOR_BOUNDS` each prepaint. Wraps any node declared with `anchorName` so
/// floating elements bound to that name can position against it (read one frame
/// later, like Zed's `PopoverMenu`). It forwards the child's `LayoutId` and paints
/// only the child, contributing nothing of its own to layout or paint.
struct Measured {
    id: ElementId,
    child: AnyElement,
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
fn wrap_floating(spec: &FloatingSpec, rem_size: Pixels, child: AnyElement) -> AnyElement {
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

pub(crate) struct NodeView {
    id: ElementId,
}

impl Render for NodeView {
    fn render(&mut self, window: &mut Window, cx: &mut GpuiContext<Self>) -> impl IntoElement {
        let id = self.id;
        let element = with_tree(|tree| tree.nodes.get(&id).cloned());
        let Some(mut element) = element else {
            return div().into_any_element();
        };
        // Overlay in-flight style transitions onto the cloned style so every
        // branch below (View/Text/Image/Native) renders interpolated values.
        crate::anim::overlay(id, &mut element.props.style);

        let mut el = match element.kind.clone() {
            ElementKind::RawText => {
                Some(element.text.clone().unwrap_or_default().into_any_element())
            }
            ElementKind::View => {
                let children: Vec<AnyElement> = element
                    .children
                    .iter()
                    .filter_map(|&child_id| render_child(child_id, cx))
                    .collect();
                Some(build_div_with_pseudo!(
                    id,
                    element.props,
                    children,
                    window,
                    cx
                ))
            }
            ElementKind::Text => {
                // React may split an interpolated string (e.g. `Count: {count}`) into
                // multiple adjacent RawText children; concatenate them on one line.
                let mut children: Vec<AnyElement> = Vec::new();
                let mut text_buf = String::new();
                for &child_id in &element.children {
                    let raw_text = with_tree(|tree| {
                        tree.nodes.get(&child_id).and_then(|child_element| {
                            if matches!(&child_element.kind, ElementKind::RawText) {
                                Some(child_element.text.clone().unwrap_or_default())
                            } else {
                                None
                            }
                        })
                    });
                    if let Some(raw_text) = raw_text {
                        text_buf.push_str(&raw_text);
                    } else {
                        if !text_buf.is_empty() {
                            children.push(std::mem::take(&mut text_buf).into_any_element());
                        }
                        if let Some(any) = render_child(child_id, cx) {
                            children.push(any);
                        }
                    }
                }
                if !text_buf.is_empty() {
                    children.push(text_buf.into_any_element());
                }
                Some(build_div_with_pseudo!(
                    id,
                    element.props,
                    children,
                    window,
                    cx
                ))
            }
            ElementKind::Image => {
                if let Some(src) = &element.props.src {
                    // URL scheme routing:
                    //   asset://   — gluxe bundler prefix → strip → Resource::Embedded
                    //   http(s):// — Resource::Uri (requires `http` feature; silent fail without it)
                    //   file://    — parsed via `url` crate for Windows paths + percent-encoding → Resource::Path
                    //   (bare)     — treated as a local path; bundled assets always get `asset://`,
                    //               so bare paths are always developer-authored refs.
                    let mut image = if let Some(rest) = src.strip_prefix("asset://") {
                        img(rest.to_string())
                    } else if src.starts_with("http://") || src.starts_with("https://") {
                        img(src.to_string())
                    } else {
                        img(local_image_path(src))
                    };
                    if let Some(v) = element.props.style.width {
                        image = image.w(v.to_length());
                    }
                    if let Some(v) = element.props.style.height {
                        image = image.h(v.to_length());
                    }
                    if let Some(v) = element.props.style.border_radius {
                        if let Some(a) = v.to_absolute() {
                            image = image.rounded(a);
                        }
                    }

                    // `.id()` → `Stateful<Img>` (StatefulInteractiveElement, same as Stateful<Div>).
                    if element.props.events.any()
                        || element.props.is_focusable()
                        || element.props.window_control_area.is_some()
                    {
                        let mut stateful = image.id(id as usize);
                        if element.props.is_focusable() {
                            stateful = attach_focus!(stateful, id, &element.props, window, cx);
                        }
                        if let Some(area) = element.props.window_control_area {
                            stateful = attach_window_control!(stateful, area);
                        }
                        let stateful = attach_events!(stateful, id, element.props.events);
                        Some(stateful.into_any_element())
                    } else {
                        Some(image.into_any_element())
                    }
                } else {
                    Some(build_div_with_pseudo!(
                        id,
                        element.props,
                        Vec::<AnyElement>::new(),
                        window,
                        cx
                    ))
                }
            }
            ElementKind::TextInput => Some(text_input_entity(id, cx).into_any_element()),
            ElementKind::Native(name) => {
                // Render reconciler children + pass them with raw props to the host-registered
                // component function; wrap its output in a `Stateful<Div>` (same as `<View>`).
                let children: Vec<AnyElement> = element
                    .children
                    .iter()
                    .filter_map(|&child_id| render_child(child_id, cx))
                    .collect();
                let null = serde_json::Value::Null;
                let props = element.props.raw.as_ref().unwrap_or(&null);
                // `name` is always registered (registry is immutable after startup);
                // the `unwrap_or_default` is defensive.
                let inner = component::render(&name, NativeRenderContext { props, children })
                    .unwrap_or_default();
                Some(build_div_with_pseudo!(id, element.props, inner, window, cx))
            }
        }
        .unwrap_or_else(|| div().into_any_element());

        // An anchor node records its bounds; a floating node lifts itself into an
        // anchored overlay. Applied to the final element regardless of kind.
        //
        // Re-register on every render so a dynamic `anchorName` change or removal —
        // which arrives via UpdateProps, not node removal, so `removed_nodes` never
        // sees it — can't leave a stale name→id mapping or a frozen bounds entry
        // behind. The guard keeps ordinary (never-anchor) nodes off this path.
        if element.props.anchor_name.is_some()
            || ANCHOR_BOUNDS.with(|m| m.borrow().contains_key(&id))
        {
            ANCHOR_NAMES.with(|m| m.borrow_mut().retain(|_, &mut v| v != id));
            if let Some(name) = &element.props.anchor_name {
                ANCHOR_NAMES.with(|m| {
                    m.borrow_mut().insert(name.clone(), id);
                });
                el = Measured { id, child: el }.into_any_element();
            } else {
                // No longer an anchor: drop the frozen bounds so nothing reads them.
                ANCHOR_BOUNDS.with(|m| {
                    m.borrow_mut().remove(&id);
                });
            }
        }
        if let Some(spec) = &element.props.floating {
            el = wrap_floating(spec, window.rem_size(), el);
        }
        el
    }
}

// ---------------------------------------------------------------------------
// GPUI root view
// ---------------------------------------------------------------------------

// Global Tab navigation actions. Declared here (the root view consumes them via
// `on_action`); the matching key bindings are registered once in `lib.rs`.
gpui::actions!(gluxe_focus, [FocusNext, FocusPrev]);

/// Key context for the root view. The `Tab`/`Shift-Tab` bindings are scoped to it
/// (in `lib.rs`) so a deeper context (e.g. `"TextInput"`) can override the key.
pub(crate) const ROOT_KEY_CONTEXT: &str = "Root";

pub(crate) struct RootView {
    /// Holds focus while nothing else is, giving `Tab` a dispatch origin.
    pub(crate) focus_handle: gpui::FocusHandle,
}

impl RootView {
    /// Construct the root view, allocating its focus handle.
    pub(crate) fn new(cx: &mut GpuiContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    /// If focus has fallen to the root fallback / nothing (after blur or a click
    /// elsewhere), restore it to the last focused element so Tab resumes from there
    /// instead of the first stop. The anchor is kept current by `render`. Restoring
    /// then `focus_next` fires no spurious events — gpui coalesces focus per frame.
    fn resume_focus_anchor(&self, window: &mut Window, cx: &mut App) {
        let on_real_element = window.focused(cx).is_some() && !self.focus_handle.is_focused(window);
        if !on_real_element {
            if let Some(prev) = crate::state::focus_anchor() {
                if let Some(handle) = crate::state::get_focus_handle(prev, cx) {
                    window.focus(&handle, cx);
                }
            }
        }
    }

    pub(crate) fn apply_outcome(
        &mut self,
        outcome: ApplyOutcome,
        cx: &mut GpuiContext<Self>,
    ) -> bool {
        let mut root_dirty = outcome.root_dirty;
        for id in outcome.removed_nodes {
            remove_node_view(id);
            ANCHOR_BOUNDS.with(|m| {
                m.borrow_mut().remove(&id);
            });
            ANCHOR_NAMES.with(|m| m.borrow_mut().retain(|_, &mut v| v != id));
        }
        for id in outcome.dirty_nodes {
            if is_text_input_node(id) {
                notify_text_input_entity(id, cx);
            }
            if let Some(entity) = get_node_view_entity(id) {
                let _ = entity.update(cx, |_, cx| cx.notify());
            } else {
                root_dirty = true;
            }
        }
        root_dirty
    }
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut GpuiContext<Self>) -> impl IntoElement {
        // Record the focused element as the Tab resume anchor (see resume_focus_anchor).
        if let Some(id) = crate::state::focused_element_id(window) {
            crate::state::set_focus_anchor(Some(id));
        }
        // Snapshot the active element (any kind, incl. TextInput) for the
        // synchronous JS `getActiveElement()`. Recorded BEFORE the root-focus
        // fallback below so it reflects the real focused element, not the root.
        crate::state::set_active_element(crate::state::active_element_id(window, cx));
        // Keep the root focused while nothing else is, so `Tab` has a dispatch
        // origin. Self-correcting: once something is focused this is skipped.
        if window.focused(cx).is_none() {
            window.focus(&self.focus_handle, cx);
        }
        let root_ids = with_tree(|tree| tree.root_children.clone());
        let root_children: Vec<AnyElement> = root_ids
            .iter()
            .filter_map(|&id| render_child(id, cx))
            .collect();
        div()
            .track_focus(&self.focus_handle)
            .key_context(ROOT_KEY_CONTEXT)
            .on_action(cx.listener(|this, _: &FocusNext, window, cx| {
                this.resume_focus_anchor(window, cx);
                window.focus_next(cx);
            }))
            .on_action(cx.listener(|this, _: &FocusPrev, window, cx| {
                this.resume_focus_anchor(window, cx);
                window.focus_prev(cx);
            }))
            .size_full()
            .flex()
            .items_start()
            .justify_start()
            .children(root_children)
    }
}

// ---------------------------------------------------------------------------
// Local filesystem path resolution
// ---------------------------------------------------------------------------

/// Resolve a `file://` URL or bare path into a `PathBuf` for `gpui::img`.
///
/// `file://` is parsed via the [`url`] crate to handle Windows paths
/// (`file:///C:/…`), percent-encoding, and the host component in `file://host/path`.
/// Falls back to a raw strip of the prefix if parsing fails; bare paths pass through.
fn local_image_path(src: &str) -> PathBuf {
    if src.starts_with("file://") {
        if let Ok(url) = Url::parse(src) {
            if let Ok(path) = url.to_file_path() {
                return path;
            }
        }
        // Malformed `file://` that `url` refused to parse: strip and treat as path.
        return PathBuf::from(src.trim_start_matches("file://"));
    }
    PathBuf::from(src)
}
