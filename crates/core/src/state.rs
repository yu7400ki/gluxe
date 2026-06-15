use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
    },
};

use std::time::Duration;

use boa_engine::context::time::JsInstant;
use boa_engine::{Context as JsContext, JsObject, JsString, JsValue, js_string};
use gpui::{App, AppContext, BackgroundExecutor, Entity, FocusHandle, ScrollHandle, Window};

use crate::{
    jobs::GpuiJobExecutor,
    model::{ApplyOutcome, ElementId, Tree, UICommand, apply_command},
    plugin::{AsyncCommandHandler, CommandResult, StreamCommandHandler, StreamMessage, StreamSink},
    text_input::TextInputState,
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
    /// Remove focus from the element with this id (only if it currently holds focus).
    BlurElement(ElementId),
}

thread_local! {
    /// Never reset on hot reload — title changes issued during old-bundle unmount
    /// still apply to the still-open window.
    static WINDOW_COMMANDS: RefCell<Vec<WindowCommand>> = RefCell::new(Vec::new());
    /// Programmatic `focus()` requests whose handle isn't painted yet, with a
    /// remaining retry budget. Retried one-per-frame until resolved or exhausted.
    static PENDING_FOCUS: RefCell<Vec<(ElementId, u8)>> = RefCell::new(Vec::new());
    /// Element Tab navigation should resume from when focus falls to nothing / the root.
    static FOCUS_ANCHOR: Cell<Option<ElementId>> = const { Cell::new(None) };
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
    static COMMAND_QUEUE: RefCell<VecDeque<UICommand>> = RefCell::new(VecDeque::new());
    static TREE: RefCell<Tree> = RefCell::new(Tree::default());
    // Live Boa context — held here so the foreground pump can call run_jobs().
    static BOA: RefCell<Option<JsContext>> = RefCell::new(None);
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
    // GPUI background executor — clone stored here; only the resulting Send
    // future crosses threads, not the executor itself.
    static BG_EXECUTOR: RefCell<Option<BackgroundExecutor>> = const { RefCell::new(None) };
    // Per-live-stream cancel flags. The map lives on the main thread (registered
    // in `spawn_stream_command`, removed on terminal drain / `error_stream`); the
    // flag itself is an `Arc<AtomicBool>` shared with the sink on the background
    // thread so `__streamCancel` can ask a running handler to stop.
    static STREAM_CANCELS: RefCell<HashMap<u64, Arc<AtomicBool>>> = RefCell::new(HashMap::new());
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

// ---------------------------------------------------------------------------
// Async invoke plumbing
// ---------------------------------------------------------------------------

/// Process-global channel so background tasks can `send` invoke results without
/// depending on thread-local init order. `Receiver` is drained on the main thread only.
struct InvokeChannel {
    tx: Sender<(u64, String)>,
    rx: Mutex<Receiver<(u64, String)>>,
}

fn invoke_channel() -> &'static InvokeChannel {
    static INVOKE_RESULTS: OnceLock<InvokeChannel> = OnceLock::new();
    INVOKE_RESULTS.get_or_init(|| {
        let (tx, rx) = mpsc::channel();
        InvokeChannel {
            tx,
            rx: Mutex::new(rx),
        }
    })
}

// ---------------------------------------------------------------------------
// Streaming plumbing (`invokeStream`)
// ---------------------------------------------------------------------------

/// Bumped on every dev-mode hot reload ([`reset_for_reload`]). Each stream is
/// stamped with the epoch live when it was spawned; messages from a prior epoch
/// are dropped on the consume side. This prevents a stale message from an
/// old-bundle handler (whose `streamId` the fresh JS context — which restarts
/// ids at 1 — has since reused) from being delivered to an unrelated new stream.
static STREAM_EPOCH: AtomicU64 = AtomicU64::new(0);

/// Coalesces per-chunk wakes: a producer only signals the pump when no wake is
/// already pending, collapsing a burst of chunks into a single pump pass. Reset
/// by [`resolve_pending_streams`] before it drains, so a chunk that races the
/// reset still re-arms the wake (no lost delivery).
static STREAM_WAKE_PENDING: AtomicBool = AtomicBool::new(false);

/// Message carried over [`StreamChannel`]: `(epoch, stream_id, message)`.
type StreamEnvelope = (u64, u64, StreamMessage);

/// Process-global channel carrying ordered stream messages keyed by `streamId`,
/// the multi-message analogue of [`InvokeChannel`]. mpsc preserves FIFO, so
/// chunks within one stream arrive in order. `Receiver` is drained on the main
/// thread only (in [`resolve_pending_streams`]).
struct StreamChannel {
    tx: Sender<StreamEnvelope>,
    rx: Mutex<Receiver<StreamEnvelope>>,
}

fn stream_channel() -> &'static StreamChannel {
    static STREAM_MSGS: OnceLock<StreamChannel> = OnceLock::new();
    STREAM_MSGS.get_or_init(|| {
        let (tx, rx) = mpsc::channel();
        StreamChannel {
            tx,
            rx: Mutex::new(rx),
        }
    })
}

/// Signal the pump that stream messages are pending, coalescing bursts. Called
/// by [`StreamSink`] on every send; see [`STREAM_WAKE_PENDING`].
pub(crate) fn signal_stream_wake() {
    if !STREAM_WAKE_PENDING.swap(true, Ordering::AcqRel) {
        signal_wake();
    }
}

// ---------------------------------------------------------------------------
// Render-pump wake channel
// ---------------------------------------------------------------------------

/// Process-global signal channel for the render pump. The pump parks on
/// `recv().await`; producers call [`signal_wake`] on any work arrival. Unbounded
/// so `try_send` never blocks; unit payload — the pump always does a full drain,
/// so coalesced signals are harmless. `Sender` is `Send` (background threads);
/// `Receiver` is `Clone` and pollable inside the `!Send` foreground task.
fn wake_channel() -> &'static (async_channel::Sender<()>, async_channel::Receiver<()>) {
    static WAKE: OnceLock<(async_channel::Sender<()>, async_channel::Receiver<()>)> =
        OnceLock::new();
    WAKE.get_or_init(async_channel::unbounded)
}

/// Signal the render pump that there may be work. Non-blocking; any thread.
pub(crate) fn signal_wake() {
    let _ = wake_channel().0.try_send(());
}

pub(crate) fn wake_receiver() -> async_channel::Receiver<()> {
    wake_channel().1.clone()
}

/// Encode a command result into the JSON envelope the JS `invoke` wrapper
/// expects: `{"ok":true,"value":..}` or `{"ok":false,"error":".."}`.
fn encode_invoke_result(result: CommandResult) -> String {
    match result {
        Ok(value) => serde_json::to_string(&serde_json::json!({ "ok": true, "value": value }))
            .unwrap_or_else(|_| r#"{"ok":true,"value":null}"#.to_string()),
        Err(msg) => serde_json::to_string(&serde_json::json!({ "ok": false, "error": msg }))
            .unwrap_or_else(|_| r#"{"ok":false,"error":"serialization error"}"#.to_string()),
    }
}

/// Store the GPUI background executor before the pump loop starts.
pub(crate) fn set_bg_executor(exec: BackgroundExecutor) {
    BG_EXECUTOR.with(|e| *e.borrow_mut() = Some(exec));
}

/// Queue a synchronous invoke result through the same channel as async results
/// so there is a single resolve path on the JS side.
pub(crate) fn enqueue_invoke_result(call_id: u64, result: CommandResult) {
    let _ = invoke_channel()
        .tx
        .send((call_id, encode_invoke_result(result)));
    signal_wake();
}

/// Run an async command handler on a GPUI background thread and queue its result.
///
/// The handler runs directly inside the spawned future (this GPUI rev has no
/// `spawn_blocking`), so blocking I/O occupies a background-pool thread for its
/// duration — acceptable at current load.
///
/// Falls back to inline execution if the executor isn't registered yet (e.g. a
/// top-level `invoke` during the initial bundle eval, before the run closure).
pub(crate) fn spawn_async_command(
    call_id: u64,
    handler: AsyncCommandHandler,
    args: serde_json::Value,
) {
    let executor = BG_EXECUTOR.with(|e| e.borrow().clone());
    match executor {
        Some(executor) => {
            let tx = invoke_channel().tx.clone();
            executor
                .spawn(async move {
                    let json = encode_invoke_result(handler(args));
                    let _ = tx.send((call_id, json));
                    signal_wake();
                })
                .detach();
        }
        None => enqueue_invoke_result(call_id, handler(args)),
    }
}

/// Drain completed invoke results and resolve their JS Promises.
///
/// Must be called *before* `run_boa_jobs` so `.then` microtasks are flushed in
/// the same tick. Does not call `run_jobs` itself — the following `run_boa_jobs`
/// handles that.
pub(crate) fn resolve_pending_invokes() {
    let drained: Vec<(u64, String)> = {
        let rx = invoke_channel()
            .rx
            .lock()
            .expect("invoke result rx poisoned");
        rx.try_iter().collect()
    };
    if drained.is_empty() {
        return;
    }
    with_boa(|js| {
        for (id, json) in drained {
            let args = [
                JsValue::from(id as f64),
                JsValue::from(JsString::from(json)),
            ];
            call_global(js, "__resolveInvoke", &args);
        }
    });
}

/// Encode a stream message into the JSON envelope the JS `invokeStream` wrapper
/// expects: `{"t":"chunk","value":..}` / `{"t":"end"}` / `{"t":"error","error":".."}`.
fn encode_stream_message(msg: &StreamMessage) -> String {
    let v = match msg {
        StreamMessage::Chunk(value) => serde_json::json!({ "t": "chunk", "value": value }),
        StreamMessage::End => serde_json::json!({ "t": "end" }),
        StreamMessage::Error(e) => serde_json::json!({ "t": "error", "error": e }),
    };
    serde_json::to_string(&v).unwrap_or_else(|_| r#"{"t":"error","error":"encode failed"}"#.into())
}

/// Run a streaming command handler on a GPUI background thread, wiring a
/// [`StreamSink`] that pushes chunks back through [`stream_channel`].
///
/// Registers a cancel flag in `STREAM_CANCELS` (removed when the terminal
/// drains, in `resolve_pending_streams`). The sink's `Drop` guarantees a
/// terminal even if the handler returns or panics early, so the entry is always
/// reclaimed. Falls back to inline execution if the executor isn't registered
/// yet (mirrors [`spawn_async_command`]).
pub(crate) fn spawn_stream_command(
    stream_id: u64,
    handler: StreamCommandHandler,
    args: serde_json::Value,
) {
    let cancel = Arc::new(AtomicBool::new(false));
    STREAM_CANCELS.with(|m| m.borrow_mut().insert(stream_id, cancel.clone()));
    let epoch = STREAM_EPOCH.load(Ordering::Relaxed);
    let sink = StreamSink::new(stream_id, stream_channel().tx.clone(), cancel, epoch);
    let executor = BG_EXECUTOR.with(|e| e.borrow().clone());
    match executor {
        Some(executor) => {
            executor
                .spawn(async move {
                    handler(args, sink);
                })
                .detach();
        }
        None => handler(args, sink),
    }
}

/// Drain queued stream messages and deliver them to JS via `__streamPush`.
///
/// Like [`resolve_pending_invokes`], must run *before* `run_boa_jobs` so the
/// microtasks each chunk schedules flush in the same tick. Terminal messages
/// (`End`/`Error`) reclaim the stream's cancel-flag entry.
pub(crate) fn resolve_pending_streams() {
    // Clear the coalescing flag *before* draining so a chunk that races this
    // drain re-arms the wake and is delivered on the next pass (never lost).
    STREAM_WAKE_PENDING.store(false, Ordering::Release);
    let drained: Vec<StreamEnvelope> = {
        let rx = stream_channel()
            .rx
            .lock()
            .expect("stream result rx poisoned");
        rx.try_iter().collect()
    };
    if drained.is_empty() {
        return;
    }
    // Drop messages from a superseded epoch (an old-bundle handler still winding
    // down after a hot reload), so they can't hit a reused id in the new context.
    let current_epoch = STREAM_EPOCH.load(Ordering::Relaxed);
    with_boa(|js| {
        for (epoch, id, msg) in &drained {
            if *epoch != current_epoch {
                continue;
            }
            let json = encode_stream_message(msg);
            let args = [
                JsValue::from(*id as f64),
                JsValue::from(JsString::from(json)),
            ];
            call_global(js, "__streamPush", &args);
        }
    });
    for (epoch, id, msg) in &drained {
        if *epoch == current_epoch && matches!(msg, StreamMessage::End | StreamMessage::Error(_)) {
            STREAM_CANCELS.with(|m| {
                m.borrow_mut().remove(id);
            });
        }
    }
}

/// Flag a running stream handler for cooperative cancellation (called from
/// `__streamCancel`). The entry is *not* removed here — the handler may still
/// emit a terminal after observing the flag, and if it never does, the sink's
/// `Drop` sends `End` and `resolve_pending_streams` reclaims the entry.
pub(crate) fn cancel_stream(stream_id: u64) {
    STREAM_CANCELS.with(|m| {
        if let Some(flag) = m.borrow().get(&stream_id) {
            flag.store(true, Ordering::Relaxed);
        }
    });
}

/// Terminate a stream with an error without ever running a handler — used by the
/// bridge for unknown keys and wrong-flavour (`invokeStream` on a sync/async
/// command). Goes through the normal drain path so the JS controller errors on a
/// later tick, consistent with `invoke`'s deferred error delivery.
pub(crate) fn error_stream(stream_id: u64, msg: String) {
    let epoch = STREAM_EPOCH.load(Ordering::Relaxed);
    let _ = stream_channel()
        .tx
        .send((epoch, stream_id, StreamMessage::Error(msg)));
    STREAM_CANCELS.with(|m| {
        m.borrow_mut().remove(&stream_id);
    });
    signal_stream_wake();
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
            // When a node is destroyed, release its ScrollHandle, TextInput
            // entity, and FocusHandle so the Rc-chains are fully dropped and we
            // don't accumulate stale handles.
            if let UICommand::DetachDeleted { id } = &cmd {
                SCROLL_HANDLES.with(|handles| {
                    handles.borrow_mut().remove(id);
                });
                TEXT_INPUTS.with(|inputs| {
                    inputs.borrow_mut().remove(id);
                });
                FOCUS_HANDLES.with(|handles| {
                    handles.borrow_mut().remove(id);
                });
                AUTO_FOCUSED.with(|set| {
                    set.borrow_mut().remove(id);
                });
                // Drop a stale Tab resume-anchor pointing at the removed element.
                if FOCUS_ANCHOR.with(|c| c.get()) == Some(*id) {
                    FOCUS_ANCHOR.with(|c| c.set(None));
                }
                crate::render::drop_focus_subscriptions(*id);
                crate::anim::remove_node(*id);
            }
            // Start/replace/cancel style transitions before `apply_command`
            // swaps the props in (it needs the old style to diff against).
            if let UICommand::UpdateProps { id, props } = &cmd {
                if let Some(old) = tree.nodes.get(id) {
                    if old.props != *props {
                        crate::anim::on_props_update(*id, &old.props.style, props, now_ms);
                    }
                }
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

/// Dev-mode: background executor for debounce/retry timers. `None` until the
/// app run closure stores it.
#[cfg(debug_assertions)]
pub(crate) fn bg_executor() -> Option<BackgroundExecutor> {
    BG_EXECUTOR.with(|e| e.borrow().clone())
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
    crate::render::clear_focus_subscriptions();
    ARMED_DEADLINE.with(|a| a.set(None));
    // Ask any still-running stream handlers (from the old bundle) to wind down:
    // their sinks poll `is_closed()`. Then drop the flags.
    STREAM_CANCELS.with(|m| {
        let mut map = m.borrow_mut();
        for flag in map.values() {
            flag.store(true, Ordering::Relaxed);
        }
        map.clear();
    });
    // Bump the epoch so any messages those old handlers still emit (including the
    // `End` from their sink's `Drop`) are filtered out in `resolve_pending_streams`
    // instead of landing on a reused stream id in the fresh JS context.
    STREAM_EPOCH.fetch_add(1, Ordering::Relaxed);
    crate::render::clear_node_views();
    crate::anim::clear();
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
    if in_flight {
        if let Some(armed) = ARMED_DEADLINE.with(|a| a.get()) {
            if due >= armed {
                // A live timer covers this deadline; its wake will re-arm if needed.
                return;
            }
        }
    }

    let executor = BG_EXECUTOR.with(|e| e.borrow().clone());
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
    if let Some(func) = callable.as_callable() {
        if let Err(err) = func.call(&JsValue::undefined(), args, js) {
            log_js_error(name, &err);
        }
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

/// Retrieve the `FocusHandle` for `id` without creating it. Used in `render_node`
/// where `&mut App` is unavailable; `None` if the pre-pass did not create it yet.
pub(crate) fn get_focus_handle(id: ElementId) -> Option<FocusHandle> {
    FOCUS_HANDLES.with(|handles| handles.borrow().get(&id).cloned())
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
    if mark_autofocus(id) {
        if let Some(handle) = get_focus_handle(id) {
            window.focus(&handle, cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- encode_invoke_result ----

    fn parse_envelope(json: &str) -> serde_json::Value {
        serde_json::from_str(json).expect("valid JSON envelope")
    }

    #[test]
    fn ok_result_produces_ok_true_envelope() {
        let result: CommandResult = Ok(serde_json::json!(42));
        let encoded = encode_invoke_result(result);
        let v = parse_envelope(&encoded);
        assert_eq!(v["ok"], serde_json::json!(true));
        assert_eq!(v["value"], serde_json::json!(42));
    }

    #[test]
    fn ok_result_with_string_value() {
        let result: CommandResult = Ok(serde_json::json!("hello"));
        let encoded = encode_invoke_result(result);
        let v = parse_envelope(&encoded);
        assert_eq!(v["ok"], serde_json::json!(true));
        assert_eq!(v["value"], serde_json::json!("hello"));
    }

    #[test]
    fn ok_result_with_object_value() {
        let result: CommandResult = Ok(serde_json::json!({ "a": 1 }));
        let encoded = encode_invoke_result(result);
        let v = parse_envelope(&encoded);
        assert_eq!(v["ok"], serde_json::json!(true));
        assert_eq!(v["value"]["a"], serde_json::json!(1));
    }

    #[test]
    fn err_result_produces_ok_false_envelope() {
        let result: CommandResult = Err("something went wrong".to_string());
        let encoded = encode_invoke_result(result);
        let v = parse_envelope(&encoded);
        assert_eq!(v["ok"], serde_json::json!(false));
        assert_eq!(v["error"], serde_json::json!("something went wrong"));
    }

    #[test]
    fn err_result_has_no_value_key() {
        let result: CommandResult = Err("oops".to_string());
        let encoded = encode_invoke_result(result);
        let v = parse_envelope(&encoded);
        assert!(v.get("value").is_none());
    }

    #[test]
    fn ok_result_has_no_error_key() {
        let result: CommandResult = Ok(serde_json::json!(null));
        let encoded = encode_invoke_result(result);
        let v = parse_envelope(&encoded);
        assert!(v.get("error").is_none());
    }

    // ---- encode_stream_message ----

    #[test]
    fn chunk_message_envelope() {
        let v = parse_envelope(&encode_stream_message(&StreamMessage::Chunk(
            serde_json::json!({ "line": "hi" }),
        )));
        assert_eq!(v["t"], serde_json::json!("chunk"));
        assert_eq!(v["value"]["line"], serde_json::json!("hi"));
    }

    #[test]
    fn end_message_envelope() {
        let v = parse_envelope(&encode_stream_message(&StreamMessage::End));
        assert_eq!(v["t"], serde_json::json!("end"));
        assert!(v.get("value").is_none());
        assert!(v.get("error").is_none());
    }

    #[test]
    fn error_message_envelope() {
        let v = parse_envelope(&encode_stream_message(&StreamMessage::Error("boom".into())));
        assert_eq!(v["t"], serde_json::json!("error"));
        assert_eq!(v["error"], serde_json::json!("boom"));
    }
}
