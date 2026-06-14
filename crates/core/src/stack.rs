// Heap-backed stack switching for JS entry points.
//
// Boa's parser/compiler is recursive-descent with **no recursion guard**;
// native-stack use is proportional to expression-nesting depth (≈150–250 KB
// per level in debug builds — unbounded in principle). VM *execution* is
// different: JS→JS calls use heap-allocated CallFrames bounded by Boa's
// default `RuntimeLimits` (recursion 512), so it needs only modest native stack.
//
// Rather than demanding a large thread stack from the host (different mechanism
// per platform, silent crash when misconfigured), we switch onto heap stacks
// exactly where the demand arises:
//
//   - [`eval_on_parser_stack`] — the **only** sanctioned `Context::eval` call
//     site. Always switches to a 256 MB stack (parser frames are huge in debug
//     builds; see `PARSER_STACK_SIZE`). Never call `Context::eval` directly.
//   - [`with_js_stack`] — wraps every `state::with_boa` callback (compiled JS).
//     Uses `maybe_grow` with a 4 MB red zone: on a normal 8 MB main thread
//     (macOS/Linux default, or Windows with the `gluxe-build` linker arg) the
//     closure runs in place at zero cost; only a 1 MB Windows main thread
//     without that arg pays for a fiber switch on every JS entry.
//
// Stacker uses guard-paged mmaps on Unix and `CreateFiber` on Windows.
// Overflows fault loudly; panics unwind across the switch. The switch stays on
// the same OS thread, so Boa's `!Send` GC state is unaffected.
//
// Windows caveat: `CreateFiber(n)` commits `n` bytes up front (RAM+pagefile;
// physical pages are lazy). The parser stack costs 256 MB of commit charge per
// eval (startup + each dev reload). On a misconfigured 1 MB main thread,
// `with_js_stack` pays a 64 MB fiber create/destroy on *every* JS entry —
// a performance cliff, not a crash.
//
// The `gluxe-build` linker stack expansion (8 MB, Windows/MSVC only) is not
// needed by Boa; it exists for GPUI's recursive layout/paint on the plain
// thread stack.

use boa_engine::{Context as JsContext, JsResult, JsValue, Source};

/// Stack for `Context::eval` (parse + compile). A depth-1000 nested expression
/// overflows 128 MB in a debug build (≈150–250 KB of frames per level), so
/// 256 MB is the floor that covers the regression test with margin. On Windows
/// this is the fiber's commit charge — see module docs.
const PARSER_STACK_SIZE: usize = 256 * 1024 * 1024;

/// Stack switched to for compiled-JS execution when headroom is short.
const VM_STACK_SIZE: usize = 64 * 1024 * 1024;

/// Headroom required before executing compiled JS — sized for the worst-case
/// native↔JS re-entry chain (512 frames × a few KB each) while staying below
/// what an 8 MB main thread has left, so the common case never allocates.
const VM_RED_ZONE: usize = 4 * 1024 * 1024;

/// Parse, compile, and execute `src` on a stack large enough for Boa's
/// unguarded parser/compiler recursion.
///
/// **Only** sanctioned `Context::eval` call site; route all evals here.
pub(crate) fn eval_on_parser_stack(ctx: &mut JsContext, src: &[u8]) -> JsResult<JsValue> {
    stacker::grow(PARSER_STACK_SIZE, || ctx.eval(Source::from_bytes(src)))
}

/// Run `f` with enough native stack for compiled-JS VM execution.
///
/// `state::with_boa` routes every JS callback through this; on a 1 MB Windows
/// main thread (without the `gluxe-build` linker arg) this pays for a fiber
/// switch — see module docs.
pub(crate) fn with_js_stack<R>(f: impl FnOnce() -> R) -> R {
    stacker::maybe_grow(VM_RED_ZONE, VM_STACK_SIZE, f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use boa_engine::js_string;

    /// Generates `1+(1+(…1…))` nested `depth` levels. Parser/compiler recursion
    /// is proportional to depth; compiled bytecode is flat, so execution is not.
    fn deeply_nested_source(depth: usize) -> String {
        let mut src = String::with_capacity(depth * 3 + 16);
        for _ in 0..depth {
            src.push_str("1+(");
        }
        src.push('1');
        src.push_str(&")".repeat(depth));
        src
    }

    /// Verifies that depth-1000 nesting (which would overflow a small thread
    /// stack) parses on a 1 MB thread via the heap-backed stack. This is the
    /// canary that sizes `PARSER_STACK_SIZE` (>128 MB needed in debug builds).
    #[test]
    fn deep_parse_succeeds_on_tiny_thread_stack() {
        std::thread::Builder::new()
            .stack_size(1024 * 1024)
            .spawn(|| {
                let mut ctx = crate::create_js_context().expect("context");
                let src = deeply_nested_source(1_000);
                let result = eval_on_parser_stack(&mut ctx, src.as_bytes());
                let value = result.expect("deeply nested source should eval");
                assert_eq!(value.to_i32(&mut ctx).expect("i32"), 1_001);
            })
            .expect("spawn")
            .join()
            .expect("deep parse thread panicked (stack overflow?)");
    }

    /// Runaway JS recursion must throw a catchable `JsError` (Boa's default
    /// `RuntimeLimits`: recursion 512), never overflow the native stack.
    #[test]
    fn runaway_js_recursion_errors_gracefully() {
        let mut ctx = crate::create_js_context().expect("context");
        let result = eval_on_parser_stack(&mut ctx, b"function f(){return f()} f()");
        assert!(
            result.is_err(),
            "infinite recursion should throw, not abort"
        );
    }

    /// `with_js_stack` executes already-compiled JS from a tiny stack (512 KB),
    /// mirroring the production split: eval at startup, `with_boa` for callbacks.
    #[test]
    fn js_stack_runs_compiled_js_on_tiny_thread_stack() {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(|| {
                let mut ctx = crate::create_js_context().expect("context");
                eval_on_parser_stack(
                    &mut ctx,
                    b"function g(){ return [1,2,3].map(x=>x*2).join(','); }",
                )
                .expect("eval");
                let g = ctx
                    .global_object()
                    .get(js_string!("g"), &mut ctx)
                    .expect("global g");
                let g = g.as_callable().expect("callable");
                let value =
                    with_js_stack(|| g.call(&JsValue::undefined(), &[], &mut ctx)).expect("call");
                let s = value
                    .as_string()
                    .expect("string")
                    .to_std_string()
                    .expect("utf8");
                assert_eq!(s, "2,4,6");
            })
            .expect("spawn")
            .join()
            .expect("js stack thread panicked");
    }
}
