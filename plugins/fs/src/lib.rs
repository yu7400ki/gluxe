// gluxe fs plugin — exposes basic filesystem operations to JS.
//
// JS usage (after registering the plugin):
//   import { fs } from "@gluxe/plugin-fs";
//
//   const text  = await fs.readTextFile("path/to/file.txt");
//   await fs.writeTextFile("out.txt", "hello");
//   const entries = await fs.readDir(".");   // [{name, isDir, isFile, size, modified}]
//   const ok    = await fs.exists("path");  // boolean
//   await fs.mkdir("new/dir");
//   await fs.remove("path");                // file or empty dir
//   const meta  = await fs.metadata("path");// {isDir, isFile, isSymlink, size, readonly, modified, accessed, created}
//   const cwd   = await fs.cwd();           // absolute working directory
//   const home  = await fs.homeDir();       // home directory
//   const abs   = await fs.canonicalize("./foo/../bar"); // absolute normalised path
//
// Commands are declared with `#[gluxe::command]`: typed parameters are
// deserialized from the JS args object by name, and the `Result` return is
// serialized back automatically. The JS-facing (camelCase) name is set via
// `name = "..."`; the Rust ident stays snake_case.

use std::io::{self, BufRead, BufReader};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use gluxe::{Plugin, PluginBuilder, StreamSink, command, commands};

/// Return the fs plugin, ready to pass into `RuntimeOptions::plugins`.
pub fn plugin() -> Plugin {
    PluginBuilder::new("fs")
        .commands(commands![
            read_text_file,
            write_text_file,
            read_dir,
            exists,
            mkdir,
            remove,
            metadata,
            cwd,
            home_dir,
            canonicalize,
            read_file_stream,
        ])
        .build()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a `SystemTime` result to a JSON value: milliseconds since the Unix
/// epoch, or `null` when the time is unavailable or predates the epoch.
fn system_time_ms(t: std::io::Result<SystemTime>) -> Value {
    match t {
        Ok(st) => match st.duration_since(UNIX_EPOCH) {
            Ok(dur) => Value::Number((dur.as_millis() as i64).into()),
            Err(_) => Value::Null,
        },
        Err(_) => Value::Null,
    }
}

/// Strip the Windows extended-length path prefix `\\?\` (or the UNC variant
/// `\\?\UNC\`) so displayed paths are clean.  On non-Windows platforms this is
/// a no-op.
fn strip_verbatim(path: std::path::PathBuf) -> String {
    let s = path.to_string_lossy();
    #[cfg(windows)]
    {
        if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{}", rest);
        }
        if let Some(rest) = s.strip_prefix(r"\\?\") {
            return rest.to_string();
        }
    }
    s.into_owned()
}

// ---------------------------------------------------------------------------
// Command implementations
//
// All commands are `async`: filesystem I/O is blocking and must not stall the
// UI thread.
// ---------------------------------------------------------------------------

/// `readTextFile(path) -> string`
#[command(async, name = "readTextFile")]
fn read_text_file(path: String) -> io::Result<String> {
    std::fs::read_to_string(&path)
}

/// `writeTextFile(path, contents)`
#[command(async, name = "writeTextFile")]
fn write_text_file(path: String, contents: String) -> io::Result<()> {
    std::fs::write(&path, contents)
}

/// `readDir(path) -> [{name, isDir, isFile, size, modified}]`
#[command(async, name = "readDir")]
fn read_dir(path: String) -> io::Result<Value> {
    let entries = std::fs::read_dir(&path)?;
    let mut result = Vec::new();
    for entry in entries {
        let entry = entry?;
        let meta = entry.metadata()?;
        let name = entry
            .file_name()
            .into_string()
            .unwrap_or_else(|_| String::from("<non-utf8>"));
        result.push(json!({
            "name": name,
            "isDir": meta.is_dir(),
            "isFile": meta.is_file(),
            "size": meta.len(),
            "modified": system_time_ms(meta.modified()),
        }));
    }
    Ok(Value::Array(result))
}

/// `exists(path) -> boolean`
#[command(async, name = "exists")]
fn exists(path: String) -> Result<bool, String> {
    Ok(std::path::Path::new(&path).exists())
}

/// `mkdir(path)` — creates the directory and all parents (like `mkdir -p`).
#[command(async, name = "mkdir")]
fn mkdir(path: String) -> io::Result<()> {
    std::fs::create_dir_all(&path)
}

/// `remove(path)` — removes a file or an empty directory.
#[command(async, name = "remove")]
fn remove(path: String) -> io::Result<()> {
    let p = std::path::Path::new(&path);
    if p.is_dir() {
        std::fs::remove_dir(p)
    } else {
        std::fs::remove_file(p)
    }
}

/// `metadata(path) -> { isDir, isFile, isSymlink, size, readonly, modified, accessed, created }`
///
/// Uses `symlink_metadata` so that symlinks are not followed for the type
/// flags (i.e. `isSymlink` is accurate).
#[command(async, name = "metadata")]
fn metadata(path: String) -> io::Result<Value> {
    let meta = std::fs::symlink_metadata(&path)?;
    Ok(json!({
        "isDir":      meta.is_dir(),
        "isFile":     meta.is_file(),
        "isSymlink":  meta.is_symlink(),
        "size":       meta.len(),
        "readonly":   meta.permissions().readonly(),
        "modified":   system_time_ms(meta.modified()),
        "accessed":   system_time_ms(meta.accessed()),
        "created":    system_time_ms(meta.created()),
    }))
}

/// `cwd() -> string` — the process's current working directory (absolute).
#[command(async, name = "cwd")]
fn cwd() -> io::Result<String> {
    std::env::current_dir().map(strip_verbatim)
}

/// `homeDir() -> string` — the current user's home directory.
#[command(async, name = "homeDir")]
fn home_dir() -> Result<String, String> {
    std::env::home_dir()
        .map(strip_verbatim)
        .ok_or_else(|| "home directory could not be determined".to_string())
}

/// `canonicalize(path) -> string` — resolves `path` to an absolute, normalised
/// path with symlinks resolved and `..` segments removed.
#[command(async, name = "canonicalize")]
fn canonicalize(path: String) -> io::Result<String> {
    std::fs::canonicalize(&path).map(strip_verbatim)
}

/// `readFileStream(path)` → stream of `string`, one chunk per line.
///
/// Reads lazily through a `BufReader`, so a large file is never fully buffered.
/// Honours cooperative cancellation between lines (`for await` break / reader
/// `cancel()` flips `sink.is_closed()`).
#[command(stream, name = "readFileStream")]
fn read_file_stream(path: String, mut sink: StreamSink) {
    let file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            sink.error(e.to_string());
            return;
        }
    };
    for line in BufReader::new(file).lines() {
        if sink.is_closed() {
            return; // cancelled by JS — stop producing
        }
        match line {
            Ok(l) => sink.chunk(l),
            Err(e) => {
                sink.error(e.to_string());
                return;
            }
        }
    }
    sink.end();
}
