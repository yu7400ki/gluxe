// Runtime half of the `#[gluxe::command]` macro, exposed as the hidden
// `gluxe::__macro` module. Only the generated code and the `commands!` helper
// reference these items; they are not a stable public surface.

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
            command: Command::sync(f),
        }
    }

    /// An async command (runs on a GPUI background thread, single result).
    pub fn async_(name: &'static str, f: fn(Value) -> CommandResult) -> Self {
        Self {
            name,
            command: Command::async_(f),
        }
    }

    /// A streaming command (runs on a GPUI background thread, many chunks).
    pub fn stream(name: &'static str, f: fn(Value, StreamSink)) -> Self {
        Self {
            name,
            command: Command::stream(f),
        }
    }
}

/// Pull one named argument out of the JS-supplied args object and deserialize it.
///
/// A missing field deserializes from `null`, so an `Option<T>` parameter still
/// becomes `None` (Ok). The two non-Ok cases are reported distinctly: an absent
/// *required* (non-`Option`) field is a `missing argument \`field\`` error rather
/// than serde's confusing `invalid type: null, expected …`, while a field that is
/// present but of the wrong type is `invalid argument \`field\`: <why>`.
pub fn extract<T: DeserializeOwned>(args: &Value, field: &str) -> Result<T, String> {
    match args.get(field) {
        // Present: any deserialize failure is a genuine type mismatch.
        Some(raw) => serde_json::from_value(raw.clone())
            .map_err(|e| format!("invalid argument `{field}`: {e}")),
        // Absent: null deserializes to `None` for `Option<T>`; for a required
        // field it fails, which we report as a clear "missing argument".
        None => {
            serde_json::from_value(Value::Null).map_err(|_| format!("missing argument `{field}`"))
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn present_field_deserializes() {
        let args = serde_json::json!({ "n": 5 });
        assert_eq!(extract::<i32>(&args, "n"), Ok(5));
    }

    #[test]
    fn missing_optional_field_is_none() {
        let args = serde_json::json!({});
        assert_eq!(extract::<Option<i32>>(&args, "n"), Ok(None));
    }

    #[test]
    fn explicit_null_for_optional_field_is_none() {
        let args = serde_json::json!({ "n": null });
        assert_eq!(extract::<Option<i32>>(&args, "n"), Ok(None));
    }

    #[test]
    fn missing_required_field_reports_missing() {
        let args = serde_json::json!({});
        let err = extract::<i32>(&args, "n").unwrap_err();
        assert!(err.contains("missing argument `n`"), "{err}");
    }

    #[test]
    fn present_field_of_wrong_type_reports_invalid() {
        let args = serde_json::json!({ "n": "nope" });
        let err = extract::<i32>(&args, "n").unwrap_err();
        assert!(err.contains("invalid argument `n`"), "{err}");
    }
}
