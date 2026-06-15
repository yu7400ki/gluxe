use boa_engine::{
    Context as JsContext, JsNativeError, JsObject, JsValue, NativeFunction, js_string,
    property::PropertyKey,
};

use crate::{
    component,
    model::{ElementKind, Events, Props, UICommand},
    plugin, state,
    state::{next_id, push_cmd},
    style::parse_props,
};

// ---------------------------------------------------------------------------
// Bridge registration
// ---------------------------------------------------------------------------

fn parse_kind(type_str: &str) -> ElementKind {
    match type_str {
        "View" => ElementKind::View,
        "Text" => ElementKind::Text,
        "Image" => ElementKind::Image,
        "TextInput" => ElementKind::TextInput,
        // Any other type string is a host-registered native component when its
        // name is in the registry; otherwise fall back to a plain `View`.
        other if component::is_registered(other) => ElementKind::Native(other.to_string()),
        _ => ElementKind::View,
    }
}

/// Recursively convert a JS value into a `serde_json::Value`.
///
/// Used to pass raw props to native component render functions. Skips callable
/// values defensively (JS already strips handlers in `extractHandlers`).
/// Implemented by hand rather than boa's `to_json` to avoid a feature flag and
/// to control how non-JSON values (NaN, ±Inf, symbols) map.
fn js_to_json(value: &JsValue, ctx: &mut JsContext) -> serde_json::Value {
    use serde_json::Value;

    if value.is_null_or_undefined() {
        return Value::Null;
    }
    if let Some(b) = value.as_boolean() {
        return Value::Bool(b);
    }
    if let Some(n) = value.as_number() {
        // Store integral values as JSON integers: JS has only f64, so `5`
        // arrives as `5.0`; a float-backed serde number would make `as_u64()` return `None`.
        if n.is_finite() && n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
            return Value::Number((n as i64).into());
        }
        // NaN / ±Inf are not representable in JSON → Null.
        return serde_json::Number::from_f64(n)
            .map(Value::Number)
            .unwrap_or(Value::Null);
    }
    if let Some(s) = value.as_string() {
        return Value::String(s.to_std_string().unwrap_or_default());
    }
    if let Some(obj) = value.as_object() {
        if obj.is_array() {
            let len = obj
                .get(js_string!("length"), ctx)
                .ok()
                .and_then(|v| v.as_number())
                .unwrap_or(0.0) as u32;
            let mut arr = Vec::with_capacity(len as usize);
            for i in 0..len {
                let item = obj.get(js_string!(format!("{i}")), ctx).unwrap_or_default();
                arr.push(js_to_json(&item, ctx));
            }
            return Value::Array(arr);
        }
        // Own string + integer-index keys only (symbols skipped).
        // Integer-index keys arrive as `PropertyKey::Index` and are stringified
        // so they aren't silently dropped.
        let mut map = serde_json::Map::new();
        if let Ok(keys) = obj.own_property_keys(ctx) {
            for key in keys {
                let key_str = match &key {
                    PropertyKey::String(k) => k.to_std_string().unwrap_or_default(),
                    PropertyKey::Index(i) => i.get().to_string(),
                    PropertyKey::Symbol(_) => continue,
                };
                let v = obj.get(key, ctx).unwrap_or_default();
                if v.is_callable() {
                    continue;
                }
                map.insert(key_str, js_to_json(&v, ctx));
            }
        }
        return Value::Object(map);
    }
    Value::Null
}

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
    if capture_raw {
        if let Some(v) = args.get(1) {
            props.raw = Some(js_to_json(v, ctx));
        }
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

pub(crate) fn register_bridge(ctx: &mut JsContext) -> boa_engine::JsResult<()> {
    let create_instance = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let type_str: String = args.first().cloned().unwrap_or_default().try_js_into(ctx)?;
        let kind = parse_kind(&type_str);
        let capture_raw = matches!(kind, ElementKind::Native(_));
        let props = props_from_args(&args, ctx, capture_raw);
        let id = next_id();
        push_cmd(UICommand::CreateInstance { id, kind, props });
        Ok(JsValue::from(id as f64))
    });

    let create_text = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let text: String = args.first().cloned().unwrap_or_default().try_js_into(ctx)?;
        let id = next_id();
        push_cmd(UICommand::CreateText { id, text });
        Ok(JsValue::from(id as f64))
    });

    let append_child = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let parent = element_id_arg(args, 0, ctx)?;
        let child = element_id_arg(args, 1, ctx)?;
        push_cmd(UICommand::AppendChild { parent, child });
        Ok(JsValue::undefined())
    });

    let append_to_container = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let child = element_id_arg(args, 0, ctx)?;
        push_cmd(UICommand::AppendToContainer { child });
        Ok(JsValue::undefined())
    });

    let insert_before = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let parent = element_id_arg(args, 0, ctx)?;
        let child = element_id_arg(args, 1, ctx)?;
        let before = element_id_arg(args, 2, ctx)?;
        push_cmd(UICommand::InsertBefore {
            parent,
            child,
            before,
        });
        Ok(JsValue::undefined())
    });

    let insert_in_container = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let child = element_id_arg(args, 0, ctx)?;
        let before = element_id_arg(args, 1, ctx)?;
        push_cmd(UICommand::InsertInContainer { child, before });
        Ok(JsValue::undefined())
    });

    let remove_child = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let parent = element_id_arg(args, 0, ctx)?;
        let child = element_id_arg(args, 1, ctx)?;
        push_cmd(UICommand::RemoveChild { parent, child });
        Ok(JsValue::undefined())
    });

    let remove_from_container = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let child = element_id_arg(args, 0, ctx)?;
        push_cmd(UICommand::RemoveFromContainer { child });
        Ok(JsValue::undefined())
    });

    let update_props = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let id = element_id_arg(args, 0, ctx)?;
        // 4th arg: element type string from host-config's `commitUpdate`.
        // Re-captures raw props for native components so prop changes reflow;
        // absent/empty for older bundles → non-native path.
        let type_str: String = args
            .get(3)
            .cloned()
            .unwrap_or_default()
            .try_js_into(ctx)
            .unwrap_or_default();
        let capture_raw = matches!(parse_kind(&type_str), ElementKind::Native(_));
        let props = props_from_args(&args, ctx, capture_raw);
        push_cmd(UICommand::UpdateProps { id, props });
        Ok(JsValue::undefined())
    });

    let update_text = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let id = element_id_arg(args, 0, ctx)?;
        let text: String = args.get(1).cloned().unwrap_or_default().try_js_into(ctx)?;
        push_cmd(UICommand::UpdateText { id, text });
        Ok(JsValue::undefined())
    });

    let clear_container = NativeFunction::from_copy_closure(|_this, _args, _ctx| {
        push_cmd(UICommand::ClearContainer);
        Ok(JsValue::undefined())
    });

    let detach_deleted = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let id = element_id_arg(args, 0, ctx)?;
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

    ctx.global_object()
        .set(js_string!("__bridge"), bridge, false, ctx)?;
    Ok(())
}

fn element_id_arg(
    args: &[JsValue],
    index: usize,
    ctx: &mut JsContext,
) -> boa_engine::JsResult<u64> {
    Ok(args
        .get(index)
        .cloned()
        .unwrap_or_default()
        .try_js_into::<f64>(ctx)? as u64)
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
pub(crate) fn register_invoke(ctx: &mut JsContext) -> boa_engine::JsResult<()> {
    let invoke_fn = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let call_id: f64 = args.first().cloned().unwrap_or_default().try_js_into(ctx)?;
        let call_id = call_id as u64;
        let key: String = args.get(1).cloned().unwrap_or_default().try_js_into(ctx)?;
        let args_json: String = args
            .get(2)
            .cloned()
            .unwrap_or_default()
            .try_js_into(ctx)
            .unwrap_or_else(|_| "{}".to_string());

        let args_value: serde_json::Value =
            serde_json::from_str(&args_json).unwrap_or(serde_json::Value::Null);

        match plugin::dispatch_command(&key, args_value) {
            plugin::Dispatched::Ready(result) => state::enqueue_invoke_result(call_id, result),
            plugin::Dispatched::Spawn(handler, args) => {
                state::spawn_async_command(call_id, handler, args)
            }
            // A streaming command can't deliver a single result — reject the
            // `invoke` Promise; the caller must use `invokeStream` instead.
            plugin::Dispatched::SpawnStream(_, _) => state::enqueue_invoke_result(
                call_id,
                Err(format!("'{key}' is a stream command — use invokeStream")),
            ),
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
pub(crate) fn register_stream(ctx: &mut JsContext) -> boa_engine::JsResult<()> {
    let invoke_stream = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let stream_id: f64 = args.first().cloned().unwrap_or_default().try_js_into(ctx)?;
        let stream_id = stream_id as u64;
        let key: String = args.get(1).cloned().unwrap_or_default().try_js_into(ctx)?;
        let args_json: String = args
            .get(2)
            .cloned()
            .unwrap_or_default()
            .try_js_into(ctx)
            .unwrap_or_else(|_| "{}".to_string());

        let args_value: serde_json::Value =
            serde_json::from_str(&args_json).unwrap_or(serde_json::Value::Null);

        match plugin::dispatch_command(&key, args_value) {
            plugin::Dispatched::SpawnStream(handler, args) => {
                state::spawn_stream_command(stream_id, handler, args)
            }
            // Unknown key — propagate dispatch's own message.
            plugin::Dispatched::Ready(Err(msg)) => state::error_stream(stream_id, msg),
            // Wrong flavour: sync/async command invoked as a stream.
            plugin::Dispatched::Ready(Ok(_)) => {
                state::error_stream(stream_id, format!("'{key}' is not a stream command"))
            }
            plugin::Dispatched::Spawn(_, _) => state::error_stream(
                stream_id,
                format!("'{key}' is an async (non-stream) command"),
            ),
        }

        Ok(JsValue::undefined())
    });

    let stream_cancel = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let stream_id: f64 = args.first().cloned().unwrap_or_default().try_js_into(ctx)?;
        state::cancel_stream(stream_id as u64);
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
pub(crate) fn register_raf(ctx: &mut JsContext) -> boa_engine::JsResult<()> {
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
pub(crate) fn register_performance(ctx: &mut JsContext) -> boa_engine::JsResult<()> {
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
