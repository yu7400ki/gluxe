// Runtime half of the `#[gluxe::command]` macro, exposed as the hidden
// `gluxe::__macro` module. Only the generated code and the `commands!` helper
// reference these items; they are not a stable public surface.

use std::sync::Arc;

use serde::{Serialize, de::DeserializeOwned};

pub use serde_json::Value;

use crate::plugin::{Command, CommandResult, StreamSink};

/// One command produced by `#[command]`, collected into a plugin by
/// [`PluginBuilder::commands`](crate::PluginBuilder::commands) (usually via the
/// [`commands!`](crate::commands) helper).
///
/// The `__spec()` constructors take plain `fn` pointers — which are
/// `Copy + Send + Sync` — so the `Send + Sync` bound on async/stream handlers is
/// satisfied automatically, with none of the closure-`Copy` gymnastics the Boa
/// native-function layer needs.
pub struct CommandSpec {
    pub(crate) name: &'static str,
    pub(crate) command: Command,
}

impl CommandSpec {
    /// A synchronous command (runs inline on the Boa thread).
    pub fn sync(name: &'static str, f: fn(Value) -> CommandResult) -> Self {
        Self {
            name,
            command: Command::Sync(Box::new(f)),
        }
    }

    /// An async command (runs on a GPUI background thread, single result).
    pub fn async_(name: &'static str, f: fn(Value) -> CommandResult) -> Self {
        Self {
            name,
            command: Command::Async(Arc::new(f)),
        }
    }

    /// A streaming command (runs on a GPUI background thread, many chunks).
    pub fn stream(name: &'static str, f: fn(Value, StreamSink)) -> Self {
        Self {
            name,
            command: Command::Stream(Arc::new(f)),
        }
    }
}

/// Pull one named argument out of the JS-supplied args object and deserialize it.
/// A missing field is treated as `null`, so an `Option<T>` parameter becomes
/// `None` rather than an error.
pub fn extract<T: DeserializeOwned>(args: &Value, field: &str) -> Result<T, String> {
    let raw = args.get(field).cloned().unwrap_or(Value::Null);
    serde_json::from_value(raw).map_err(|e| format!("invalid argument `{field}`: {e}"))
}

/// Convert a command function's return value into a [`CommandResult`].
///
/// PoC scope: only `Result<T, E>` (with `T: Serialize`, `E: Display`) is
/// supported. Returning a bare `T: Serialize` would need autoref specialization;
/// deferred until there is a real need.
pub trait IntoCommandResult {
    fn into_command_result(self) -> CommandResult;
}

impl<T: Serialize, E: std::fmt::Display> IntoCommandResult for Result<T, E> {
    fn into_command_result(self) -> CommandResult {
        match self {
            Ok(value) => serde_json::to_value(value).map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        }
    }
}
