//! Per-node lifecycle teardown seam.
//!
//! Registry-owning modules register a detach hook (run when a node id is
//! destroyed) and a reload hook (run on dev-mode full reload), instead of
//! state.rs reaching into each module. `reset_for_reload` becomes "fire every
//! reload hook"; the `DetachDeleted` arm becomes "fire every detach hook".

use std::cell::RefCell;

use crate::model::ElementId;

thread_local! {
    static ON_DETACH: RefCell<Vec<fn(ElementId)>> = const { RefCell::new(Vec::new()) };
    #[cfg(debug_assertions)]
    static ON_RELOAD: RefCell<Vec<fn()>> = const { RefCell::new(Vec::new()) };
}

/// Register a hook run for each destroyed node id (see [`detach_node`]).
pub(crate) fn on_detach(hook: fn(ElementId)) {
    ON_DETACH.with(|h| h.borrow_mut().push(hook));
}

/// Register a hook run on dev-mode full reload (see [`reload`]).
#[cfg(debug_assertions)]
pub(crate) fn on_reload(hook: fn()) {
    ON_RELOAD.with(|h| h.borrow_mut().push(hook));
}

/// Fire every detach hook for `id`. Clones the hook list first so a hook can't
/// trip a re-entrant borrow.
pub(crate) fn detach_node(id: ElementId) {
    let hooks = ON_DETACH.with(|h| h.borrow().clone());
    for hook in hooks {
        hook(id);
    }
}

/// Fire every reload hook (dev-mode full reload).
#[cfg(debug_assertions)]
pub(crate) fn reload() {
    let hooks = ON_RELOAD.with(|h| h.borrow().clone());
    for hook in hooks {
        hook();
    }
}

/// (Re)install all built-in lifecycle hooks. Idempotent: clears first, so it is
/// safe to call on every context build (including dev reload).
pub(crate) fn install() {
    ON_DETACH.with(|h| h.borrow_mut().clear());
    #[cfg(debug_assertions)]
    ON_RELOAD.with(|h| h.borrow_mut().clear());
    crate::scrollbar::register_lifecycle();
    crate::render::register_lifecycle();
    crate::anim::register_lifecycle();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    thread_local! {
        static DETACHED: Cell<Option<ElementId>> = const { Cell::new(None) };
        static RELOADED: Cell<bool> = const { Cell::new(false) };
    }

    fn record_detach(id: ElementId) {
        DETACHED.with(|c| c.set(Some(id)));
    }

    #[cfg(debug_assertions)]
    fn record_reload() {
        RELOADED.with(|c| c.set(true));
    }

    #[test]
    fn detach_fires_registered_hooks_with_id() {
        // Don't rely on cross-test isolation: clear first, register, fire, assert.
        ON_DETACH.with(|h| h.borrow_mut().clear());
        DETACHED.with(|c| c.set(None));

        on_detach(record_detach);
        detach_node(42);
        assert_eq!(DETACHED.with(|c| c.get()), Some(42));

        ON_DETACH.with(|h| h.borrow_mut().clear());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn reload_fires_registered_hooks() {
        ON_RELOAD.with(|h| h.borrow_mut().clear());
        RELOADED.with(|c| c.set(false));

        on_reload(record_reload);
        reload();
        assert!(RELOADED.with(|c| c.get()));

        ON_RELOAD.with(|h| h.borrow_mut().clear());
    }
}
