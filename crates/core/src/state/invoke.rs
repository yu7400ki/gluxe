//! Cross-thread background-work plumbing: the async / stream `invoke` channels,
//! the render-pump wake channel, and the GPUI background executor.
//!
//! This is the only part of `state` that touches other threads. Command handlers
//! run on GPUI background threads (`spawn_async_command` / `spawn_stream_command`);
//! their results flow back over process-global mpsc channels and are drained on
//! the main thread (`resolve_pending_invokes` / `resolve_pending_streams`), which
//! deliver them to JS through the parent module's `with_boa` choke point. The
//! `Send` boundary lives here; the rest of `state` is single-threaded.

use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
    },
};

use boa_engine::{JsString, JsValue};
use gpui::BackgroundExecutor;

use crate::plugin::{
    AsyncCommandHandler, CommandResult, StreamCommandHandler, StreamMessage, StreamSink,
};

use super::{call_global, with_boa};

thread_local! {
    // GPUI background executor — clone stored here; only the resulting Send
    // future crosses threads, not the executor itself.
    static BG_EXECUTOR: RefCell<Option<BackgroundExecutor>> = const { RefCell::new(None) };
    // Per-live-stream cancel flags. The map lives on the main thread (registered
    // in `spawn_stream_command`, removed on terminal drain / `error_stream`); the
    // flag itself is an `Arc<AtomicBool>` shared with the sink on the background
    // thread so `__streamCancel` can ask a running handler to stop.
    static STREAM_CANCELS: RefCell<HashMap<u64, Arc<AtomicBool>>> = RefCell::new(HashMap::new());
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

/// Bumped on every dev-mode hot reload ([`reset_streams_for_reload`]). Each stream
/// is stamped with the epoch live when it was spawned; messages from a prior epoch
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

/// Clone of the GPUI background executor, or `None` until the app run closure
/// installs it via [`set_bg_executor`]. Used to spawn async / stream command
/// handlers here and the timer-arm background wake (`super::arm_next_timer`).
pub(crate) fn bg_executor() -> Option<BackgroundExecutor> {
    BG_EXECUTOR.with(|e| e.borrow().clone())
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

/// Dev-mode hot reload: ask any still-running stream handlers from the old bundle
/// to wind down (their sinks poll `is_closed()`), drop the cancel flags, then bump
/// the stream epoch so any late messages those handlers still emit (including the
/// `End` from a sink's `Drop`) are filtered out in [`resolve_pending_streams`]
/// instead of landing on a reused stream id in the fresh JS context.
#[cfg(debug_assertions)]
pub(super) fn reset_streams_for_reload() {
    STREAM_CANCELS.with(|m| {
        let mut map = m.borrow_mut();
        for flag in map.values() {
            flag.store(true, Ordering::Relaxed);
        }
        map.clear();
    });
    STREAM_EPOCH.fetch_add(1, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_envelope(json: &str) -> serde_json::Value {
        serde_json::from_str(json).expect("valid JSON envelope")
    }

    // ---- encode_invoke_result ----

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
