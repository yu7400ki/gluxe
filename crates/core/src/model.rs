use gpui::WindowControlArea;
use rustc_hash::{FxHashMap, FxHashSet};

// The style data vocabulary — `StyleFields`, `LengthValue`, the floating/overflow
// enums and their parsers — lives in `style/fields.rs` (the style system owns the
// types it parses and applies). `Props` below composes it; this re-export keeps
// callers that reach the vocabulary through `crate::model` resolving unchanged.
pub(crate) use crate::style::fields::{
    FloatingAlign, FloatingSide, FloatingSpec, LengthValue, OverflowMode, StyleFields,
};

// ---------------------------------------------------------------------------
// Core data model
// ---------------------------------------------------------------------------

pub(crate) type ElementId = u64;

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
    /// `<TextInput multiline>`: accept newlines and grow vertically. `false` →
    /// single-line (newlines stripped on paste, Enter submits).
    pub(crate) multiline: bool,
    /// `multiline` auto-grow floor / cap in rows. `None` → 1 / unbounded.
    pub(crate) min_rows: Option<u32>,
    pub(crate) max_rows: Option<u32>,
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
    /// in Tab order. Backs `getFocusableElements` and the active Tab scope.
    pub(crate) fn focusable_descendants(&self, root: ElementId) -> Vec<ElementId> {
        self.collect_tab_stops(std::iter::once(root))
    }

    /// The window-global Tab order: every tab stop across the root children, in the
    /// order Tab visits them. The single source of truth for `RootView::navigate_tab`
    /// when no Tab scope is active (gpui's own focus_next is not consulted).
    pub(crate) fn focusable_order(&self) -> Vec<ElementId> {
        self.collect_tab_stops(self.root_children.iter().copied())
    }

    /// Collect tab stops under `roots` in Tab order: ascending `tabIndex` (default
    /// `0`), ties by preorder (≈ paint order). `display:none` / `visibility:hidden`
    /// subtrees are skipped entirely, matching what GPUI leaves out of its tab map.
    fn collect_tab_stops(&self, roots: impl Iterator<Item = ElementId>) -> Vec<ElementId> {
        // (tab_index, preorder index, id). One shared counter across all roots so
        // the global order matches a single preorder walk.
        let mut found: Vec<(i32, usize, ElementId)> = Vec::new();
        let mut order: usize = 0;
        // Reverse so the first root is processed first (LIFO stack).
        let mut stack: Vec<ElementId> = roots.collect();
        stack.reverse();
        while let Some(id) = stack.pop() {
            let Some(node) = self.nodes.get(&id) else {
                continue;
            };
            let style = &node.props.style;
            // Hidden subtree: not laid out / not painted → not Tab-reachable. Skip
            // it and its descendants (no recursion).
            if style.display.as_deref() == Some("none")
                || style.visibility.as_deref() == Some("hidden")
            {
                continue;
            }
            let props = &node.props;
            // TextInput is intrinsically focusable; View/Image only with a focus
            // prop. The tab_stop default is single-sourced in `resolve_tab_stop`.
            let is_text_input = matches!(node.kind, ElementKind::TextInput);
            if (is_text_input || props.is_focusable()) && props.resolve_tab_stop(is_text_input) {
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
    fn focusable_order_sorts_globally_across_root_children() {
        let mut tree = Tree::default();
        // Root children in mount order: a tabIndex-5 stop first, then two tabIndex-0.
        apply_command(&mut tree, make_tabbable_instance(10, Some(5)));
        apply_command(&mut tree, make_tabbable_instance(11, Some(0)));
        apply_command(&mut tree, make_tabbable_instance(12, Some(0)));
        for child in [10, 11, 12] {
            apply_command(&mut tree, UICommand::AppendToContainer { child });
        }
        // tabIndex sorts globally: the two 0s (preorder) before the 5, even though
        // the 5 mounted first.
        assert_eq!(tree.focusable_order(), vec![11, 12, 10]);
    }

    #[test]
    fn collect_tab_stops_skips_hidden_subtrees() {
        for hide in ["display", "visibility"] {
            let mut tree = Tree::default();
            let mut props = Props::default();
            props.tab_index = Some(0); // the container is itself a stop...
            if hide == "display" {
                props.style.display = Some("none".to_string());
            } else {
                props.style.visibility = Some("hidden".to_string());
            }
            apply_command(
                &mut tree,
                UICommand::CreateInstance {
                    id: 1,
                    kind: ElementKind::View,
                    props,
                },
            );
            apply_command(&mut tree, make_tabbable_instance(2, Some(0))); // ...with a focusable child
            apply_command(
                &mut tree,
                UICommand::AppendChild {
                    parent: 1,
                    child: 2,
                },
            );
            // Both the hidden container and its subtree are out of the Tab order.
            assert!(tree.focusable_descendants(1).is_empty(), "{hide}");
        }
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
