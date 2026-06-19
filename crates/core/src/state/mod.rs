use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    sync::atomic::{AtomicU64, Ordering},
};

use std::time::Duration;

use boa_engine::context::time::JsInstant;
use boa_engine::{Context as JsContext, JsObject, JsString, JsValue, js_string};
use gpui::{App, AppContext, Entity, FocusHandle, Focusable, ScrollHandle, Window};

use crate::{
    jobs::GpuiJobExecutor,
    model::{ApplyOutcome, ElementId, Tree, UICommand, apply_command},
    text_input::TextInputState,
};

mod invoke;

// The cross-thread async / stream invoke plumbing lives in `invoke.rs`. Re-export
// its public surface so callers keep using `crate::state::{…}` unchanged.
pub(crate) use invoke::{
    bg_executor, cancel_stream, enqueue_invoke_result, error_stream, resolve_pending_invokes,
    resolve_pending_streams, set_bg_executor, signal_stream_wake, signal_wake, spawn_async_command,
    spawn_stream_command, wake_receiver,
};

// ---------------------------------------------------------------------------
// Window command queue
// ---------------------------------------------------------------------------

/// Commands that the JS side can issue to the native window at runtime.
///
/// Queued by plugin handlers (synchronous, on the main thread) and drained by
/// the pump loop each pass via [`take_window_commands`], which routes each
/// variant through `root.update(cx, |_, window, _| …)` to reach `&mut Window`.
pub(crate) enum WindowCommand {
    /// Change the window title bar text.
    SetTitle(String),
    /// Move keyboard focus to the element with this id (no-op if not focusable).
    FocusElement(ElementId),
    /// Focus the first tab-stop focusable inside this subtree (or the element
    /// itself when it has none). Processed after the command flush, so the just-
    /// mounted subtree is visible — the race-free "focus into a scope on open".
    FocusFirstIn(ElementId),
    /// Remove focus from the element with this id (only if it currently holds focus).
    BlurElement(ElementId),
}

thread_local! {
    /// Never reset on hot reload — title changes issued during old-bundle unmount
    /// still apply to the still-open window.
    static WINDOW_COMMANDS: RefCell<Vec<WindowCommand>> = const { RefCell::new(Vec::new()) };
    /// Programmatic `focus()` requests whose handle isn't painted yet, with a
    /// remaining retry budget. Retried one-per-frame until resolved or exhausted.
    static PENDING_FOCUS: RefCell<Vec<(ElementId, u8)>> = const { RefCell::new(Vec::new()) };
    /// Element Tab navigation should resume from when focus falls to nothing / the root.
    static FOCUS_ANCHOR: Cell<Option<ElementId>> = const { Cell::new(None) };
    /// Element holding keyboard focus as of the last paint (any kind: View/Image/
    /// TextInput), or `None` for the root fallback / nothing. Refreshed every
    /// `RootView::render`; read synchronously by `__bridge.getActiveElement()`.
    static ACTIVE_ELEMENT: Cell<Option<ElementId>> = const { Cell::new(None) };
    /// Stack of active Tab scopes (subtree roots). While non-empty, `FocusNext`/
    /// `FocusPrev` confine Tab to the innermost scope — the runtime `inert`. A stack
    /// so a nested overlay restores the outer scope on close.
    static TAB_SCOPE_STACK: RefCell<Vec<ElementId>> = const { RefCell::new(Vec::new()) };
}

/// Confine Tab to `id`'s subtree (push onto the scope stack).
pub(crate) fn push_tab_scope(id: ElementId) {
    TAB_SCOPE_STACK.with(|s| s.borrow_mut().push(id));
}

/// Release a Tab scope. Removes `id` by value (not just the top) so out-of-order
/// unmounts can't strand a scope.
pub(crate) fn pop_tab_scope(id: ElementId) {
    TAB_SCOPE_STACK.with(|s| {
        let mut stack = s.borrow_mut();
        if let Some(pos) = stack.iter().rposition(|&x| x == id) {
            stack.remove(pos);
        }
    });
}

/// The innermost active Tab scope, if any.
pub(crate) fn active_tab_scope() -> Option<ElementId> {
    TAB_SCOPE_STACK.with(|s| s.borrow().last().copied())
}

/// Next focus for Tab (`prev=false`) / Shift+Tab (`prev=true`) within a scope of
/// tab order `order` and current focus `current`. Wraps at the ends; `current`
/// outside `order` → first/last (recover into the scope). `None` iff `order`
/// empty. Pure — unit-tested.
pub(crate) fn scope_tab_target(
    order: &[ElementId],
    current: Option<ElementId>,
    prev: bool,
) -> Option<ElementId> {
    let len = order.len();
    if len == 0 {
        return None;
    }
    let idx = current.and_then(|c| order.iter().position(|&x| x == c));
    let next = match idx {
        Some(i) if prev => (i + len - 1) % len,
        Some(i) => (i + 1) % len,
        None if prev => len - 1,
        None => 0,
    };
    Some(order[next])
}

/// Record the element holding focus as of the current paint (called from
/// `RootView::render`). `None` clears it.
pub(crate) fn set_active_element(id: Option<ElementId>) {
    ACTIVE_ELEMENT.with(|c| c.set(id));
}

/// The element holding keyboard focus as of the last paint, if any. Backs the
/// synchronous JS `getActiveElement()` (used to save/restore focus around modals).
pub(crate) fn active_element() -> Option<ElementId> {
    ACTIVE_ELEMENT.with(|c| c.get())
}

/// Set/clear the Tab resume anchor.
pub(crate) fn set_focus_anchor(id: Option<ElementId>) {
    FOCUS_ANCHOR.with(|c| c.set(id));
}

/// The Tab resume anchor, if any.
pub(crate) fn focus_anchor() -> Option<ElementId> {
    FOCUS_ANCHOR.with(|c| c.get())
}

/// Frames to retry a deferred `focus()` before giving up (handle normally appears next frame).
pub(crate) const FOCUS_RETRY_BUDGET: u8 = 5;

/// Enqueue a window command and wake the render pump. Main thread only.
pub(crate) fn push_window_command(cmd: WindowCommand) {
    WINDOW_COMMANDS.with(|q| q.borrow_mut().push(cmd));
    signal_wake();
}

/// Queue a focus retry for a later pass with `retries` attempts left.
pub(crate) fn defer_focus(id: ElementId, retries: u8) {
    PENDING_FOCUS.with(|q| q.borrow_mut().push((id, retries)));
}

/// Drain the focus retry list for this pass.
pub(crate) fn take_pending_focus() -> Vec<(ElementId, u8)> {
    PENDING_FOCUS.with(|q| std::mem::take(&mut *q.borrow_mut()))
}

/// Whether any focus retries remain.
pub(crate) fn has_pending_focus() -> bool {
    PENDING_FOCUS.with(|q| !q.borrow().is_empty())
}

pub(crate) fn take_window_commands() -> Vec<WindowCommand> {
    WINDOW_COMMANDS.with(|q| std::mem::take(&mut *q.borrow_mut()))
}

// ---------------------------------------------------------------------------
// Thread-local runtime state
// ---------------------------------------------------------------------------

// Per-element / per-bundle registries added here must also be reset in
// `reset_for_reload` (dev-mode hot reload).
thread_local! {
    static ID_COUNTER: Cell<u64> = const { Cell::new(1) };
    static COMMAND_QUEUE: RefCell<VecDeque<UICommand>> = const { RefCell::new(VecDeque::new()) };
    static TREE: RefCell<Tree> = RefCell::new(Tree::default());
    // Live Boa context — held here so the foreground pump can call run_jobs().
    static BOA: RefCell<Option<JsContext>> = const { RefCell::new(None) };
    // ScrollHandle/Entity/FocusHandle are Rc-backed (!Send), so thread_local is
    // the correct home. Handles survive re-renders to preserve scroll position,
    // caret/IME state, and keyboard focus across frames.
    static SCROLL_HANDLES: RefCell<HashMap<ElementId, ScrollHandle>> =
        RefCell::new(HashMap::new());
    static TEXT_INPUTS: RefCell<HashMap<ElementId, Entity<TextInputState>>> =
        RefCell::new(HashMap::new());
    // FocusHandle is also looked up from `render_node` (which has no `&mut App`),
    // so it must be pre-created in the render pre-pass.
    static FOCUS_HANDLES: RefCell<HashMap<ElementId, FocusHandle>> =
        RefCell::new(HashMap::new());
    // Tracks which elements have already received their initial autoFocus so we
    // only steal focus once, not on every re-render.
    static AUTO_FOCUSED: RefCell<HashSet<ElementId>> = RefCell::new(HashSet::new());
    // Deadline of the most recently armed background wake timer for JS timers.
    // In-flight status is tracked by generation counters in `arm_next_timer`,
    // not by comparing this against the clock (see there for why).
    static ARMED_DEADLINE: Cell<Option<JsInstant>> = const { Cell::new(None) };
    // JsObject is a rooted boa_gc handle, so storing it here keeps callbacks alive.
    static RAF_QUEUE: RefCell<Vec<(u64, JsObject)>> = const { RefCell::new(Vec::new()) };
    static RAF_NEXT_ID: Cell<u64> = const { Cell::new(1) };
    // Ids cancelled while their batch was already being drained (same-frame
    // cancel from inside a callback). Checked before each call, cleared per batch.
    static RAF_CANCELLED: RefCell<HashSet<u64>> = RefCell::new(HashSet::new());
    // RAF_FRAME_ARMED: at most one on_next_frame callback in flight at a time.
    static RAF_FRAME_ARMED: Cell<bool> = const { Cell::new(false) };
    // Set by the frame callback — signals the pump to run the rAF batch.
    static RAF_FRAME_FIRED: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn next_id() -> ElementId {
    ID_COUNTER.with(|counter| {
        let id = counter.get();
        counter.set(id + 1);
        id
    })
}

pub(crate) fn push_cmd(cmd: UICommand) {
    COMMAND_QUEUE.with(|queue| queue.borrow_mut().push_back(cmd));
}

/// Drop state-owned per-element registries for a destroyed node `id`. The
/// cross-module cleanup (scrollbar/render/anim) runs via `lifecycle::detach_node`.
fn release_element(id: ElementId) {
    // Release the ScrollHandle, TextInput entity, and FocusHandle so the
    // Rc-chains are fully dropped and we don't accumulate stale handles.
    SCROLL_HANDLES.with(|h| {
        h.borrow_mut().remove(&id);
    });
    TEXT_INPUTS.with(|i| {
        i.borrow_mut().remove(&id);
    });
    FOCUS_HANDLES.with(|h| {
        h.borrow_mut().remove(&id);
    });
    AUTO_FOCUSED.with(|s| {
        s.borrow_mut().remove(&id);
    });
    // Drop a stale Tab resume-anchor / active-element pointing at the removed
    // element (recomputed next render, but clear now so a query between detach
    // and the next paint can't return a dead id).
    if FOCUS_ANCHOR.with(|c| c.get()) == Some(id) {
        FOCUS_ANCHOR.with(|c| c.set(None));
    }
    if ACTIVE_ELEMENT.with(|c| c.get()) == Some(id) {
        ACTIVE_ELEMENT.with(|c| c.set(None));
    }
    // Defensive: drop a Tab scope whose root was detached without unmount cleanup.
    pop_tab_scope(id);
}

/// Flush COMMAND_QUEUE → TREE; returns changed render targets when anything visible changed.
pub(crate) fn flush_commands() -> Option<ApplyOutcome> {
    let cmds: Vec<UICommand> = COMMAND_QUEUE.with(|queue| queue.borrow_mut().drain(..).collect());
    if cmds.is_empty() {
        return None;
    }

    let now_ms = clock_now_ms();
    let outcome = TREE.with(|tree| {
        let mut tree = tree.borrow_mut();
        let mut outcome = ApplyOutcome::default();
        for cmd in cmds {
            // When a node is destroyed, release its state-owned registries and
            // fire the per-node detach hooks (scrollbar/render/anim self-register).
            if let UICommand::DetachDeleted { id } = &cmd {
                release_element(*id);
                crate::lifecycle::detach_node(*id);
            }
            // Start/replace/cancel style transitions before `apply_command`
            // swaps the props in (it needs the old style to diff against).
            if let UICommand::UpdateProps { id, props } = &cmd
                && let Some(old) = tree.nodes.get(id)
                && old.props != *props
            {
                crate::anim::on_props_update(*id, &old.props.style, props, now_ms);
            }
            outcome.merge(apply_command(&mut tree, cmd));
        }
        outcome
    });
    if outcome.is_empty() {
        None
    } else {
        Some(outcome)
    }
}

/// Return (or create) the `ScrollHandle` for `id`. Returns a cheap `Rc` clone;
/// the registry copy remains the authoritative scroll-state owner.
pub(crate) fn scroll_handle(id: ElementId) -> ScrollHandle {
    SCROLL_HANDLES.with(|handles| {
        handles
            .borrow_mut()
            .entry(id)
            .or_insert_with(ScrollHandle::default)
            .clone()
    })
}

/// Return (or create) the `Entity<TextInputState>` for `id`. Must be called from
/// a GPUI render/update callback (entity creation requires `&mut App`).
pub(crate) fn text_input_entity(id: ElementId, cx: &mut App) -> Entity<TextInputState> {
    TEXT_INPUTS.with(|inputs| {
        let existing = inputs.borrow().get(&id).cloned();
        if let Some(entity) = existing {
            return entity;
        }
        let (initial_value, initial_placeholder) = with_tree(|tree| {
            tree.nodes
                .get(&id)
                .map(|e| (e.props.value.clone(), e.props.placeholder.clone()))
                .unwrap_or_default()
        });
        let entity = cx.new(|cx| TextInputState::new(id, initial_value, initial_placeholder, cx));
        inputs.borrow_mut().insert(id, entity.clone());
        entity
    })
}

pub(crate) fn notify_text_input_entity(id: ElementId, cx: &mut App) -> bool {
    let entity = TEXT_INPUTS.with(|inputs| inputs.borrow().get(&id).cloned());
    let Some(entity) = entity else {
        return false;
    };
    entity.update(cx, |_, cx| cx.notify());
    true
}

pub(crate) fn with_tree<R>(f: impl FnOnce(&Tree) -> R) -> R {
    TREE.with(|tree| f(&tree.borrow()))
}

pub(crate) fn set_boa(ctx: JsContext) {
    BOA.with(|boa| *boa.borrow_mut() = Some(ctx));
}

/// Take the live Boa context out for dev-mode reload. When dropped, its
/// `GpuiJobExecutor` and any parked timers are released with it.
#[cfg(debug_assertions)]
pub(crate) fn take_boa() -> Option<JsContext> {
    BOA.with(|boa| boa.borrow_mut().take())
}

/// Dev-mode reload rollback: discard UI commands from a failed bundle eval that
/// must never reach the tree.
#[cfg(debug_assertions)]
pub(crate) fn clear_command_queue() {
    COMMAND_QUEUE.with(|queue| queue.borrow_mut().clear());
}

/// Dev-mode: reloader refuses to start a reload while old-context commands are
/// still queued — they must flush into the old tree, not the post-reload one.
#[cfg(debug_assertions)]
pub(crate) fn command_queue_len() -> usize {
    COMMAND_QUEUE.with(|queue| queue.borrow().len())
}

/// Reset per-bundle runtime state for a dev-mode full reload.
///
/// Called after the new bundle is evaluated and the context swapped. The whole
/// `Tree` is replaced — old nodes would otherwise leak since no `DetachDeleted`
/// is issued for them.
///
/// **Contract for contributors:** every `thread_local` holding per-element state
/// (keyed by `ElementId`) or rooted `JsObject`s from the old `JsContext` must be
/// cleared here. Omitting it causes stale state or memory leaks that only appear
/// under dev-mode hot reload.
///
/// Deliberately untouched:
/// - `ID_COUNTER` — monotonic; new-mount ids must never collide with old ones
/// - `COMMAND_QUEUE` — holds the new bundle's mount commands at this point
/// - `WINDOW_COMMANDS` — title changes during unmount still apply to the open window
/// - wake/invoke channels, `BG_EXECUTOR`, plugin/component registries
/// - `RAF_FRAME_ARMED`/`RAF_FRAME_FIRED` — window-level frame handshake
#[cfg(debug_assertions)]
pub(crate) fn reset_for_reload() {
    TREE.with(|tree| *tree.borrow_mut() = Tree::default());
    SCROLL_HANDLES.with(|handles| handles.borrow_mut().clear());
    TEXT_INPUTS.with(|inputs| inputs.borrow_mut().clear());
    FOCUS_HANDLES.with(|handles| handles.borrow_mut().clear());
    AUTO_FOCUSED.with(|set| set.borrow_mut().clear());
    PENDING_FOCUS.with(|q| q.borrow_mut().clear());
    FOCUS_ANCHOR.with(|c| c.set(None));
    ACTIVE_ELEMENT.with(|c| c.set(None));
    TAB_SCOPE_STACK.with(|s| s.borrow_mut().clear());
    ARMED_DEADLINE.with(|a| a.set(None));
    // Wind down still-running stream handlers from the old bundle and bump the
    // stream epoch so their late messages are filtered out (see `invoke.rs`).
    invoke::reset_streams_for_reload();
    // Fire each owning module's reload hook (render/anim/scrollbar self-register
    // their dev-reload cleanup with the lifecycle seam).
    crate::lifecycle::reload();
}

/// Run `f` with exclusive access to the live Boa context.
///
/// The context is *moved out* of `BOA` for the duration of `f` and restored
/// on return (even on panic). While `f` runs, `BOA` holds `None`, so any nested
/// `with_boa` call safely no-ops instead of panicking on a double `borrow_mut`.
/// This makes holding the context across a JS `call`/`run_jobs` sound by
/// construction — no reachable code can re-borrow `BOA` while it is checked out.
///
/// `f` runs via `stack::with_js_stack` for native-stack headroom (see stack.rs).
/// All Boa calls must go through this choke point.
///
/// Returns `None` when no context is registered yet.
fn with_boa<R>(f: impl FnOnce(&mut JsContext) -> R) -> Option<R> {
    let ctx = BOA.with(|boa| boa.borrow_mut().take())?;

    // Restore on the way out even on panic.
    struct Restore(Option<JsContext>);
    impl Drop for Restore {
        fn drop(&mut self) {
            if let Some(ctx) = self.0.take() {
                BOA.with(|boa| *boa.borrow_mut() = Some(ctx));
            }
        }
    }

    let mut guard = Restore(Some(ctx));
    let ctx = guard.0.as_mut().expect("context set above");
    Some(crate::stack::with_js_stack(|| f(ctx)))
}

/// Log an uncaught JS error to stderr — debug builds only.
///
/// Errors from event handlers or the microtask queue have no meaningful recovery
/// on the GPUI side, but swallowing them silently can leave the UI frozen with
/// no diagnostic. Gated on `debug_assertions` to keep release builds quiet;
/// `err`/`context` are intentionally unused in release.
fn log_js_error(context: &str, err: &boa_engine::JsError) {
    #[cfg(debug_assertions)]
    eprintln!("[gluxe] uncaught JS error in {context}: {err}");
    #[cfg(not(debug_assertions))]
    let _ = (context, err);
}

pub(crate) fn run_boa_jobs() {
    with_boa(|js| {
        if let Err(err) = js.run_jobs() {
            log_js_error("microtask queue", &err);
        }
    });
}

/// Decide whether [`arm_next_timer`] should (re-)arm a background timer for `due`.
///
/// Returns `false` (skip arming) only when a timer is already `in_flight`, an
/// armed deadline exists, and `due` is at or after it — i.e. a live timer already
/// covers this deadline. When nothing is in flight, it always re-arms.
fn should_rearm<T: PartialOrd>(in_flight: bool, due: T, armed: Option<T>) -> bool {
    !(in_flight && matches!(armed, Some(a) if due >= a))
}

/// Arm a GPUI background timer to wake the pump when the next JS timer comes due.
///
/// [`GpuiJobExecutor`] parks not-yet-due clock jobs instead of busy-waiting.
/// This function is the other half: it reads the earliest parked deadline and
/// schedules a one-shot background timer that calls [`signal_wake`], so the pump
/// re-enters `run_boa_jobs` and the now-due job fires.
///
/// In-flight status is tracked by generation counters (never by comparing the
/// armed deadline against the clock). An OS timer may wake the pump marginally
/// *before* the Boa clock reaches the deadline; a clock comparison would then
/// misclassify the spent timer as still live and skip re-arming, permanently
/// stalling hands-off `setTimeout` loops (e.g. 2048's auto mode).
pub(crate) fn arm_next_timer() {
    // Process-global atomics (not thread_locals): the firing task runs on a
    // background thread. FIRED advances via `fetch_max` so an older timer firing
    // late can never mark a newer in-flight one as spent. A newer timer firing
    // first marking older ones spent only causes a redundant re-arm — the safe
    // direction.
    static ARMED_GEN: AtomicU64 = AtomicU64::new(0);
    static FIRED_GEN: AtomicU64 = AtomicU64::new(0);

    let due_now = with_boa(|js| {
        let exec = js.downcast_job_executor::<GpuiJobExecutor>()?;
        let due = exec.next_due()?;
        Some((due, js.clock().now()))
    })
    .flatten();

    let Some((due, now)) = due_now else {
        ARMED_DEADLINE.with(|a| a.set(None));
        return;
    };

    let in_flight = FIRED_GEN.load(Ordering::Relaxed) < ARMED_GEN.load(Ordering::Relaxed);
    let armed = ARMED_DEADLINE.with(|a| a.get());
    if !should_rearm(in_flight, due, armed) {
        // A live timer covers this deadline; its wake will re-arm if needed.
        return;
    }

    let executor = bg_executor();
    let Some(executor) = executor else {
        return; // pump not started yet; first tick will arm
    };

    // Nanosecond precision: truncating to millis would fire ~1 ms early,
    // causing an extra parked-job wake round-trip on nearly every timer.
    let delay = Duration::from_nanos(
        due.nanos_since_epoch()
            .saturating_sub(now.nanos_since_epoch())
            .min(u64::MAX as u128) as u64,
    );
    let generation = ARMED_GEN.fetch_add(1, Ordering::Relaxed) + 1;
    let timer = executor.timer(delay);
    executor
        .spawn(async move {
            timer.await;
            // Mark spent before waking so the triggered pump pass sees the slot
            // free and can re-arm if the job is still not due (early OS wakeup).
            FIRED_GEN.fetch_max(generation, Ordering::Relaxed);
            signal_wake();
        })
        .detach();
    ARMED_DEADLINE.with(|a| a.set(Some(due)));
}

// ---------------------------------------------------------------------------
// requestAnimationFrame plumbing
// ---------------------------------------------------------------------------
//
// Per-frame handshake:
//   raf_request      → queue callback + wake pump
//   raf_try_arm      → pump installs window.on_next_frame(raf_frame_fired) + cx.notify()
//   raf_frame_fired  → flag batch due + wake pump
//   run_raf_callbacks→ invoke batch with one shared timestamp
//
// Idle windows with no pending rAF cost zero wakeups.

/// Register a rAF callback; returns its monotonically increasing id.
pub(crate) fn raf_request(cb: JsObject) -> u64 {
    let id = RAF_NEXT_ID.with(|c| {
        let id = c.get();
        c.set(id + 1);
        id
    });
    RAF_QUEUE.with(|q| q.borrow_mut().push((id, cb)));
    signal_wake();
    id
}

/// Snapshot queue length before a dev-mode reload eval so rollback/commit can
/// distinguish old-context entries from new ones.
#[cfg(debug_assertions)]
pub(crate) fn raf_queue_len() -> usize {
    RAF_QUEUE.with(|q| q.borrow().len())
}

/// Reload rollback: drop rAF entries appended by a failed eval (their
/// `JsObject`s belong to the discarded context).
#[cfg(debug_assertions)]
pub(crate) fn raf_truncate(len: usize) {
    RAF_QUEUE.with(|q| q.borrow_mut().truncate(len));
}

/// Reload commit: drop the first `len` rAF entries (old-context; must not be
/// invoked via the new context) and clear pending cancellations (old-context ids).
#[cfg(debug_assertions)]
pub(crate) fn raf_drop_front(len: usize) {
    RAF_QUEUE.with(|q| {
        q.borrow_mut().drain(..len);
    });
    RAF_CANCELLED.with(|s| s.borrow_mut().clear());
}

/// Cancel a pending rAF callback. The id is remembered so a cancellation issued
/// from inside the running batch still suppresses sibling callbacks in that frame.
pub(crate) fn raf_cancel(id: u64) {
    RAF_QUEUE.with(|q| q.borrow_mut().retain(|(i, _)| *i != id));
    RAF_CANCELLED.with(|s| {
        s.borrow_mut().insert(id);
    });
}

/// Returns `true` when frame-driven work is pending and no frame callback is
/// armed yet, atomically marking one as armed. The caller must then install
/// `window.on_next_frame(|_, _| raf_frame_fired())`.
pub(crate) fn raf_try_arm() -> bool {
    let pending = RAF_QUEUE.with(|q| !q.borrow().is_empty()) || crate::anim::has_active();
    if pending && !RAF_FRAME_ARMED.with(|a| a.get()) {
        RAF_FRAME_ARMED.with(|a| a.set(true));
        true
    } else {
        false
    }
}

/// `on_next_frame` callback: flags the batch as due and wakes the pump.
pub(crate) fn raf_frame_fired() {
    RAF_FRAME_ARMED.with(|a| a.set(false));
    RAF_FRAME_FIRED.with(|f| f.set(true));
    signal_wake();
}

/// Consume the frame-fired flag once per pump pass so both the rAF batch and
/// `anim::tick` observe the same frame signal.
pub(crate) fn take_frame_fired() -> bool {
    RAF_FRAME_FIRED.with(|f| f.replace(false))
}

/// Boa monotonic clock in fractional ms (same as rAF timestamps). `None` before `set_boa()`.
pub(crate) fn clock_now_ms() -> Option<f64> {
    with_boa(|js| js.clock().now().nanos_since_epoch() as f64 / 1_000_000.0)
}

/// Run the pending rAF batch. No-op unless `frame_fired` — prevents
/// timer/invoke wakes from firing callbacks off-frame.
///
/// The queue is `mem::take`n before invoking so callbacks registered inside the
/// batch land in the fresh queue (browser semantics) with no `RefCell` held
/// across JS calls.
pub(crate) fn run_raf_callbacks(frame_fired: bool) {
    if !frame_fired {
        return;
    }
    let batch = RAF_QUEUE.with(|q| std::mem::take(&mut *q.borrow_mut()));
    if batch.is_empty() {
        RAF_CANCELLED.with(|s| s.borrow_mut().clear());
        return;
    }
    with_boa(|js| {
        // One timestamp for the whole batch (rAF spec), on the same clock as JS timers.
        let ts = js.clock().now().nanos_since_epoch() as f64 / 1_000_000.0;
        for (id, cb) in batch {
            if RAF_CANCELLED.with(|s| s.borrow_mut().remove(&id)) {
                continue;
            }
            if let Err(err) = cb.call(&JsValue::undefined(), &[JsValue::from(ts)], js) {
                log_js_error("requestAnimationFrame callback", &err);
            }
        }
    });
    RAF_CANCELLED.with(|s| s.borrow_mut().clear());
}

// ---------------------------------------------------------------------------
// Event dispatch: GPUI → Boa → JS handler → setState → command queue
// ---------------------------------------------------------------------------

/// Call a named global JS function, ignoring its return value. No-ops if the
/// global is missing or not callable. Does not run the job queue.
fn call_global(js: &mut JsContext, name: &str, args: &[JsValue]) {
    let global = js.global_object();
    let Ok(callable) = global.get(JsString::from(name), js) else {
        return;
    };
    if let Some(func) = callable.as_callable()
        && let Err(err) = func.call(&JsValue::undefined(), args, js)
    {
        log_js_error(name, &err);
    }
}

/// Invoke `__dispatchEvent` and flush microtasks. Wakes the pump so UICommands
/// queued by the handler and its microtasks are applied on the next pass.
/// Does not call `flush_commands` — the pump does that after `cx.notify()`.
fn call_dispatch(js: &mut JsContext, args: &[JsValue]) {
    call_global(js, "__dispatchEvent", args);
    if let Err(err) = js.run_jobs() {
        log_js_error("microtask queue", &err);
    }
    signal_wake();
}

/// Per-event-kind payload for `__dispatchEvent`. JS spreads the object into the
/// handler's event argument (`{ type, target, ...payload }`), so field names are
/// part of the wire contract with host-config.ts.
pub(crate) enum EventPayload<'a> {
    /// Events with no extra fields (View/Image focus/blur). JS receives `{ type, target }`.
    Empty,
    /// Mouse events: cursor position within the window.
    Mouse { x: f32, y: f32 },
    /// Text events (change/submit/focus/blur): current input text.
    Value(&'a str),
    /// Keyboard events (keydown): key name + modifier flags.
    Key {
        key: &'a str,
        shift: bool,
        ctrl: bool,
        alt: bool,
        meta: bool,
    },
}

/// Build the payload object and call `__dispatchEvent` for `id`. Goes through
/// `with_boa` so re-entrant dispatch safely no-ops instead of double-borrowing `BOA`.
fn dispatch_event_with_payload(id: ElementId, event_type: &str, payload: EventPayload<'_>) {
    with_boa(|js| {
        let obj = JsObject::with_object_proto(js.intrinsics());
        match payload {
            EventPayload::Empty => {}
            EventPayload::Mouse { x, y } => {
                let _ = obj.set(js_string!("x"), f64::from(x), false, js);
                let _ = obj.set(js_string!("y"), f64::from(y), false, js);
            }
            EventPayload::Value(value) => {
                let _ = obj.set(js_string!("value"), JsString::from(value), false, js);
            }
            EventPayload::Key {
                key,
                shift,
                ctrl,
                alt,
                meta,
            } => {
                let _ = obj.set(js_string!("key"), JsString::from(key), false, js);
                let _ = obj.set(js_string!("shift"), shift, false, js);
                let _ = obj.set(js_string!("ctrl"), ctrl, false, js);
                let _ = obj.set(js_string!("alt"), alt, false, js);
                let _ = obj.set(js_string!("meta"), meta, false, js);
            }
        }
        let args = [
            JsValue::from(id as f64),
            JsValue::from(JsString::from(event_type)),
            JsValue::from(obj),
        ];
        call_dispatch(js, &args);
    });
}

pub(crate) fn dispatch_mouse_event(id: ElementId, event_type: &str, x: f32, y: f32) {
    dispatch_event_with_payload(id, event_type, EventPayload::Mouse { x, y });
}

pub(crate) fn dispatch_value_event(id: ElementId, event_type: &str, value: &str) {
    dispatch_event_with_payload(id, event_type, EventPayload::Value(value));
}

/// Dispatch an event carrying no extra fields (e.g. View/Image `focus`/`blur`).
/// JS handler receives `{ type, target }`.
pub(crate) fn dispatch_simple_event(id: ElementId, event_type: &str) {
    dispatch_event_with_payload(id, event_type, EventPayload::Empty);
}

pub(crate) fn dispatch_key_event(
    id: ElementId,
    key: &str,
    shift: bool,
    ctrl: bool,
    alt: bool,
    meta: bool,
) {
    dispatch_event_with_payload(
        id,
        "keydown",
        EventPayload::Key {
            key,
            shift,
            ctrl,
            alt,
            meta,
        },
    );
}

// ---------------------------------------------------------------------------
// Focus registry: keyboard-focusable View/Image elements
// ---------------------------------------------------------------------------

/// Return (or create) the `FocusHandle` for `id`. Must be called from a GPUI
/// render/update callback (handle creation requires `&mut App`).
pub(crate) fn focus_handle(id: ElementId, cx: &mut App) -> FocusHandle {
    FOCUS_HANDLES.with(|handles| {
        let existing = handles.borrow().get(&id).cloned();
        if let Some(h) = existing {
            return h;
        }
        let h = cx.focus_handle();
        handles.borrow_mut().insert(id, h.clone());
        h
    })
}

/// Resolve the `FocusHandle` for any focusable element id, spanning both
/// View/Image (`FOCUS_HANDLES`) and `TextInput` (`TEXT_INPUTS`, whose handle lives
/// inside the entity). `None` if `id` isn't focusable or its handle isn't created
/// yet (element not painted) — callers fall back to a deferred retry.
///
/// Reads without creating; handles are created in the render pre-pass
/// (`focus_handle` / `text_input_entity`). View/Image is checked first; element
/// kinds are mutually exclusive so there is no id collision.
pub(crate) fn get_focus_handle(id: ElementId, cx: &App) -> Option<FocusHandle> {
    FOCUS_HANDLES
        .with(|handles| handles.borrow().get(&id).cloned())
        .or_else(|| {
            TEXT_INPUTS.with(|inputs| {
                inputs
                    .borrow()
                    .get(&id)
                    .map(|entity| entity.read(cx).focus_handle(cx))
            })
        })
}

/// The focusable View/Image that currently holds focus, if any (used to keep the
/// Tab resume anchor current). `None` for the root fallback / TextInput / nothing.
pub(crate) fn focused_element_id(window: &Window) -> Option<ElementId> {
    FOCUS_HANDLES.with(|handles| {
        handles
            .borrow()
            .iter()
            .find(|(_, handle)| handle.is_focused(window))
            .map(|(id, _)| *id)
    })
}

/// The element holding focus across BOTH handle stores — View/Image (`FOCUS_HANDLES`)
/// and `TextInput` (`TEXT_INPUTS`) — or `None` for the root fallback / nothing.
///
/// Unlike `focused_element_id` (View/Image-only, for the Tab anchor), this is the
/// full active-element query that backs `getActiveElement`. View/Image is checked
/// first; kinds are mutually exclusive so the order only affects the empty case.
pub(crate) fn active_element_id(window: &Window, cx: &App) -> Option<ElementId> {
    focused_element_id(window).or_else(|| {
        TEXT_INPUTS.with(|inputs| {
            inputs
                .borrow()
                .iter()
                .find(|(_, entity)| entity.read(cx).focus_handle(cx).is_focused(window))
                .map(|(id, _)| *id)
        })
    })
}

/// Tab-stop focusables in `root`'s subtree (root included when it qualifies), in
/// GPUI Tab order. Backs `__bridge.getFocusableElements`. Reads the tree as of the
/// last applied commands (like [`active_element`]) — commands queued in the
/// current JS task aren't visible yet. See [`Tree::focusable_descendants`].
pub(crate) fn focusable_descendants(root: ElementId) -> Vec<ElementId> {
    with_tree(|tree| tree.focusable_descendants(root))
}

/// The window-global Tab order (see [`Tree::focusable_order`]). Used by
/// `navigate_tab` when no Tab scope is active.
pub(crate) fn focusable_order() -> Vec<ElementId> {
    with_tree(|tree| tree.focusable_order())
}

/// Mark `id` as having received its initial autoFocus. Returns `true` the first
/// time for a given `id` (caller should then call `window.focus`), `false` on
/// subsequent calls (no-op to avoid stealing focus back on every re-render).
pub(crate) fn mark_autofocus(id: ElementId) -> bool {
    AUTO_FOCUSED.with(|set| set.borrow_mut().insert(id))
}

/// Focus the element at `id` in the given window, if its handle is registered.
///
/// Convenience wrapper used in the `RootView::render` pre-pass for `autoFocus`
/// elements. Guarded by `mark_autofocus` so focus is stolen at most once.
pub(crate) fn try_autofocus(id: ElementId, window: &mut Window, cx: &mut App) {
    if mark_autofocus(id)
        && let Some(handle) = get_focus_handle(id, cx)
    {
        window.focus(&handle, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Tab scope ----

    #[test]
    fn scope_tab_target_steps_and_wraps() {
        let order = [10, 20, 30];
        // Forward, wrapping past the end.
        assert_eq!(scope_tab_target(&order, Some(10), false), Some(20));
        assert_eq!(scope_tab_target(&order, Some(30), false), Some(10));
        // Backward, wrapping past the start.
        assert_eq!(scope_tab_target(&order, Some(20), true), Some(10));
        assert_eq!(scope_tab_target(&order, Some(10), true), Some(30));
    }

    #[test]
    fn scope_tab_target_recovers_when_focus_outside_scope() {
        let order = [10, 20, 30];
        assert_eq!(scope_tab_target(&order, Some(99), false), Some(10)); // → first
        assert_eq!(scope_tab_target(&order, None, true), Some(30)); // → last
    }

    #[test]
    fn scope_tab_target_empty_is_none() {
        assert_eq!(scope_tab_target(&[], Some(1), false), None);
    }

    #[test]
    fn tab_scope_stack_pushes_pops_by_value() {
        assert_eq!(active_tab_scope(), None);
        push_tab_scope(1);
        push_tab_scope(2);
        assert_eq!(active_tab_scope(), Some(2)); // innermost
        // Out-of-order pop removes by value, not just the top.
        pop_tab_scope(1);
        assert_eq!(active_tab_scope(), Some(2));
        pop_tab_scope(2);
        assert_eq!(active_tab_scope(), None);
        pop_tab_scope(99); // unknown id is a no-op
        assert_eq!(active_tab_scope(), None);
    }

    // ---- should_rearm ----

    #[test]
    fn should_rearm_when_not_in_flight() {
        // Not in flight: always re-arm, regardless of the armed deadline.
        assert!(should_rearm(false, 5u64, None));
        assert!(should_rearm(false, 5u64, Some(1)));
        assert!(should_rearm(false, 5u64, Some(10)));
    }

    #[test]
    fn should_not_rearm_when_in_flight_and_due_at_or_after_armed() {
        // A live timer already covers this deadline → skip arming.
        assert!(!should_rearm(true, 10u64, Some(5))); // due > armed
        assert!(!should_rearm(true, 5u64, Some(5))); // due == armed (>=)
    }

    #[test]
    fn should_rearm_when_in_flight_but_due_before_armed() {
        // An earlier deadline appeared; the in-flight timer won't cover it → re-arm.
        assert!(should_rearm(true, 3u64, Some(5)));
    }

    #[test]
    fn should_rearm_when_in_flight_but_no_armed_deadline() {
        // In flight but nothing recorded as armed → re-arm.
        assert!(should_rearm(true, 7u64, None::<u64>));
    }
}
