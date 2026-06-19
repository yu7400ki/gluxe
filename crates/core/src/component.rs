// Native component system — register custom GPUI render functions addressable
// from React as host element types (mirrors `plugin.rs`).
//
// Host side:
//   Component::new("Badge", |ctx| vec![
//       gpui::div().child(ctx.props["label"].as_str().unwrap_or("")).into_any_element()
//   ])
// JS side:  <Badge label="hi" style={{ ... }} />  (any string type routes through the bridge)
//
// Rendering is **hybrid**: the framework wraps the output in a `Stateful<Div>`
// that carries `style` / pseudo-selectors / event listeners (like `<View>`);
// the component function produces only the *inner content*. See `render_node` (`Native` arm).
// Components are **stateless** — no per-instance GPUI Entity.

use std::{cell::RefCell, collections::HashMap};

use gpui::AnyElement;
use serde_json::Value;

/// Context passed to a native component's render function.
pub struct NativeRenderContext<'a> {
    /// Raw JS props (event handlers stripped) as JSON; `Null` when none were set.
    pub props: &'a Value,
    /// Reconciler children, already rendered; component decides how to place them.
    pub children: Vec<AnyElement>,
}

/// Boxed render function for a native component.
///
/// `Fn` (not `FnMut`) so it can be called from a shared borrow of the registry.
/// `'static` because it lives in a `thread_local!`.
pub type NativeRenderFn = Box<dyn Fn(NativeRenderContext) -> Vec<AnyElement> + 'static>;

/// A named native component addressable from JS by its element type string.
pub struct Component {
    pub(crate) name: String,
    pub(crate) render: NativeRenderFn,
}

impl Component {
    /// Register a native component under `name` (the JS element type string).
    pub fn new(
        name: impl Into<String>,
        render: impl Fn(NativeRenderContext) -> Vec<AnyElement> + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            render: Box::new(render),
        }
    }
}

thread_local! {
    /// Element-type name → render function; populated before the bundle is eval'd.
    static COMPONENTS: RefCell<HashMap<String, NativeRenderFn>> =
        RefCell::new(HashMap::new());
}

/// Register the framework's built-in native components (reserved `__` names that
/// apps can't shadow). Called before [`register_components`] so the reserved
/// types resolve during the very first reconciliation pass.
pub(crate) fn register_builtin_components() {
    COMPONENTS.with(|c| {
        c.borrow_mut().insert(
            "__GluxeScrollbar".to_string(),
            Box::new(|ctx: NativeRenderContext| crate::scrollbar::build_scrollbar(ctx.props)),
        );
    });
}

/// Insert components into the registry. Must be called before the JS bundle
/// is evaluated so `parse_kind` can resolve native types during reconciliation.
///
/// The reserved `__` namespace is owned by [`register_builtin_components`]; user
/// components must not use it (mirrors the plugin-name rule in `plugin.rs`).
pub(crate) fn register_components(list: Vec<Component>) {
    COMPONENTS.with(|c| {
        let mut map = c.borrow_mut();
        for component in list {
            assert!(
                !component.name.starts_with("__"),
                "component name `{}` is reserved (the `__` prefix is for built-ins)",
                component.name
            );
            map.insert(component.name, component.render);
        }
    });
}

/// Whether `name` is a registered native component (`bridge::parse_kind` uses this).
pub(crate) fn is_registered(name: &str) -> bool {
    COMPONENTS.with(|c| c.borrow().contains_key(name))
}

/// Invoke the render function for `name`; returns `None` if not registered.
///
/// Registry is held with a shared `borrow()` for the duration of the call.
/// Re-entrancy is safe: `ctx.children` are already rendered before this is
/// called, so a native component never re-borrows `COMPONENTS`.
pub(crate) fn render(name: &str, ctx: NativeRenderContext) -> Option<Vec<AnyElement>> {
    COMPONENTS.with(|c| {
        let map = c.borrow();
        map.get(name).map(|render| render(ctx))
    })
}
