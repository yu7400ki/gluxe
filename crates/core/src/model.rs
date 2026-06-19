use gpui::{DefiniteLength, GridPlacement, Rgba, WindowControlArea};
use rustc_hash::{FxHashMap, FxHashSet};

// ---------------------------------------------------------------------------
// Core data model
// ---------------------------------------------------------------------------

pub(crate) type ElementId = u64;

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
    /// CSS `font-family`: a font family name string.
    pub(crate) font_family: Option<String>,
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

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Props {
    pub(crate) style: StyleFields,
    /// Parsed `style.transition` declarations. Empty = no transitions; changes
    /// to animatable style fields then apply instantly.
    pub(crate) transitions: Vec<crate::anim::TransitionSpec>,
    /// Override styles applied while the pointer hovers over the element (`:hover`).
    pub(crate) hover: Option<Box<StyleFields>>,
    /// Override styles while pressed (`:active`). Requires a stable element id.
    pub(crate) active: Option<Box<StyleFields>>,
    /// Override styles while the element holds keyboard focus (`:focus`). Requires `track_focus`.
    pub(crate) focus_style: Option<Box<StyleFields>>,
    /// Override styles while focused *via the keyboard* (`:focus-visible`). Requires `track_focus`.
    pub(crate) focus_visible_style: Option<Box<StyleFields>>,
    /// Tab order index (HTML `tabIndex`). `Some(n)` makes the element focusable;
    /// `n >= 0` joins the Tab order, `n < 0` is programmatically focusable only.
    pub(crate) tab_index: Option<i32>,
    /// Explicit `tabStop` override. `None` → derived from `tab_index` (n>=0 = stop).
    pub(crate) tab_stop: Option<bool>,
    pub(crate) src: Option<String>,         // for Image elements
    pub(crate) value: Option<String>,       // for TextInput elements (controlled value)
    pub(crate) placeholder: Option<String>, // for TextInput elements
    /// Element receives keyboard focus on first render. Implies `track_focus` even
    /// when no `onKeyDown` handler is registered.
    pub(crate) autofocus: bool,
    /// Raw JS props (event handlers stripped) as JSON, for `Native` component render fns.
    pub(crate) raw: Option<serde_json::Value>,
    /// Which GPUI event listeners to attach (populated from JS props like `onClick`).
    pub(crate) events: Events,
    /// Custom-titlebar hit-test region. `None` = ordinary content. On Windows the
    /// OS handles the NCHITTEST; on macOS/Linux `render.rs` attaches framework handlers.
    pub(crate) window_control_area: Option<WindowControlArea>,
    /// Marks this element as a named anchor that floating elements can bind to.
    pub(crate) anchor_name: Option<String>,
    /// Positions this element relative to a named anchor (floating overlay).
    pub(crate) floating: Option<FloatingSpec>,
    /// Explicit override for mouse occlusion (see [`Props::should_occlude`]).
    /// `None` → derived from [`Props::is_overlay`]; `Some(b)` forces it on/off
    /// regardless of position, decoupling occlusion from `position: absolute`.
    pub(crate) occlude: Option<bool>,
}

impl Props {
    /// Whether this element needs a `FocusHandle` / `track_focus` — true for any
    /// focus-related prop (`onKeyDown`/`onFocus`/`onBlur`/`autoFocus`/`tabIndex`/
    /// `_focus`/`_focusVisible`). Shared by `focusable_ids` tracking and `render.rs`.
    pub(crate) fn is_focusable(&self) -> bool {
        self.events.keydown
            || self.events.focus
            || self.events.blur
            || self.autofocus
            || self.tab_index.is_some()
            || self.focus_style.is_some()
            || self.focus_visible_style.is_some()
    }

    /// Whether this element is a keyboard Tab stop: explicit `tabStop` wins;
    /// otherwise a TextInput is a stop unless `tabIndex < 0`, while everything else
    /// is a stop only with `tabIndex >= 0`. The SINGLE source for this rule — used
    /// by the gpui handle config (render.rs `attach_focus!`, text_input.rs) and by
    /// `Tree::focusable_descendants`, which must agree to keep Tab order consistent.
    pub(crate) fn resolve_tab_stop(&self, is_text_input: bool) -> bool {
        self.tab_stop.unwrap_or_else(|| {
            if is_text_input {
                self.tab_index.map_or(true, |i| i >= 0)
            } else {
                self.tab_index.is_some_and(|i| i >= 0)
            }
        })
    }

    /// Whether this element, *by default*, is an overlay that paints on top of
    /// other content and should block the mouse from reaching elements painted
    /// behind it (GPUI's `.occlude()`). True for floating elements and out-of-flow
    /// `position:absolute` boxes — an out-of-flow positioned box captures pointer
    /// events over its area.
    ///
    /// This is only the *default* heuristic; the actual decision is
    /// [`Props::should_occlude`], which an explicit `occlude` prop can override.
    pub(crate) fn is_overlay(&self) -> bool {
        self.floating.is_some() || self.style.position.as_deref() == Some("absolute")
    }

    /// Whether to call GPUI's `.occlude()` on this element, blocking the mouse
    /// from reaching elements painted behind it.
    ///
    /// Without occlusion, GPUI dispatches a click/hover to *every* hitbox under
    /// the cursor, so overlays (floating dropdowns, full-window dismiss backdrops)
    /// let events fall through to whatever sits behind them. The explicit
    /// `occlude` prop decouples this from layout: an in-flow element can block
    /// events (e.g. a centred dialog panel inside a flex positioner, so clicks on
    /// it don't reach the dismiss layer), and an out-of-flow `position:absolute`
    /// element can opt *out* (e.g. a decorative scrim that lets clicks through).
    ///
    /// Occlude sparingly: occluding ordinary nodes suppresses an ancestor's
    /// `_hover` / `_active` wherever the occluding child sits on top of it.
    pub(crate) fn should_occlude(&self) -> bool {
        self.occlude.unwrap_or_else(|| self.is_overlay())
    }
}

/// Which JS event handlers an element has registered.
///
/// Only presence flags are stored in Rust; the actual JS functions live in the
/// `handlers` Map in host-config.ts (avoids GC-tracing issues with Boa JsObjects).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Events {
    pub(crate) click: bool,
    pub(crate) mousedown: bool,
    pub(crate) mouseup: bool,
    pub(crate) mousemove: bool,
    pub(crate) mouseenter: bool,
    pub(crate) mouseleave: bool,
    pub(crate) keydown: bool,
    /// `onFocus` — element gained keyboard focus (View/Image; TextInput uses its own path).
    pub(crate) focus: bool,
    /// `onBlur` — element lost keyboard focus.
    pub(crate) blur: bool,
}

impl Events {
    pub(crate) fn any(self) -> bool {
        self.click
            || self.mousedown
            || self.mouseup
            || self.mousemove
            || self.mouseenter
            || self.mouseleave
            || self.keydown
            || self.focus
            || self.blur
    }

    pub(crate) fn from_types(types: &[String]) -> Self {
        let mut events = Events::default();
        for event_type in types {
            match event_type.as_str() {
                "click" => events.click = true,
                "mousedown" => events.mousedown = true,
                "mouseup" => events.mouseup = true,
                "mousemove" => events.mousemove = true,
                "mouseenter" => events.mouseenter = true,
                "mouseleave" => events.mouseleave = true,
                "keydown" => events.keydown = true,
                "focus" => events.focus = true,
                "blur" => events.blur = true,
                _ => {}
            }
        }
        events
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ElementKind {
    View,
    Text,
    Image,
    RawText,
    TextInput,
    /// A host-registered native GPUI component, addressed by its element-type
    /// name (see `component.rs`). Not `Copy` because it carries the name string.
    Native(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Element {
    pub(crate) kind: ElementKind,
    pub(crate) props: Props,
    pub(crate) text: Option<String>,
    pub(crate) children: Vec<ElementId>,
}

#[derive(Debug, Default)]
pub(crate) struct Tree {
    pub(crate) nodes: FxHashMap<ElementId, Element>,
    pub(crate) root_children: Vec<ElementId>,
    pub(crate) parents: FxHashMap<ElementId, ElementId>,
    /// Incrementally maintained index of `TextInput` node ids for the render pre-pass.
    /// A node's kind never changes, so this set only grows on create and shrinks on detach.
    pub(crate) text_input_ids: FxHashSet<ElementId>,
    /// Incrementally maintained index of keyboard-focusable node ids → `autofocus` flag.
    /// A node is focusable per [`Props::is_focusable`]; toggled by `UpdateProps`.
    pub(crate) focusable_ids: FxHashMap<ElementId, bool>,
}

impl Tree {
    /// Tab-stop focusables in `root`'s subtree (root included when it qualifies),
    /// in GPUI `focus_next` order: ascending `tabIndex` (default `0`), ties by
    /// preorder. Backs `getFocusableElements`.
    pub(crate) fn focusable_descendants(&self, root: ElementId) -> Vec<ElementId> {
        // (tab_index, preorder index, id) — sorted to match GPUI's TabStopMap.
        let mut found: Vec<(i32, usize, ElementId)> = Vec::new();
        let mut order: usize = 0;
        let mut stack: Vec<ElementId> = vec![root];
        // Preorder DFS; push children reversed so they pop in document order.
        while let Some(id) = stack.pop() {
            let Some(node) = self.nodes.get(&id) else {
                continue;
            };
            let props = &node.props;
            // TextInput is intrinsically focusable; View/Image only with a focus
            // prop. The tab_stop default is single-sourced in `resolve_tab_stop`.
            let is_text_input = matches!(node.kind, ElementKind::TextInput);
            let is_tab_stop =
                (is_text_input || props.is_focusable()) && props.resolve_tab_stop(is_text_input);
            if is_tab_stop {
                found.push((props.tab_index.unwrap_or(0), order, id));
            }
            order += 1;
            for &child in node.children.iter().rev() {
                stack.push(child);
            }
        }
        found.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        found.into_iter().map(|(_, _, id)| id).collect()
    }
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct ApplyOutcome {
    pub(crate) dirty_nodes: FxHashSet<ElementId>,
    pub(crate) root_dirty: bool,
    pub(crate) removed_nodes: Vec<ElementId>,
}

impl ApplyOutcome {
    fn dirty_node(&mut self, id: ElementId) {
        self.dirty_nodes.insert(id);
    }

    fn dirty_root(&mut self) {
        self.root_dirty = true;
    }

    fn dirty_parent_or_root(&mut self, tree: &Tree, id: ElementId) {
        if let Some(parent) = tree.parents.get(&id).copied() {
            self.dirty_node(parent);
        } else {
            self.dirty_root();
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.dirty_nodes.is_empty() && !self.root_dirty && self.removed_nodes.is_empty()
    }

    pub(crate) fn merge(&mut self, other: ApplyOutcome) {
        self.dirty_nodes.extend(other.dirty_nodes);
        self.root_dirty |= other.root_dirty;
        self.removed_nodes.extend(other.removed_nodes);
    }
}

#[derive(Debug)]
pub(crate) enum UICommand {
    CreateInstance {
        id: ElementId,
        kind: ElementKind,
        props: Props,
    },
    CreateText {
        id: ElementId,
        text: String,
    },
    AppendChild {
        parent: ElementId,
        child: ElementId,
    },
    AppendToContainer {
        child: ElementId,
    },
    InsertBefore {
        parent: ElementId,
        child: ElementId,
        before: ElementId,
    },
    InsertInContainer {
        child: ElementId,
        before: ElementId,
    },
    RemoveChild {
        parent: ElementId,
        child: ElementId,
    },
    RemoveFromContainer {
        child: ElementId,
    },
    UpdateProps {
        id: ElementId,
        props: Props,
    },
    UpdateText {
        id: ElementId,
        text: String,
    },
    ClearContainer,
    DetachDeleted {
        id: ElementId,
    },
}

pub(crate) fn apply_command(tree: &mut Tree, cmd: UICommand) -> ApplyOutcome {
    let mut outcome = ApplyOutcome::default();
    match cmd {
        UICommand::CreateInstance { id, kind, props } => {
            if matches!(kind, ElementKind::TextInput) {
                tree.text_input_ids.insert(id);
            }
            if props.is_focusable() {
                tree.focusable_ids.insert(id, props.autofocus);
            }
            tree.nodes.insert(
                id,
                Element {
                    kind,
                    props,
                    text: None,
                    children: Vec::new(),
                },
            );
        }
        UICommand::CreateText { id, text } => {
            tree.nodes.insert(
                id,
                Element {
                    kind: ElementKind::RawText,
                    props: Props::default(),
                    text: Some(text),
                    children: Vec::new(),
                },
            );
        }
        UICommand::AppendChild { parent, child } => {
            // React reorders children by re-issuing AppendChild on an already-mounted
            // child without a preceding RemoveChild (DOM appendChild semantics). Remove
            // any existing occurrence first to avoid duplicates. Cross-parent moves rely
            // on React's RemoveChild(old_parent, child) to clear the old entry.
            if let Some(parent_node) = tree.nodes.get_mut(&parent) {
                parent_node.children.retain(|&id| id != child);
                parent_node.children.push(child);
                tree.parents.insert(child, parent);
                outcome.dirty_node(parent);
            }
        }
        UICommand::AppendToContainer { child } => {
            tree.root_children.retain(|&id| id != child);
            tree.root_children.push(child);
            tree.parents.remove(&child);
            outcome.dirty_root();
        }
        UICommand::InsertBefore {
            parent,
            child,
            before,
        } => {
            // Same dedup as AppendChild: remove before inserting so a same-parent
            // reorder positions correctly. `before` is found after removal.
            if let Some(parent_node) = tree.nodes.get_mut(&parent) {
                parent_node.children.retain(|&id| id != child);
                if let Some(pos) = parent_node.children.iter().position(|&id| id == before) {
                    parent_node.children.insert(pos, child);
                } else {
                    parent_node.children.push(child);
                }
                tree.parents.insert(child, parent);
                outcome.dirty_node(parent);
            }
        }
        UICommand::InsertInContainer { child, before } => {
            tree.root_children.retain(|&id| id != child);
            if let Some(pos) = tree.root_children.iter().position(|&id| id == before) {
                tree.root_children.insert(pos, child);
            } else {
                tree.root_children.push(child);
            }
            tree.parents.remove(&child);
            outcome.dirty_root();
        }
        UICommand::RemoveChild { parent, child } => {
            if let Some(parent_node) = tree.nodes.get_mut(&parent) {
                let before_len = parent_node.children.len();
                parent_node.children.retain(|&id| id != child);
                if parent_node.children.len() != before_len {
                    if tree.parents.get(&child).copied() == Some(parent) {
                        tree.parents.remove(&child);
                    }
                    outcome.dirty_node(parent);
                }
            }
        }
        UICommand::RemoveFromContainer { child } => {
            let before_len = tree.root_children.len();
            tree.root_children.retain(|&id| id != child);
            if tree.root_children.len() != before_len {
                outcome.dirty_root();
            }
        }
        UICommand::UpdateProps { id, props } => {
            if let Some(element) = tree.nodes.get_mut(&id) {
                if element.props == props {
                    return outcome;
                }
                // Focus-related props can be added or removed, so recompute on every update.
                if props.is_focusable() {
                    tree.focusable_ids.insert(id, props.autofocus);
                } else {
                    tree.focusable_ids.remove(&id);
                }
                element.props = props;
                outcome.dirty_node(id);
            }
        }
        UICommand::UpdateText { id, text } => {
            if let Some(element) = tree.nodes.get_mut(&id) {
                if element.text.as_deref() == Some(text.as_str()) {
                    return outcome;
                }
                element.text = Some(text);
                outcome.dirty_parent_or_root(tree, id);
            }
        }
        UICommand::ClearContainer => {
            if !tree.root_children.is_empty() {
                tree.root_children.clear();
                outcome.dirty_root();
            }
        }
        UICommand::DetachDeleted { id } => {
            // Pure memory cleanup — the preceding Remove*/ClearContainer already
            // dirtied the surviving parent/root, so no dirty is emitted here.
            // Each descendant issues its own DetachDeleted, so no subtree scan needed.
            tree.text_input_ids.remove(&id);
            tree.focusable_ids.remove(&id);
            tree.nodes.remove(&id);
            tree.parents.remove(&id);
            outcome.removed_nodes.push(id);
        }
    }
    outcome
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

    // ---- Events::from_types / any ----

    #[test]
    fn events_from_single_type() {
        let e = Events::from_types(&["click".to_string()]);
        assert!(e.click);
        assert!(!e.mousedown);
        assert!(!e.keydown);
    }

    #[test]
    fn events_from_multiple_types() {
        let e = Events::from_types(&[
            "click".to_string(),
            "mousedown".to_string(),
            "keydown".to_string(),
        ]);
        assert!(e.click);
        assert!(e.mousedown);
        assert!(e.keydown);
        assert!(!e.mouseup);
    }

    #[test]
    fn events_unknown_type_is_ignored() {
        let e = Events::from_types(&["unknown".to_string(), "click".to_string()]);
        assert!(e.click);
        assert!(!e.mouseup);
    }

    #[test]
    fn events_all_types_recognized() {
        let all = vec![
            "click".to_string(),
            "mousedown".to_string(),
            "mouseup".to_string(),
            "mousemove".to_string(),
            "mouseenter".to_string(),
            "mouseleave".to_string(),
            "keydown".to_string(),
        ];
        let e = Events::from_types(&all);
        assert!(e.click && e.mousedown && e.mouseup && e.mousemove);
        assert!(e.mouseenter && e.mouseleave && e.keydown);
    }

    #[test]
    fn events_any_false_when_empty() {
        let e = Events::default();
        assert!(!e.any());
    }

    #[test]
    fn events_any_true_when_one_set() {
        let mut e = Events::default();
        e.click = true;
        assert!(e.any());
    }

    // ---- StyleFields::scrolls ----

    #[test]
    fn scrolls_false_by_default() {
        let s = StyleFields::default();
        assert!(!s.scrolls());
    }

    #[test]
    fn scrolls_true_when_overflow_x_scroll() {
        let mut s = StyleFields::default();
        s.overflow_x = Some(OverflowMode::Scroll);
        assert!(s.scrolls());
    }

    #[test]
    fn scrolls_true_when_overflow_y_scroll() {
        let mut s = StyleFields::default();
        s.overflow_y = Some(OverflowMode::Scroll);
        assert!(s.scrolls());
    }

    #[test]
    fn scrolls_false_when_hidden() {
        let mut s = StyleFields::default();
        s.overflow_x = Some(OverflowMode::Hidden);
        s.overflow_y = Some(OverflowMode::Hidden);
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

    // ---- Props::is_overlay ----

    #[test]
    fn is_overlay_default_false() {
        assert!(!Props::default().is_overlay());
    }

    #[test]
    fn is_overlay_true_for_absolute_position() {
        let mut props = Props::default();
        props.style.position = Some("absolute".to_string());
        assert!(props.is_overlay());
    }

    #[test]
    fn is_overlay_false_for_relative_position() {
        let mut props = Props::default();
        props.style.position = Some("relative".to_string());
        assert!(!props.is_overlay());
    }

    #[test]
    fn is_overlay_true_for_floating() {
        let mut props = Props::default();
        props.floating = Some(FloatingSpec {
            anchor: "a".to_string(),
            side: FloatingSide::Bottom,
            align: FloatingAlign::Start,
            offset: LengthValue::Px(0.0),
            margin: LengthValue::Px(0.0),
            priority: None,
        });
        assert!(props.is_overlay());
    }

    // ---- Props::should_occlude ----

    #[test]
    fn should_occlude_defaults_to_is_overlay() {
        // No explicit prop: an ordinary node does not occlude, an overlay does.
        assert!(!Props::default().should_occlude());
        let mut overlay = Props::default();
        overlay.style.position = Some("absolute".to_string());
        assert!(overlay.should_occlude());
    }

    #[test]
    fn should_occlude_explicit_true_on_in_flow_node() {
        // An in-flow element can be forced to occlude (e.g. a dialog panel).
        let mut props = Props::default();
        props.occlude = Some(true);
        assert!(props.should_occlude());
    }

    #[test]
    fn should_occlude_explicit_false_overrides_overlay() {
        // An absolute element can opt out of occlusion (e.g. a decorative scrim).
        let mut props = Props::default();
        props.style.position = Some("absolute".to_string());
        props.occlude = Some(false);
        assert!(!props.should_occlude());
    }

    // ---- apply_command — helpers (shared with sequence_tests below) ----

    pub(super) fn make_view_instance(id: ElementId) -> UICommand {
        UICommand::CreateInstance {
            id,
            kind: ElementKind::View,
            props: Props::default(),
        }
    }

    pub(super) fn make_text_input_instance(id: ElementId) -> UICommand {
        UICommand::CreateInstance {
            id,
            kind: ElementKind::TextInput,
            props: Props::default(),
        }
    }

    pub(super) fn make_keydown_instance(id: ElementId, autofocus: bool) -> UICommand {
        let mut props = Props::default();
        props.events.keydown = true;
        props.autofocus = autofocus;
        UICommand::CreateInstance {
            id,
            kind: ElementKind::View,
            props,
        }
    }

    fn make_tabbable_instance(id: ElementId, tab_index: Option<i32>) -> UICommand {
        let mut props = Props::default();
        props.tab_index = tab_index;
        UICommand::CreateInstance {
            id,
            kind: ElementKind::View,
            props,
        }
    }

    // ---- resolve_tab_stop ----

    #[test]
    fn resolve_tab_stop_defaults_by_kind() {
        let mut props = Props::default(); // no tabIndex, no tabStop
        // TextInput defaults to a stop; View/Image do not.
        assert!(props.resolve_tab_stop(true));
        assert!(!props.resolve_tab_stop(false));
        // tabIndex >= 0 makes either a stop; < 0 takes either out.
        props.tab_index = Some(0);
        assert!(props.resolve_tab_stop(true));
        assert!(props.resolve_tab_stop(false));
        props.tab_index = Some(-1);
        assert!(!props.resolve_tab_stop(true));
        assert!(!props.resolve_tab_stop(false));
        // Explicit tabStop wins over the kind default and tabIndex.
        props.tab_index = Some(-1);
        props.tab_stop = Some(true);
        assert!(props.resolve_tab_stop(false));
    }

    // ---- focusable_descendants ----

    #[test]
    fn focusable_descendants_orders_by_tab_index_then_tree_order() {
        let mut tree = Tree::default();
        // panel(1) is programmatic-only (-1, skipped); children are tab stops.
        apply_command(&mut tree, make_tabbable_instance(1, Some(-1)));
        apply_command(&mut tree, make_tabbable_instance(2, Some(0))); // tree order 0
        apply_command(&mut tree, make_tabbable_instance(3, Some(0))); // tree order 1, index 0
        apply_command(&mut tree, make_tabbable_instance(4, Some(5))); // higher index → last
        for child in [2, 3, 4] {
            apply_command(&mut tree, UICommand::AppendChild { parent: 1, child });
        }
        // 2 and 3 share tabIndex 0 → tree order; 4 (index 5) sorts after both.
        // 1 (index -1) is not a tab stop, so it is excluded.
        assert_eq!(tree.focusable_descendants(1), vec![2, 3, 4]);
    }

    #[test]
    fn focusable_descendants_includes_plain_text_input() {
        let mut tree = Tree::default();
        // A bare TextInput (no focus props) is intrinsically a tab stop.
        apply_command(&mut tree, make_tabbable_instance(1, Some(-1))); // panel, not a stop
        apply_command(&mut tree, make_text_input_instance(2));
        apply_command(
            &mut tree,
            UICommand::AppendChild {
                parent: 1,
                child: 2,
            },
        );
        assert_eq!(tree.focusable_descendants(1), vec![2]);
    }

    #[test]
    fn focusable_descendants_text_input_respects_tab_stop_overrides() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_tabbable_instance(1, Some(-1)));
        // tabIndex < 0 takes the TextInput out of the Tab order.
        let mut hidden = Props::default();
        hidden.tab_index = Some(-1);
        apply_command(
            &mut tree,
            UICommand::CreateInstance {
                id: 2,
                kind: ElementKind::TextInput,
                props: hidden,
            },
        );
        apply_command(
            &mut tree,
            UICommand::AppendChild {
                parent: 1,
                child: 2,
            },
        );
        assert!(tree.focusable_descendants(1).is_empty());
    }

    #[test]
    fn focusable_descendants_excludes_non_tab_stops_and_unknown_roots() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_view_instance(1)); // not focusable
        apply_command(&mut tree, make_tabbable_instance(2, Some(-1))); // focusable, not a stop
        apply_command(
            &mut tree,
            UICommand::AppendChild {
                parent: 1,
                child: 2,
            },
        );
        assert!(tree.focusable_descendants(1).is_empty());
        assert!(tree.focusable_descendants(999).is_empty());
    }

    // ---- apply_command — CreateInstance ----

    #[test]
    fn create_view_instance_adds_node() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_view_instance(1));
        assert!(tree.nodes.contains_key(&1));
        assert!(!tree.text_input_ids.contains(&1));
        assert!(!tree.focusable_ids.contains_key(&1));
    }

    #[test]
    fn create_text_input_registers_in_text_input_ids() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_text_input_instance(2));
        assert!(tree.text_input_ids.contains(&2));
    }

    #[test]
    fn create_keydown_node_registers_in_focusable_ids() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_keydown_instance(3, false));
        assert!(tree.focusable_ids.contains_key(&3));
        assert_eq!(tree.focusable_ids[&3], false);
    }

    #[test]
    fn create_autofocus_node_registers_with_true_flag() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_keydown_instance(4, true));
        assert_eq!(tree.focusable_ids[&4], true);
    }

    // ---- apply_command — CreateText / UpdateText ----

    #[test]
    fn create_text_node() {
        let mut tree = Tree::default();
        apply_command(
            &mut tree,
            UICommand::CreateText {
                id: 5,
                text: "hello".to_string(),
            },
        );
        let node = tree.nodes.get(&5).unwrap();
        assert_eq!(node.text.as_deref(), Some("hello"));
    }

    #[test]
    fn update_text_changes_text_content() {
        let mut tree = Tree::default();
        apply_command(
            &mut tree,
            UICommand::CreateText {
                id: 6,
                text: "old".to_string(),
            },
        );
        let outcome = apply_command(
            &mut tree,
            UICommand::UpdateText {
                id: 6,
                text: "new".to_string(),
            },
        );
        assert_eq!(tree.nodes[&6].text.as_deref(), Some("new"));
        assert!(outcome.root_dirty);
    }

    // ---- apply_command — AppendChild / AppendToContainer ----

    #[test]
    fn append_child_adds_to_parent_children() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_view_instance(10));
        apply_command(&mut tree, make_view_instance(11));
        let outcome = apply_command(
            &mut tree,
            UICommand::AppendChild {
                parent: 10,
                child: 11,
            },
        );
        assert_eq!(tree.nodes[&10].children, vec![11]);
        assert_eq!(tree.parents.get(&11), Some(&10));
        assert!(outcome.dirty_nodes.contains(&10));
        assert!(!outcome.root_dirty);
    }

    #[test]
    fn append_child_moves_existing_to_end_without_duplicating() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_view_instance(12));
        apply_command(&mut tree, make_view_instance(13));
        apply_command(&mut tree, make_view_instance(14));
        for child in [13, 14] {
            apply_command(&mut tree, UICommand::AppendChild { parent: 12, child });
        }
        // Move 13 to the end.
        apply_command(
            &mut tree,
            UICommand::AppendChild {
                parent: 12,
                child: 13,
            },
        );
        assert_eq!(tree.nodes[&12].children, vec![14, 13]);
    }

    #[test]
    fn append_to_container_moves_existing_to_end_without_duplicating() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_view_instance(15));
        apply_command(&mut tree, make_view_instance(16));
        apply_command(&mut tree, UICommand::AppendToContainer { child: 15 });
        apply_command(&mut tree, UICommand::AppendToContainer { child: 16 });
        let outcome = apply_command(&mut tree, UICommand::AppendToContainer { child: 15 });
        assert_eq!(tree.root_children, vec![16, 15]);
        assert!(outcome.root_dirty);
    }

    #[test]
    fn append_to_container_adds_to_root() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_view_instance(20));
        let outcome = apply_command(&mut tree, UICommand::AppendToContainer { child: 20 });
        assert!(tree.root_children.contains(&20));
        assert!(outcome.root_dirty);
    }

    // ---- apply_command — InsertBefore ----

    #[test]
    fn insert_before_existing_anchor() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_view_instance(30));
        apply_command(&mut tree, make_view_instance(31));
        apply_command(&mut tree, make_view_instance(32));
        apply_command(
            &mut tree,
            UICommand::AppendChild {
                parent: 30,
                child: 31,
            },
        );
        // insert 32 before 31
        let outcome = apply_command(
            &mut tree,
            UICommand::InsertBefore {
                parent: 30,
                child: 32,
                before: 31,
            },
        );
        assert_eq!(tree.nodes[&30].children, vec![32, 31]);
        assert_eq!(tree.parents.get(&32), Some(&30));
        assert!(outcome.dirty_nodes.contains(&30));
    }

    #[test]
    fn insert_before_missing_anchor_falls_back_to_push() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_view_instance(40));
        apply_command(&mut tree, make_view_instance(41));
        // anchor 99 does not exist → push to end
        apply_command(
            &mut tree,
            UICommand::InsertBefore {
                parent: 40,
                child: 41,
                before: 99,
            },
        );
        assert_eq!(tree.nodes[&40].children, vec![41]);
    }

    #[test]
    fn insert_before_reorders_without_duplicating() {
        // Same-parent reorder: re-issue InsertBefore on an already-mounted child.
        let mut tree = Tree::default();
        apply_command(&mut tree, make_view_instance(42));
        apply_command(&mut tree, make_view_instance(43));
        apply_command(&mut tree, make_view_instance(44));
        for child in [43, 44] {
            apply_command(&mut tree, UICommand::AppendChild { parent: 42, child });
        }
        // Move 44 before 43.
        apply_command(
            &mut tree,
            UICommand::InsertBefore {
                parent: 42,
                child: 44,
                before: 43,
            },
        );
        assert_eq!(tree.nodes[&42].children, vec![44, 43]);
    }

    // ---- apply_command — InsertInContainer ----

    #[test]
    fn insert_in_container_before_existing_anchor() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_view_instance(50));
        apply_command(&mut tree, make_view_instance(51));
        apply_command(&mut tree, UICommand::AppendToContainer { child: 50 });
        let outcome = apply_command(
            &mut tree,
            UICommand::InsertInContainer {
                child: 51,
                before: 50,
            },
        );
        assert_eq!(tree.root_children, vec![51, 50]);
        assert!(!tree.parents.contains_key(&51));
        assert!(outcome.root_dirty);
    }

    #[test]
    fn insert_in_container_missing_anchor_falls_back_to_push() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_view_instance(60));
        apply_command(
            &mut tree,
            UICommand::InsertInContainer {
                child: 60,
                before: 99,
            },
        );
        assert!(tree.root_children.contains(&60));
    }

    #[test]
    fn insert_in_container_reorders_without_duplicating() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_view_instance(61));
        apply_command(&mut tree, make_view_instance(62));
        apply_command(&mut tree, UICommand::AppendToContainer { child: 61 });
        apply_command(&mut tree, UICommand::AppendToContainer { child: 62 });
        // Move 62 before 61.
        apply_command(
            &mut tree,
            UICommand::InsertInContainer {
                child: 62,
                before: 61,
            },
        );
        assert_eq!(tree.root_children, vec![62, 61]);
    }

    // ---- apply_command — RemoveChild / RemoveFromContainer / ClearContainer ----

    #[test]
    fn remove_child_removes_from_parent() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_view_instance(70));
        apply_command(&mut tree, make_view_instance(71));
        apply_command(
            &mut tree,
            UICommand::AppendChild {
                parent: 70,
                child: 71,
            },
        );
        let outcome = apply_command(
            &mut tree,
            UICommand::RemoveChild {
                parent: 70,
                child: 71,
            },
        );
        assert!(tree.nodes[&70].children.is_empty());
        assert!(!tree.parents.contains_key(&71));
        assert!(outcome.dirty_nodes.contains(&70));
    }

    #[test]
    fn remove_from_container() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_view_instance(80));
        apply_command(&mut tree, UICommand::AppendToContainer { child: 80 });
        let outcome = apply_command(&mut tree, UICommand::RemoveFromContainer { child: 80 });
        assert!(!tree.root_children.contains(&80));
        assert!(outcome.root_dirty);
    }

    #[test]
    fn clear_container_empties_root_children() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_view_instance(90));
        apply_command(&mut tree, make_view_instance(91));
        apply_command(&mut tree, UICommand::AppendToContainer { child: 90 });
        apply_command(&mut tree, UICommand::AppendToContainer { child: 91 });
        let outcome = apply_command(&mut tree, UICommand::ClearContainer);
        assert!(tree.root_children.is_empty());
        assert!(outcome.root_dirty);
    }

    // ---- apply_command — UpdateProps ----

    #[test]
    fn update_props_adds_to_focusable_ids_when_keydown_added() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_view_instance(100));
        assert!(!tree.focusable_ids.contains_key(&100));
        let mut props = Props::default();
        props.events.keydown = true;
        let outcome = apply_command(&mut tree, UICommand::UpdateProps { id: 100, props });
        assert!(tree.focusable_ids.contains_key(&100));
        assert!(outcome.dirty_nodes.contains(&100));
    }

    #[test]
    fn update_props_removes_from_focusable_ids_when_keydown_removed() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_keydown_instance(101, false));
        assert!(tree.focusable_ids.contains_key(&101));
        apply_command(
            &mut tree,
            UICommand::UpdateProps {
                id: 101,
                props: Props::default(), // no keydown, no autofocus
            },
        );
        assert!(!tree.focusable_ids.contains_key(&101));
    }

    #[test]
    fn update_props_noop_has_empty_outcome() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_view_instance(102));
        let outcome = apply_command(
            &mut tree,
            UICommand::UpdateProps {
                id: 102,
                props: Props::default(),
            },
        );
        assert!(outcome.is_empty());
    }

    #[test]
    fn update_text_marks_parent_dirty() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_view_instance(120));
        apply_command(
            &mut tree,
            UICommand::CreateText {
                id: 121,
                text: "old".to_string(),
            },
        );
        apply_command(
            &mut tree,
            UICommand::AppendChild {
                parent: 120,
                child: 121,
            },
        );
        let outcome = apply_command(
            &mut tree,
            UICommand::UpdateText {
                id: 121,
                text: "new".to_string(),
            },
        );
        assert!(outcome.dirty_nodes.contains(&120));
        assert!(!outcome.root_dirty);
    }

    #[test]
    fn update_text_noop_has_empty_outcome() {
        let mut tree = Tree::default();
        apply_command(
            &mut tree,
            UICommand::CreateText {
                id: 122,
                text: "same".to_string(),
            },
        );
        let outcome = apply_command(
            &mut tree,
            UICommand::UpdateText {
                id: 122,
                text: "same".to_string(),
            },
        );
        assert!(outcome.is_empty());
    }

    // ---- apply_command — DetachDeleted ----

    #[test]
    fn detach_deleted_removes_from_all_indices() {
        let mut tree = Tree::default();
        apply_command(&mut tree, make_text_input_instance(110));
        apply_command(&mut tree, make_keydown_instance(110, false));
        // second CreateInstance overwrites the first node; both indices are populated
        let outcome = apply_command(&mut tree, UICommand::DetachDeleted { id: 110 });
        assert!(!tree.nodes.contains_key(&110));
        assert!(!tree.text_input_ids.contains(&110));
        assert!(!tree.focusable_ids.contains_key(&110));
        assert_eq!(outcome.removed_nodes, vec![110]);
    }

    #[test]
    fn detach_deleted_emits_no_dirty() {
        // RemoveChild already dirtied the parent; DetachDeleted must not re-dirty
        // or every subtree deletion would force a full RootView re-render.
        let mut tree = Tree::default();
        apply_command(&mut tree, make_view_instance(130));
        apply_command(&mut tree, make_view_instance(131));
        apply_command(
            &mut tree,
            UICommand::AppendChild {
                parent: 130,
                child: 131,
            },
        );
        apply_command(
            &mut tree,
            UICommand::RemoveChild {
                parent: 130,
                child: 131,
            },
        );
        let outcome = apply_command(&mut tree, UICommand::DetachDeleted { id: 131 });
        assert!(outcome.dirty_nodes.is_empty());
        assert!(!outcome.root_dirty);
        assert_eq!(outcome.removed_nodes, vec![131]);
    }
}

#[cfg(test)]
mod sequence_tests {
    use super::tests::{
        make_keydown_instance as keydown_view, make_text_input_instance as text_input,
        make_view_instance as view,
    };
    use super::*;

    fn text(id: ElementId, s: &str) -> UICommand {
        UICommand::CreateText {
            id,
            text: s.to_string(),
        }
    }

    /// Applies every command in order, merging all per-command outcomes into one.
    fn apply_all(tree: &mut Tree, cmds: impl IntoIterator<Item = UICommand>) -> ApplyOutcome {
        let mut merged = ApplyOutcome::default();
        for cmd in cmds {
            merged.merge(apply_command(tree, cmd));
        }
        merged
    }

    /// Asserts the structural invariants that `apply_command` must preserve.
    fn assert_consistent(tree: &Tree) {
        // child→parent: parent exists and lists the child exactly once.
        for (&child, &parent) in &tree.parents {
            let parent_node = tree
                .nodes
                .get(&parent)
                .unwrap_or_else(|| panic!("parent {parent} of {child} missing from nodes"));
            let count = parent_node.children.iter().filter(|&&c| c == child).count();
            assert_eq!(
                count, 1,
                "child {child} should appear exactly once in parent {parent}'s children, found {count}"
            );
        }

        // each node's children exist and back-reference this node as parent.
        for (&id, node) in &tree.nodes {
            let mut seen = FxHashSet::default();
            for &c in &node.children {
                assert!(
                    seen.insert(c),
                    "duplicate child {c} in node {id}'s children"
                );
                assert!(
                    tree.nodes.contains_key(&c),
                    "child {c} of {id} missing from nodes"
                );
                assert_eq!(
                    tree.parents.get(&c).copied(),
                    Some(id),
                    "parents[{c}] should be {id}"
                );
            }
        }

        // root children exist, are unique, and have no parents entry.
        let mut seen_root = FxHashSet::default();
        for &id in &tree.root_children {
            assert!(seen_root.insert(id), "duplicate root child {id}");
            assert!(
                tree.nodes.contains_key(&id),
                "root child {id} missing from nodes"
            );
            assert!(
                !tree.parents.contains_key(&id),
                "root child {id} must not have a parents entry"
            );
        }

        // derived indices are subsets of nodes (with the expected kinds).
        for &id in &tree.text_input_ids {
            let node = tree
                .nodes
                .get(&id)
                .unwrap_or_else(|| panic!("text_input id {id} missing from nodes"));
            assert!(
                matches!(node.kind, ElementKind::TextInput),
                "text_input id {id} is not a TextInput"
            );
        }
        for &id in tree.focusable_ids.keys() {
            assert!(
                tree.nodes.contains_key(&id),
                "focusable id {id} missing from nodes"
            );
        }
    }

    // ---- 1. initial mount ----

    #[test]
    fn initial_mount_sequence() {
        // Tree: root <- 1 <- 2 <- 3, with 2 also holding raw text node 4.
        let mut tree = Tree::default();
        let outcome = apply_all(
            &mut tree,
            [
                view(1),
                view(2),
                view(3),
                text(4, "leaf"),
                UICommand::AppendChild {
                    parent: 3,
                    child: 4,
                },
                UICommand::AppendChild {
                    parent: 2,
                    child: 3,
                },
                UICommand::AppendChild {
                    parent: 1,
                    child: 2,
                },
                UICommand::AppendToContainer { child: 1 },
            ],
        );

        assert_eq!(tree.root_children, vec![1]);
        assert_eq!(tree.nodes[&1].children, vec![2]);
        assert_eq!(tree.nodes[&2].children, vec![3]);
        assert_eq!(tree.nodes[&3].children, vec![4]);
        assert_eq!(tree.parents.get(&2), Some(&1));
        assert_eq!(tree.parents.get(&4), Some(&3));
        assert!(!tree.parents.contains_key(&1));
        assert!(outcome.root_dirty);
        assert_consistent(&tree);
    }

    // ---- 2. update + reorder via InsertBefore (regression: dedup at l.455-493) ----

    #[test]
    fn update_then_reorder_via_insert_before() {
        // Mount parent 1 with children [10, 11, 12] (A, B, C).
        let mut tree = Tree::default();
        apply_all(
            &mut tree,
            [
                view(1),
                view(10),
                view(11),
                view(12),
                UICommand::AppendChild {
                    parent: 1,
                    child: 10,
                },
                UICommand::AppendChild {
                    parent: 1,
                    child: 11,
                },
                UICommand::AppendChild {
                    parent: 1,
                    child: 12,
                },
                UICommand::AppendToContainer { child: 1 },
            ],
        );
        assert_eq!(tree.nodes[&1].children, vec![10, 11, 12]);

        // Reorder [A,B,C] -> [C,A,B]: React re-issues InsertBefore on an already-mounted
        // child (often alongside a no-op UpdateProps). The dedup must reposition, not duplicate.
        let outcome = apply_all(
            &mut tree,
            [
                UICommand::UpdateProps {
                    id: 12,
                    props: Props::default(),
                },
                UICommand::InsertBefore {
                    parent: 1,
                    child: 12,
                    before: 10,
                },
            ],
        );

        assert_eq!(tree.nodes[&1].children, vec![12, 10, 11]);
        assert_eq!(tree.nodes[&1].children.len(), 3);
        assert!(outcome.dirty_nodes.contains(&1));
        assert_consistent(&tree);
    }

    // ---- 3. remove subtree then detach ----

    #[test]
    fn remove_subtree_then_detach_frees_everything() {
        // root <- 1 <- { 2(keydown), 3(text input) }, 2 <- 4(text).
        let mut tree = Tree::default();
        apply_all(
            &mut tree,
            [
                view(1),
                keydown_view(2, false),
                text_input(3),
                text(4, "x"),
                UICommand::AppendChild {
                    parent: 2,
                    child: 4,
                },
                UICommand::AppendChild {
                    parent: 1,
                    child: 2,
                },
                UICommand::AppendChild {
                    parent: 1,
                    child: 3,
                },
                UICommand::AppendToContainer { child: 1 },
            ],
        );
        assert!(tree.focusable_ids.contains_key(&2));
        assert!(tree.text_input_ids.contains(&3));
        assert_consistent(&tree);

        let outcome = apply_all(
            &mut tree,
            [
                UICommand::RemoveFromContainer { child: 1 },
                UICommand::DetachDeleted { id: 1 },
                UICommand::DetachDeleted { id: 2 },
                UICommand::DetachDeleted { id: 3 },
                UICommand::DetachDeleted { id: 4 },
            ],
        );

        for id in [1, 2, 3, 4] {
            assert!(!tree.nodes.contains_key(&id), "node {id} should be freed");
            assert!(
                !tree.parents.contains_key(&id),
                "parents[{id}] should be freed"
            );
            assert!(!tree.text_input_ids.contains(&id));
            assert!(!tree.focusable_ids.contains_key(&id));
        }
        assert!(tree.root_children.is_empty());
        assert!(outcome.root_dirty);
        assert_consistent(&tree);
    }

    // ---- 4. clear container then remount ----

    #[test]
    fn clear_container_then_remount() {
        // Production path. Dev hot-reload replaces the Tree wholesale via
        // state::reset_for_reload(), with no per-node DetachDeleted.
        let mut tree = Tree::default();
        apply_all(
            &mut tree,
            [
                view(1),
                view(2),
                UICommand::AppendChild {
                    parent: 1,
                    child: 2,
                },
                UICommand::AppendToContainer { child: 1 },
            ],
        );
        assert_eq!(tree.root_children, vec![1]);

        let cleared = apply_command(&mut tree, UICommand::ClearContainer);
        assert!(tree.root_children.is_empty());
        assert!(
            tree.nodes.contains_key(&1),
            "ClearContainer must not free nodes"
        );
        assert!(tree.nodes.contains_key(&2));
        assert!(cleared.root_dirty);
        assert_consistent(&tree);

        // Remount with fresh ids (monotonic, never reused).
        apply_all(
            &mut tree,
            [
                view(3),
                view(4),
                UICommand::AppendChild {
                    parent: 3,
                    child: 4,
                },
                UICommand::AppendToContainer { child: 3 },
            ],
        );
        assert_eq!(tree.root_children, vec![3]);
        assert_consistent(&tree);

        // Detach stale ids left dangling by ClearContainer.
        apply_all(
            &mut tree,
            [
                UICommand::DetachDeleted { id: 1 },
                UICommand::DetachDeleted { id: 2 },
            ],
        );
        assert!(!tree.nodes.contains_key(&1));
        assert!(!tree.nodes.contains_key(&2));
        assert_eq!(tree.nodes.len(), 2);
        assert_consistent(&tree);
    }

    // ---- 5. focusable tracking across a sequence ----

    #[test]
    fn focusable_tracking_across_sequence() {
        let mut tree = Tree::default();
        apply_command(&mut tree, keydown_view(1, false));
        assert_eq!(tree.focusable_ids.get(&1), Some(&false));

        // Remove handler → dropped from index.
        apply_command(
            &mut tree,
            UICommand::UpdateProps {
                id: 1,
                props: Props::default(),
            },
        );
        assert!(!tree.focusable_ids.contains_key(&1));

        // Re-add with autofocus → re-enters index with flag=true.
        let mut props = Props::default();
        props.events.keydown = true;
        props.autofocus = true;
        apply_command(&mut tree, UICommand::UpdateProps { id: 1, props });
        assert_eq!(tree.focusable_ids.get(&1), Some(&true));

        apply_command(&mut tree, UICommand::DetachDeleted { id: 1 });
        assert!(!tree.focusable_ids.contains_key(&1));
        assert_consistent(&tree);
    }

    // ---- 6. text input index across a sequence ----

    #[test]
    fn text_input_index_across_sequence() {
        let mut tree = Tree::default();
        apply_command(&mut tree, text_input(1));
        assert!(tree.text_input_ids.contains(&1));

        apply_command(&mut tree, UICommand::AppendToContainer { child: 1 });
        assert!(tree.text_input_ids.contains(&1));

        // RemoveFromContainer is visual only — index persists until DetachDeleted.
        apply_command(&mut tree, UICommand::RemoveFromContainer { child: 1 });
        assert!(tree.text_input_ids.contains(&1));

        apply_command(&mut tree, UICommand::DetachDeleted { id: 1 });
        assert!(!tree.text_input_ids.contains(&1));
        assert!(!tree.nodes.contains_key(&1));
        assert_consistent(&tree);
    }
}
