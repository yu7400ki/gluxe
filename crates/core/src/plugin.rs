// gluxe plugin system — generic command registry (Tauri-inspired).
//
// Usage (host side):
//   let plugin = PluginBuilder::new("fs")
//       .command("readTextFile", |args| { ... Ok(json!("...")) })
//       .build();
//   run(source, RuntimeOptions { plugins: vec![plugin], ..Default::default() });
//
// Usage (JS side):
//   const result = await invoke("fs|readTextFile", { path: "/foo.txt" });
//
// Each command is addressed by "{plugin_name}|{command_name}"; args and return
// are JSON values (or a String error). Three flavours:
//   - `command(...)`       — sync, inline on the Boa main thread; CPU-light work
//                            only (blocks the UI thread).
//   - `async_command(...)` — on a GPUI background thread; the JS `invoke` Promise
//                            resolves when the pump picks up the result. For I/O.
//   - `stream_command(...)`— on a GPUI background thread; pushes many chunks to a
//                            `StreamSink` → JS `ReadableStream` (via `invokeStream`).

use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
};

use serde_json::Value;

use crate::model::ElementId;
use crate::state::{WindowCommand, push_window_command, signal_stream_wake};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// The result type for all plugin commands.
pub type CommandResult = Result<Value, String>;

/// Synchronous command handler. `Fn`/`'static` so it can be called repeatedly
/// from the `thread_local!` registry.
pub type CommandHandler = Box<dyn Fn(Value) -> CommandResult + 'static>;

/// Async command handler. `Arc<… + Send + Sync>` so it can be cloned out of the
/// thread-local registry and moved into a future on a GPUI background thread
/// while the original stays registered.
pub type AsyncCommandHandler = Arc<dyn Fn(Value) -> CommandResult + Send + Sync + 'static>;

/// Streaming command handler. Like [`AsyncCommandHandler`] it runs on a GPUI
/// background thread, but instead of returning a single result it pushes any
/// number of chunks to the [`StreamSink`] over its lifetime, terminating with
/// [`StreamSink::end`] or [`StreamSink::error`] (or implicitly by dropping the
/// sink, which sends `End`). The JS caller receives a `ReadableStream`.
pub type StreamCommandHandler = Arc<dyn Fn(Value, StreamSink) + Send + Sync + 'static>;

/// A registered command — sync (inline on the main thread), async (single
/// result, offloaded), or stream (many chunks, offloaded).
pub(crate) enum Command {
    Sync(CommandHandler),
    Async(AsyncCommandHandler),
    Stream(StreamCommandHandler),
}

// The sole constructors of the `Command` variants — the `Box`/`Arc` wrapping
// lives here only, so `PluginBuilder` (closures) and `CommandSpec` (macro `fn`
// pointers) share one source instead of each repeating the wrapping.
impl Command {
    /// Wrap a synchronous handler (boxed; runs inline on the Boa thread).
    pub(crate) fn sync(handler: impl Fn(Value) -> CommandResult + 'static) -> Self {
        Self::Sync(Box::new(handler))
    }

    /// Wrap an async handler (`Arc`'d so it can be cloned onto a background thread).
    pub(crate) fn async_(handler: impl Fn(Value) -> CommandResult + Send + Sync + 'static) -> Self {
        Self::Async(Arc::new(handler))
    }

    /// Wrap a streaming handler (`Arc`'d so it can be cloned onto a background thread).
    pub(crate) fn stream(handler: impl Fn(Value, StreamSink) + Send + Sync + 'static) -> Self {
        Self::Stream(Arc::new(handler))
    }
}

// ---------------------------------------------------------------------------
// Streaming
// ---------------------------------------------------------------------------

/// A single message in a stream's lifetime, delivered to JS in order.
pub(crate) enum StreamMessage {
    /// A data chunk — any JSON value. Becomes one `ReadableStream` enqueue.
    Chunk(Value),
    /// Graceful completion — the JS reader resolves `{ done: true }`.
    End,
    /// Failure — the JS reader/stream rejects with this message.
    Error(String),
}

/// The producer half of a stream handed to a [`StreamCommandHandler`]. Lives on
/// one background thread; not `Sync` is fine since it is owned by a single
/// handler invocation. Methods take `&mut self` so `terminated` can be a plain
/// bool guarding against post-terminal sends.
///
/// Cancellation is **cooperative**: a long-running handler must poll
/// [`StreamSink::is_closed`] and return early when it reports `true`. There is
/// no preemption — a handler blocked in a syscall is not interrupted.
pub struct StreamSink {
    stream_id: u64,
    /// Hot-reload epoch this stream was spawned in. Stamped on every message so
    /// the consume side can drop messages from a superseded bundle (see
    /// `state::STREAM_EPOCH`).
    epoch: u64,
    tx: Sender<(u64, u64, StreamMessage)>,
    cancel: Arc<AtomicBool>,
    terminated: bool,
}

impl StreamSink {
    pub(crate) fn new(
        stream_id: u64,
        tx: Sender<(u64, u64, StreamMessage)>,
        cancel: Arc<AtomicBool>,
        epoch: u64,
    ) -> Self {
        Self {
            stream_id,
            epoch,
            tx,
            cancel,
            terminated: false,
        }
    }

    fn send(&self, msg: StreamMessage) {
        let _ = self.tx.send((self.epoch, self.stream_id, msg));
        signal_stream_wake();
    }

    /// Push a data chunk to the JS `ReadableStream`. No-op after a terminal
    /// (`end`/`error`) has been sent. Binary data should be base64-encoded into
    /// a JSON string (the bridge is JSON-only).
    pub fn chunk(&mut self, value: impl Into<Value>) {
        if self.terminated {
            return;
        }
        self.send(StreamMessage::Chunk(value.into()));
    }

    /// Close the stream gracefully (`{ done: true }` on the JS side). Idempotent.
    pub fn end(&mut self) {
        if self.terminated {
            return;
        }
        self.terminated = true;
        self.send(StreamMessage::End);
    }

    /// Terminate the stream with an error (the JS reader/stream rejects).
    /// Idempotent — only the first terminal wins.
    pub fn error(&mut self, msg: impl Into<String>) {
        if self.terminated {
            return;
        }
        self.terminated = true;
        self.send(StreamMessage::Error(msg.into()));
    }

    /// `true` once a terminal has been sent, or once JS has cancelled the
    /// stream. Poll this in producer loops to honour cancellation.
    pub fn is_closed(&self) -> bool {
        self.terminated || self.cancel.load(Ordering::Relaxed)
    }
}

impl Drop for StreamSink {
    /// A handler that returns (or panics) without an explicit terminal still
    /// closes the JS stream gracefully, so the reader resolves `{ done: true }`
    /// instead of hanging forever. Natural completion is the default.
    fn drop(&mut self) {
        if !self.terminated {
            self.send(StreamMessage::End);
        }
    }
}

/// A named collection of commands, analogous to a Tauri plugin.
pub struct Plugin {
    pub(crate) name: String,
    pub(crate) commands: HashMap<String, Command>,
}

/// Builder for [`Plugin`].
pub struct PluginBuilder {
    name: String,
    commands: HashMap<String, Command>,
}

impl PluginBuilder {
    /// Start building a plugin with the given name (e.g. `"fs"`).
    ///
    /// # Panics
    ///
    /// Names starting with `__` are reserved for built-in framework plugins
    /// (mirroring the `__bridge`/`__invoke` globals) and panic at registration
    /// time so a collision is caught at startup, not silently at dispatch.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        assert!(
            !name.starts_with("__"),
            "plugin name '{name}' is reserved: the '__' prefix is for gluxe built-ins"
        );
        Self {
            name,
            commands: HashMap::new(),
        }
    }

    /// Internal constructor for built-in plugins, which live in the reserved
    /// `__` namespace that [`PluginBuilder::new`] rejects.
    pub(crate) fn builtin(name: &str) -> Self {
        debug_assert!(name.starts_with("__"), "built-ins must use the __ prefix");
        Self {
            name: name.to_owned(),
            commands: HashMap::new(),
        }
    }

    /// Register a synchronous command handler, keyed by the short name without
    /// the plugin prefix (e.g. `"readTextFile"`). Runs inline on the Boa main
    /// thread, blocking the UI; for slow work use [`PluginBuilder::async_command`].
    pub fn command(
        mut self,
        name: impl Into<String>,
        handler: impl Fn(Value) -> CommandResult + 'static,
    ) -> Self {
        self.commands.insert(name.into(), Command::sync(handler));
        self
    }

    /// Register an asynchronous command handler. Runs on a GPUI background
    /// thread (so it must be `Send + Sync`); the calling JS `invoke` Promise
    /// stays pending until the result arrives on a later pump tick.
    pub fn async_command(
        mut self,
        name: impl Into<String>,
        handler: impl Fn(Value) -> CommandResult + Send + Sync + 'static,
    ) -> Self {
        self.commands.insert(name.into(), Command::async_(handler));
        self
    }

    /// Register a streaming command handler. Runs on a GPUI background thread
    /// (so it must be `Send + Sync`); instead of a single result it pushes
    /// chunks to the [`StreamSink`] over time. The JS caller obtains a
    /// `ReadableStream` via `invokeStream` rather than a `Promise` via `invoke`.
    /// Honour cancellation by polling [`StreamSink::is_closed`] in producer loops.
    pub fn stream_command(
        mut self,
        name: impl Into<String>,
        handler: impl Fn(Value, StreamSink) + Send + Sync + 'static,
    ) -> Self {
        self.commands.insert(name.into(), Command::stream(handler));
        self
    }

    /// Register a batch of commands produced by the `#[gluxe::command]` macro,
    /// using the [`commands!`](crate::commands) helper to collect them:
    ///
    /// ```rust,ignore
    /// PluginBuilder::new("fs")
    ///     .commands(gluxe::commands![read_text_file, write_text_file])
    ///     .build()
    /// ```
    ///
    /// Each spec already carries its name and flavour (sync/async/stream), so
    /// this composes freely with the manual `command`/`async_command`/
    /// `stream_command` builders.
    pub fn commands<I>(mut self, specs: I) -> Self
    where
        I: IntoIterator<Item = crate::macro_support::CommandSpec>,
    {
        for spec in specs {
            self.commands.insert(spec.name.to_owned(), spec.command);
        }
        self
    }

    /// Finalise the plugin.
    pub fn build(self) -> Plugin {
        Plugin {
            name: self.name,
            commands: self.commands,
        }
    }
}

// ---------------------------------------------------------------------------
// Thread-local command registry
// ---------------------------------------------------------------------------

thread_local! {
    /// Flat map of `"{plugin}|{command}"` → command.
    /// Populated once by `register_plugins` before the JS bundle is eval'd.
    static COMMANDS: RefCell<HashMap<String, Command>> =
        RefCell::new(HashMap::new());
}

/// Outcome of looking up a command by key.
pub(crate) enum Dispatched {
    /// A synchronous command (or unknown-key error) — already executed inline.
    Ready(CommandResult),
    /// An async command — the caller must run `handler(args)` on a background
    /// thread.  The handler is a cheap `Arc` clone; `args` is moved along.
    Spawn(AsyncCommandHandler, Value),
    /// A streaming command — the caller must run `handler(args, sink)` on a
    /// background thread, wiring up a [`StreamSink`]. Cheap `Arc` clone + moved `args`.
    SpawnStream(StreamCommandHandler, Value),
}

/// Built-in "__window" plugin providing runtime control over the native window.
///
/// Lives in the reserved `__` namespace ([`PluginBuilder::new`] rejects it for
/// user plugins), so it can never collide with an app-registered plugin.
fn builtin_window_plugin() -> Plugin {
    PluginBuilder::builtin("__window")
        .command("setTitle", |args| {
            let title = args["title"]
                .as_str()
                .ok_or_else(|| "__window|setTitle: missing string `title` argument".to_string())?
                .to_owned();
            push_window_command(WindowCommand::SetTitle(title));
            Ok(Value::Null)
        })
        .build()
}

/// Read a required numeric `id` argument as an [`ElementId`].
fn require_id(args: &Value, cmd: &str) -> Result<ElementId, String> {
    args["id"]
        .as_u64()
        .ok_or_else(|| format!("{cmd}: missing numeric `id` argument"))
}

/// Built-in "__focus" plugin: programmatic focus control (the `ref.focus()` /
/// `ref.blur()` JS methods dispatch here). Reserved `__` namespace, like `__window`.
fn builtin_focus_plugin() -> Plugin {
    PluginBuilder::builtin("__focus")
        .command("focus", |args| {
            let id = require_id(&args, "__focus|focus")?;
            push_window_command(WindowCommand::FocusElement(id));
            Ok(Value::Null)
        })
        .command("focusFirstIn", |args| {
            let id = require_id(&args, "__focus|focusFirstIn")?;
            push_window_command(WindowCommand::FocusFirstIn(id));
            Ok(Value::Null)
        })
        .command("blur", |args| {
            let id = require_id(&args, "__focus|blur")?;
            push_window_command(WindowCommand::BlurElement(id));
            Ok(Value::Null)
        })
        .build()
}

/// Insert all plugins' commands into `COMMANDS`. Called from `run` before eval.
/// Built-ins own the reserved `__` namespace, so the only possible duplicates
/// are user plugins sharing a name.
pub(crate) fn register_plugins(plugins: Vec<Plugin>) {
    COMMANDS.with(|c| {
        let mut map = c.borrow_mut();
        let builtins = [builtin_window_plugin(), builtin_focus_plugin()];
        for plugin in builtins.into_iter().chain(plugins) {
            for (cmd_name, handler) in plugin.commands {
                let key = format!("{}|{}", plugin.name, cmd_name);
                if map.insert(key.clone(), handler).is_some() {
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "[gluxe] plugin command '{key}' registered twice — \
                         last one wins"
                    );
                }
            }
        }
    });
}

/// Look up a command by full key (e.g. `"fs|readTextFile"`) and dispatch it.
/// Sync commands run inline → [`Dispatched::Ready`]; async commands return
/// [`Dispatched::Spawn`] for the caller to run; unknown keys → `Ready(Err(..))`.
pub(crate) fn dispatch_command(key: &str, args: Value) -> Dispatched {
    COMMANDS.with(|c| {
        let map = c.borrow();
        match map.get(key) {
            Some(Command::Sync(handler)) => Dispatched::Ready(handler(args)),
            Some(Command::Async(handler)) => Dispatched::Spawn(handler.clone(), args),
            Some(Command::Stream(handler)) => Dispatched::SpawnStream(handler.clone(), args),
            None => Dispatched::Ready(Err(format!("unknown command: {key}"))),
        }
    })
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{WindowCommand, take_window_commands};

    #[test]
    #[should_panic(expected = "reserved")]
    fn user_plugin_cannot_use_reserved_prefix() {
        let _ = PluginBuilder::new("__window");
    }

    #[test]
    fn builtin_window_set_title_dispatches() {
        // Thread-local registries, so this test sees its own clean COMMANDS.
        register_plugins(Vec::new());
        let result = dispatch_command("__window|setTitle", serde_json::json!({ "title": "T" }));
        match result {
            Dispatched::Ready(Ok(Value::Null)) => {}
            _ => panic!("expected Ready(Ok(Null))"),
        }
        let cmds = take_window_commands();
        assert!(matches!(&cmds[..], [WindowCommand::SetTitle(t)] if t == "T"));
    }

    #[test]
    fn builtin_window_set_title_requires_string_title() {
        register_plugins(Vec::new());
        let result = dispatch_command("__window|setTitle", serde_json::json!({ "title": 42 }));
        match result {
            Dispatched::Ready(Err(msg)) => assert!(msg.contains("missing string `title`")),
            _ => panic!("expected Ready(Err(..))"),
        }
        assert!(take_window_commands().is_empty());
    }

    #[test]
    fn stream_command_dispatches_to_spawn_stream() {
        register_plugins(vec![
            PluginBuilder::new("s")
                .stream_command("tick", |_args, mut sink| sink.end())
                .async_command("once", |_args| Ok(Value::Null))
                .build(),
        ]);
        assert!(matches!(
            dispatch_command("s|tick", Value::Null),
            Dispatched::SpawnStream(_, _)
        ));
        assert!(matches!(
            dispatch_command("s|once", Value::Null),
            Dispatched::Spawn(_, _)
        ));
        assert!(matches!(
            dispatch_command("s|missing", Value::Null),
            Dispatched::Ready(Err(_))
        ));
    }

    #[test]
    fn stream_sink_drop_without_terminal_sends_end() {
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut sink = StreamSink::new(7, tx, cancel, 3);
            sink.chunk(Value::from(1));
        } // drop without explicit terminal
        let msgs: Vec<_> = rx.try_iter().collect();
        assert_eq!(msgs.len(), 2);
        // Every message carries (epoch, stream_id, message).
        assert!(matches!(msgs[0], (3, 7, StreamMessage::Chunk(_))));
        assert!(matches!(msgs[1], (3, 7, StreamMessage::End)));
    }

    #[test]
    fn stream_sink_chunk_after_end_is_noop() {
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let mut sink = StreamSink::new(1, tx, cancel, 0);
        sink.end();
        sink.chunk(Value::from("late"));
        sink.error("late");
        drop(sink); // already terminated → no extra End
        let count = rx.try_iter().count();
        assert_eq!(count, 1, "only the first terminal (End) should be sent");
    }

    // --- `#[gluxe::command]` macro integration ---------------------------------

    #[gluxe::command(name = "addUp")]
    fn add_up(a: i32, b: i32) -> Result<i32, String> {
        Ok(a + b)
    }

    #[gluxe::command(async, name = "echoUpper")]
    fn echo_upper(text: String) -> Result<String, String> {
        Ok(text.to_uppercase())
    }

    #[test]
    fn macro_sync_command_extracts_typed_args() {
        register_plugins(vec![
            PluginBuilder::new("m")
                .commands(crate::commands![add_up])
                .build(),
        ]);
        // Name override maps the snake_case ident to the camelCase JS key.
        match dispatch_command("m|addUp", serde_json::json!({ "a": 2, "b": 40 })) {
            Dispatched::Ready(Ok(v)) => assert_eq!(v, serde_json::json!(42)),
            _ => panic!("expected Ready(Ok(42))"),
        }
    }

    #[test]
    fn macro_async_command_dispatches_to_spawn() {
        register_plugins(vec![
            PluginBuilder::new("m")
                .commands(crate::commands![echo_upper])
                .build(),
        ]);
        match dispatch_command("m|echoUpper", serde_json::json!({ "text": "hi" })) {
            // The handler is a plain fn pointer — runnable inline in the test.
            Dispatched::Spawn(handler, args) => {
                assert_eq!(handler(args), Ok(serde_json::json!("HI")));
            }
            _ => panic!("expected async Spawn"),
        }
    }

    #[test]
    fn macro_command_reports_bad_argument() {
        register_plugins(vec![
            PluginBuilder::new("m")
                .commands(crate::commands![add_up])
                .build(),
        ]);
        match dispatch_command("m|addUp", serde_json::json!({ "a": "nope", "b": 1 })) {
            Dispatched::Ready(Err(msg)) => assert!(msg.contains("invalid argument `a`")),
            _ => panic!("expected Ready(Err(..)) for a non-integer `a`"),
        }
    }

    #[test]
    fn stream_sink_is_closed_reflects_cancel_flag() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let sink = StreamSink::new(1, tx, cancel.clone(), 0);
        assert!(!sink.is_closed());
        cancel.store(true, Ordering::Relaxed);
        assert!(sink.is_closed());
    }
}
