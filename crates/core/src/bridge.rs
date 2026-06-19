use boa_engine::{
    Context as JsContext, JsNativeError, JsObject, JsValue, NativeFunction, js_string,
    object::builtins::JsArray,
};

use crate::{
    coerce::js_to_json,
    component,
    model::{ElementKind, Events, Props, UICommand},
    plugin, state,
    state::{next_id, push_cmd},
    style::parse_props,
};

// ---------------------------------------------------------------------------
// Bridge registration
// ---------------------------------------------------------------------------

/// Parse `Props` from bridge args: position 1 = props object, position 2 =
/// event-type array. Used by both `createInstance` and `updateProps`.
/// When `capture_raw` is set (native components), the props are also stored
/// as `Props::raw` JSON for the component's render function.
fn props_from_args(args: &[JsValue], ctx: &mut JsContext, capture_raw: bool) -> Props {
    let mut props = args
        .get(1)
        .and_then(|v| v.as_object())
        .as_ref()
        .map(|obj| parse_props(obj, ctx))
        .unwrap_or_default();
    if capture_raw && let Some(v) = args.get(1) {
        props.raw = Some(js_to_json(v, ctx));
    }
    // Third argument: event-type string array (e.g. ["click", "mousedown"]);
    // absent when no handlers are registered.
    let event_types: Vec<String> = args
        .get(2)
        .cloned()
        .unwrap_or_default()
        .try_js_into(ctx)
        .unwrap_or_default();
    props.events = Events::from_types(&event_types);
    props
}

/// Install every bridge global into a fresh Boa context: `__bridge` (reconciler
/// ops), `__invoke` / `__invokeStream` / `__streamCancel` (native commands),
/// `requestAnimationFrame` / `cancelAnimationFrame`, and `performance`.
///
/// Called once per context — at startup and on each dev-mode hot reload.
pub(crate) fn register_all(ctx: &mut JsContext) -> boa_engine::JsResult<()> {
    register_bridge(ctx)?;
    register_invoke(ctx)?;
    register_stream(ctx)?;
    register_raf(ctx)?;
    register_performance(ctx)?;
    Ok(())
}

fn register_bridge(ctx: &mut JsContext) -> boa_engine::JsResult<()> {
    let create_instance = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let type_str = string_arg(args, 0, ctx)?;
        let kind = ElementKind::from_type_name(&type_str, component::is_registered);
        let capture_raw = kind.is_native();
        let props = props_from_args(args, ctx, capture_raw);
        let id = next_id();
        push_cmd(UICommand::CreateInstance { id, kind, props });
        Ok(JsValue::from(id as f64))
    });

    let create_text = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let text = string_arg(args, 0, ctx)?;
        let id = next_id();
        push_cmd(UICommand::CreateText { id, text });
        Ok(JsValue::from(id as f64))
    });

    let append_child = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let parent = u64_arg(args, 0, ctx)?;
        let child = u64_arg(args, 1, ctx)?;
        push_cmd(UICommand::AppendChild { parent, child });
        Ok(JsValue::undefined())
    });

    let append_to_container = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let child = u64_arg(args, 0, ctx)?;
        push_cmd(UICommand::AppendToContainer { child });
        Ok(JsValue::undefined())
    });

    let insert_before = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let parent = u64_arg(args, 0, ctx)?;
        let child = u64_arg(args, 1, ctx)?;
        let before = u64_arg(args, 2, ctx)?;
        push_cmd(UICommand::InsertBefore {
            parent,
            child,
            before,
        });
        Ok(JsValue::undefined())
    });

    let insert_in_container = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let child = u64_arg(args, 0, ctx)?;
        let before = u64_arg(args, 1, ctx)?;
        push_cmd(UICommand::InsertInContainer { child, before });
        Ok(JsValue::undefined())
    });

    let remove_child = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let parent = u64_arg(args, 0, ctx)?;
        let child = u64_arg(args, 1, ctx)?;
        push_cmd(UICommand::RemoveChild { parent, child });
        Ok(JsValue::undefined())
    });

    let remove_from_container = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let child = u64_arg(args, 0, ctx)?;
        push_cmd(UICommand::RemoveFromContainer { child });
        Ok(JsValue::undefined())
    });

    let update_props = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let id = u64_arg(args, 0, ctx)?;
        // 4th arg: element type string from host-config's `commitUpdate`.
        // Re-captures raw props for native components so prop changes reflow;
        // absent/empty for older bundles → non-native path.
        let type_str = string_arg_or(args, 3, ctx, "");
        let capture_raw =
            ElementKind::from_type_name(&type_str, component::is_registered).is_native();
        let props = props_from_args(args, ctx, capture_raw);
        push_cmd(UICommand::UpdateProps { id, props });
        Ok(JsValue::undefined())
    });

    let update_text = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let id = u64_arg(args, 0, ctx)?;
        let text = string_arg(args, 1, ctx)?;
        push_cmd(UICommand::UpdateText { id, text });
        Ok(JsValue::undefined())
    });

    let clear_container = NativeFunction::from_copy_closure(|_this, _args, _ctx| {
        push_cmd(UICommand::ClearContainer);
        Ok(JsValue::undefined())
    });

    let detach_deleted = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let id = u64_arg(args, 0, ctx)?;
        push_cmd(UICommand::DetachDeleted { id });
        Ok(JsValue::undefined())
    });

    // Synchronous read of the focused element id (any kind), as of the last paint;
    // `null` for the root fallback / nothing. Backs `getActiveElement()` for
    // save/restore-focus patterns. Read-only — no command queued.
    let get_active_element = NativeFunction::from_copy_closure(|_this, _args, _ctx| {
        Ok(match state::active_element() {
            Some(id) => JsValue::from(id as f64),
            None => JsValue::null(),
        })
    });

    // Tab-stop focusable ids in `rootId`'s subtree, in Tab order (last-paint
    // tree). Backs `getFocusableElements(rootId)`. Read-only — no command queued.
    let get_focusable_elements = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let root = u64_arg(args, 0, ctx)?;
        let ids = state::focusable_descendants(root);
        let array = JsArray::from_iter(ids.into_iter().map(|id| JsValue::from(id as f64)), ctx);
        Ok(array.into())
    });

    // Push/pop a Tab scope (confine Tab to a subtree). Synchronous thread-local
    // mutation — no command queued; read by FocusNext/FocusPrev.
    let push_tab_scope = NativeFunction::from_copy_closure(|_this, args, ctx| {
        state::push_tab_scope(u64_arg(args, 0, ctx)?);
        Ok(JsValue::undefined())
    });
    let pop_tab_scope = NativeFunction::from_copy_closure(|_this, args, ctx| {
        state::pop_tab_scope(u64_arg(args, 0, ctx)?);
        Ok(JsValue::undefined())
    });

    let bridge = JsObject::with_object_proto(ctx.intrinsics());
    set_bridge_fn(ctx, &bridge, "createInstance", create_instance)?;
    set_bridge_fn(ctx, &bridge, "createText", create_text)?;
    set_bridge_fn(ctx, &bridge, "appendChild", append_child)?;
    set_bridge_fn(ctx, &bridge, "appendToContainer", append_to_container)?;
    set_bridge_fn(ctx, &bridge, "insertBefore", insert_before)?;
    set_bridge_fn(ctx, &bridge, "insertInContainer", insert_in_container)?;
    set_bridge_fn(ctx, &bridge, "removeChild", remove_child)?;
    set_bridge_fn(ctx, &bridge, "removeFromContainer", remove_from_container)?;
    set_bridge_fn(ctx, &bridge, "updateProps", update_props)?;
    set_bridge_fn(ctx, &bridge, "updateText", update_text)?;
    set_bridge_fn(ctx, &bridge, "clearContainer", clear_container)?;
    set_bridge_fn(ctx, &bridge, "detachDeleted", detach_deleted)?;
    set_bridge_fn(ctx, &bridge, "getActiveElement", get_active_element)?;
    set_bridge_fn(ctx, &bridge, "getFocusableElements", get_focusable_elements)?;
    set_bridge_fn(ctx, &bridge, "pushTabScope", push_tab_scope)?;
    set_bridge_fn(ctx, &bridge, "popTabScope", pop_tab_scope)?;

    ctx.global_object()
        .set(js_string!("__bridge"), bridge, false, ctx)?;
    Ok(())
}

/// Coerce positional arg `index` to a `u64` (JS numbers are f64). Shared by the
/// reconciler element ids and the invoke/stream call ids.
///
/// Rejects non-finite (`NaN` / `±Inf`) and negative values instead of letting
/// the `as u64` cast saturate them to `0` / `u64::MAX` — a malformed id from a
/// buggy or hostile bundle becomes a JS `TypeError` here rather than a silent
/// wrong-node mutation. Well-formed bundles only ever pass the monotonic
/// positive counters this guard accepts.
fn u64_arg(args: &[JsValue], index: usize, ctx: &mut JsContext) -> boa_engine::JsResult<u64> {
    let n: f64 = args
        .get(index)
        .cloned()
        .unwrap_or_default()
        .try_js_into(ctx)?;
    if !n.is_finite() || n < 0.0 {
        return Err(JsNativeError::typ()
            .with_message(format!("bridge: argument {index} is not a valid id: {n}"))
            .into());
    }
    Ok(n as u64)
}

/// Required string positional arg — propagates a JS TypeError when it can't coerce.
fn string_arg(args: &[JsValue], index: usize, ctx: &mut JsContext) -> boa_engine::JsResult<String> {
    args.get(index)
        .cloned()
        .unwrap_or_default()
        .try_js_into(ctx)
}

/// Optional string positional arg — falls back to `default` on absence or
/// coercion failure (no error).
fn string_arg_or(args: &[JsValue], index: usize, ctx: &mut JsContext, default: &str) -> String {
    args.get(index)
        .cloned()
        .unwrap_or_default()
        .try_js_into(ctx)
        .unwrap_or_else(|_| default.to_string())
}

/// Parse the `(cmdKey, args)` pair shared by `__invoke` and `__invokeStream`:
/// positional arg 1 = command-key string (required), arg 2 = a JSON string
/// (absent/non-string → `"{}"`, then unparseable → `Value::Null`).
fn parse_invoke_args(
    args: &[JsValue],
    ctx: &mut JsContext,
) -> boa_engine::JsResult<(String, serde_json::Value)> {
    let key = string_arg(args, 1, ctx)?;
    let args_json = string_arg_or(args, 2, ctx, "{}");
    let args_value = serde_json::from_str(&args_json).unwrap_or(serde_json::Value::Null);
    Ok((key, args_value))
}

fn set_bridge_fn(
    ctx: &mut JsContext,
    bridge: &JsObject,
    name: &str,
    function: NativeFunction,
) -> boa_engine::JsResult<()> {
    bridge
        .set(
            js_string!(name),
            function.to_js_function(ctx.realm()),
            false,
            ctx,
        )
        .map(|_| ())
}

// ---------------------------------------------------------------------------
// __invoke — JS→Rust native command dispatch
// ---------------------------------------------------------------------------

/// Register `globalThis.__invoke` in the Boa context.
///
/// JS contract: `__invoke(callId: number, cmdKey: string, argsJson: string) -> void`
///   - `callId`   — echoed back via `__resolveInvoke(callId, json)` when ready.
///   - `cmdKey`   — `"{plugin}|{command}"`, e.g. `"fs|readTextFile"`.
///   - `argsJson` — `JSON.stringify(args)`.
///
/// Returns `undefined`; result delivery is always deferred — the pump loop calls
/// `__resolveInvoke` to settle the Promise whether the command was sync or async.
fn register_invoke(ctx: &mut JsContext) -> boa_engine::JsResult<()> {
    let invoke_fn = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let call_id = u64_arg(args, 0, ctx)?;
        let (key, args_value) = parse_invoke_args(args, ctx)?;

        // Flavour policy (incl. stream-via-invoke rejection) lives in `plugin`;
        // here we only route the outcome to `state`.
        match plugin::dispatch_invoke(&key, args_value) {
            plugin::InvokeOutcome::Ready(result) => state::enqueue_invoke_result(call_id, result),
            plugin::InvokeOutcome::Spawn(handler, args) => {
                state::spawn_async_command(call_id, handler, args)
            }
        }

        Ok(JsValue::undefined())
    });

    ctx.global_object().set(
        js_string!("__invoke"),
        invoke_fn.to_js_function(ctx.realm()),
        false,
        ctx,
    )?;
    Ok(())
}

/// Register `globalThis.__invokeStream` and `globalThis.__streamCancel`.
///
/// JS contract:
///   `__invokeStream(streamId: number, cmdKey: string, argsJson: string) -> void`
///     — starts a streaming command. Chunks are delivered later via
///       `__streamPush(streamId, json)` from the pump loop. Targeting a
///       non-stream command errors the stream (it never hangs).
///   `__streamCancel(streamId: number) -> void`
///     — cooperatively cancels a running stream (the handler polls `is_closed()`).
///
/// Both closures are stateless, satisfying the `from_copy_closure` `Copy` bound;
/// all state lives in `state`/`plugin` registries.
fn register_stream(ctx: &mut JsContext) -> boa_engine::JsResult<()> {
    let invoke_stream = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let stream_id = u64_arg(args, 0, ctx)?;
        let (key, args_value) = parse_invoke_args(args, ctx)?;

        // Flavour policy (non-stream commands → error) lives in `plugin`; here we
        // only route: spawn the stream, or terminate it with the dispatch error.
        match plugin::dispatch_stream(&key, args_value) {
            plugin::StreamOutcome::Spawn(handler, args) => {
                state::spawn_stream_command(stream_id, handler, args)
            }
            plugin::StreamOutcome::Error(msg) => state::error_stream(stream_id, msg),
        }

        Ok(JsValue::undefined())
    });

    let stream_cancel = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let stream_id = u64_arg(args, 0, ctx)?;
        state::cancel_stream(stream_id);
        Ok(JsValue::undefined())
    });

    ctx.global_object().set(
        js_string!("__invokeStream"),
        invoke_stream.to_js_function(ctx.realm()),
        false,
        ctx,
    )?;
    ctx.global_object().set(
        js_string!("__streamCancel"),
        stream_cancel.to_js_function(ctx.realm()),
        false,
        ctx,
    )?;
    Ok(())
}

/// Register the `requestAnimationFrame` / `cancelAnimationFrame` globals.
///
/// Callbacks are queued in `state` and run by the render pump after each GPUI
/// frame (see `state::run_raf_callbacks`). The callback receives a fractional-ms
/// timestamp on the same monotonic clock as JS timers.
fn register_raf(ctx: &mut JsContext) -> boa_engine::JsResult<()> {
    let raf = NativeFunction::from_copy_closure(|_this, args, _ctx| {
        let Some(cb) = args.first().and_then(JsValue::as_callable) else {
            return Err(JsNativeError::typ()
                .with_message("requestAnimationFrame: callback is not a function")
                .into());
        };
        Ok(JsValue::from(state::raf_request(cb.clone()) as f64))
    });
    let caf = NativeFunction::from_copy_closure(|_this, args, _ctx| {
        // Non-numeric / unknown ids are silently ignored (browser semantics).
        let id = args.first().and_then(JsValue::as_number).unwrap_or(0.0);
        if id >= 1.0 {
            state::raf_cancel(id as u64);
        }
        Ok(JsValue::undefined())
    });
    ctx.register_global_callable(js_string!("requestAnimationFrame"), 1, raf)?;
    ctx.register_global_callable(js_string!("cancelAnimationFrame"), 1, caf)?;
    Ok(())
}

/// Register the `performance` global (`performance.now()` + `performance.timeOrigin`).
///
/// `now()` returns fractional ms on the same monotonic clock as rAF and JS timers,
/// relative to `timeOrigin` (epoch-ms at Boa context init). The sum
/// `timeOrigin + now()` reconstructs the current wall-clock epoch milliseconds.
fn register_performance(ctx: &mut JsContext) -> boa_engine::JsResult<()> {
    let origin_ms = ctx.clock().now().nanos_since_epoch() as f64 / 1_000_000.0;

    let now_fn = NativeFunction::from_copy_closure(move |_this, _args, ctx| {
        let cur_ms = ctx.clock().now().nanos_since_epoch() as f64 / 1_000_000.0;
        Ok(JsValue::from(cur_ms - origin_ms))
    });

    let performance = JsObject::with_object_proto(ctx.intrinsics());
    performance.set(
        js_string!("now"),
        now_fn.to_js_function(ctx.realm()),
        false,
        ctx,
    )?;
    performance.set(
        js_string!("timeOrigin"),
        JsValue::from(origin_ms),
        false,
        ctx,
    )?;
    ctx.global_object()
        .set(js_string!("performance"), performance, false, ctx)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack::eval_on_parser_stack;

    fn fresh_ctx() -> JsContext {
        JsContext::builder().build().expect("ctx")
    }

    #[test]
    fn u64_arg_accepts_finite_non_negative() {
        let mut ctx = fresh_ctx();
        assert_eq!(
            u64_arg(&[JsValue::from(42.0_f64)], 0, &mut ctx).unwrap(),
            42
        );
        // Fractional values truncate toward zero, matching the f64 -> u64 cast.
        assert_eq!(u64_arg(&[JsValue::from(7.9_f64)], 0, &mut ctx).unwrap(), 7);
    }

    #[test]
    fn u64_arg_rejects_negative() {
        let mut ctx = fresh_ctx();
        assert!(u64_arg(&[JsValue::from(-1.0_f64)], 0, &mut ctx).is_err());
    }

    #[test]
    fn u64_arg_rejects_non_finite() {
        let mut ctx = fresh_ctx();
        let inf = eval_on_parser_stack(&mut ctx, b"(1/0)").unwrap();
        let nan = eval_on_parser_stack(&mut ctx, b"(0/0)").unwrap();
        assert!(u64_arg(&[inf], 0, &mut ctx).is_err());
        assert!(u64_arg(&[nan], 0, &mut ctx).is_err());
    }
}
