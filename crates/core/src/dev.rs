// Dev-mode hot reload — debug builds only (gated by `#[cfg(debug_assertions)]` in lib.rs).
//
// `gluxe start` sets `GLUXE_DEV_DIST` to the abs path of the dist directory and
// launches the debug binary. When set:
//   - `BundleSource::resolve()` reads bundle + assets from disk (the BundleSource
//     variant is ignored entirely),
//   - `run` starts a `notify` watcher on the dist directory,
//   - on change the pump calls [`poll_reload`], which re-reads the manifest (the
//     hashed entry filename changes each build), creates a fresh Boa context, evals
//     the new bundle, and swaps it in — **full reload**: UI tree rebuilt from scratch,
//     React state lost.
//
// A failed eval rolls back: the partial mount is discarded and the old context keeps
// running — a syntax error on save never crashes the app.
//
// Release builds omit this entirely; the embedded dist is always used.

use std::cell::RefCell;
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

use notify::{RecursiveMode, Watcher};

use crate::{AppConfig, BUNDLE_MANIFEST_FILE, state};

/// Quiet period before a reload: Vite writes multiple files in unspecified order,
/// so reloading mid-write would read a torn build.
const DEBOUNCE_MS: u64 = 150;

/// Claimed (cleared) by `poll_reload` before the reload attempt so that
/// fs events arriving during the reload re-arm it rather than being lost.
static RELOAD_PENDING: AtomicBool = AtomicBool::new(false);
/// Timestamp of the most recent fs event, for debouncing.
static LAST_FS_EVENT_MS: AtomicU64 = AtomicU64::new(0);
/// Guards the single in-flight debounce/retry wake-up timer.
static RETRY_ARMED: AtomicBool = AtomicBool::new(false);
/// Keeps the watcher alive for the process lifetime (dropping it stops watching).
static WATCHER: OnceLock<Mutex<notify::RecommendedWatcher>> = OnceLock::new();

thread_local! {
    /// `(entry path, bundle hash)` of the last bundle handed to Boa.
    /// Skips duplicate/startup fs events and avoids re-eval-looping on an
    /// unchanged broken save.
    static LAST_LOADED: RefCell<Option<(String, u64)>> = const { RefCell::new(None) };
}

/// Milliseconds since process start (origin fixed on first call).
fn now_ms() -> u64 {
    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    ORIGIN.get_or_init(Instant::now).elapsed().as_millis() as u64
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut h = rustc_hash::FxHasher::default();
    h.write(bytes);
    h.finish()
}

/// Returns the dev dist directory (from `GLUXE_DEV_DIST`), or `None` with a
/// stderr warning if the path is not an existing directory.
pub(crate) fn dev_dist_dir() -> Option<PathBuf> {
    let raw = std::env::var_os("GLUXE_DEV_DIST")?;
    let path = PathBuf::from(raw);
    let abs = if path.is_absolute() {
        path
    } else {
        std::env::current_dir().ok()?.join(path)
    };
    if abs.is_dir() {
        Some(abs)
    } else {
        eprintln!(
            "[gluxe] GLUXE_DEV_DIST is set but '{}' is not a directory; \
             falling back to the embedded bundle",
            abs.display()
        );
        None
    }
}

/// Read the manifest and entry bundle from the dist dir (used at startup and each reload).
/// Errors during reload mean Vite is mid-write; the caller retries.
pub(crate) fn load_from_disk(dir: &Path) -> Result<(Vec<u8>, AppConfig), String> {
    let manifest_path = dir.join(BUNDLE_MANIFEST_FILE);
    let manifest = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("failed to read {}: {e}", manifest_path.display()))?;
    let config = AppConfig::from_manifest_str(&manifest);
    let entry = config
        .entry
        .clone()
        .ok_or_else(|| format!("no string `entry` field in {}", manifest_path.display()))?;
    let bundle = std::fs::read(dir.join(&entry))
        .map_err(|e| format!("failed to read bundle entry '{entry}': {e}"))?;
    Ok((bundle, config))
}

/// Record the currently-installed bundle (startup and each successful reload).
pub(crate) fn record_loaded(entry: &str, bundle: &[u8]) {
    LAST_LOADED.with(|l| *l.borrow_mut() = Some((entry.to_owned(), hash_bytes(bundle))));
}

fn is_last_loaded(entry: &str, hash: u64) -> bool {
    LAST_LOADED.with(|l| {
        l.borrow()
            .as_ref()
            .is_some_and(|(e, h)| e == entry && *h == hash)
    })
}

/// Start watching the dist directory. The callback runs on notify's thread
/// and only touches atomics and the thread-safe wake channel.
pub(crate) fn start_watcher(dir: PathBuf) {
    let watcher = notify::recommended_watcher(|res: Result<notify::Event, notify::Error>| {
        if res.is_err() {
            return;
        }
        LAST_FS_EVENT_MS.store(now_ms(), Ordering::Relaxed);
        RELOAD_PENDING.store(true, Ordering::Relaxed);
        state::signal_wake();
    });
    let mut watcher = match watcher {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[gluxe] failed to create file watcher: {e}; hot reload disabled");
            return;
        }
    };
    // Recursive: Vite writes the entry and assets under dist/assets/.
    if let Err(e) = watcher.watch(&dir, RecursiveMode::Recursive) {
        eprintln!(
            "[gluxe] failed to watch '{}': {e}; hot reload disabled",
            dir.display()
        );
        return;
    }
    let _ = WATCHER.set(Mutex::new(watcher));
    eprintln!("[gluxe] dev mode: watching {}", dir.display());
}

/// Arm a one-shot background timer that wakes the pump after `delay` for a
/// deferred `poll_reload` call. At most one timer in flight at a time.
fn arm_wake_timer(delay: Duration) {
    if RETRY_ARMED.swap(true, Ordering::Relaxed) {
        return;
    }
    let Some(executor) = state::bg_executor() else {
        RETRY_ARMED.store(false, Ordering::Relaxed);
        return;
    };
    let timer = executor.timer(delay);
    executor
        .spawn(async move {
            timer.await;
            RETRY_ARMED.store(false, Ordering::Relaxed);
            state::signal_wake();
        })
        .detach();
}

/// Called by the pump at the top of every pass. Commits a pending hot reload;
/// returns `true` when the Boa context was swapped (same pump pass then flushes
/// the new mount commands). Never panics — broken builds are logged and rolled back.
pub(crate) fn poll_reload() -> bool {
    if !RELOAD_PENDING.load(Ordering::Relaxed) {
        return false;
    }
    let Some(dir) = dev_dist_dir() else {
        RELOAD_PENDING.store(false, Ordering::Relaxed);
        return false;
    };

    // Debounce: wait for the dist dir to go quiet before reading it.
    let since = now_ms().saturating_sub(LAST_FS_EVENT_MS.load(Ordering::Relaxed));
    if since < DEBOUNCE_MS {
        arm_wake_timer(Duration::from_millis(DEBOUNCE_MS - since + 10));
        return false;
    }

    // Claim the pending flag before doing work: fs events during load/eval
    // re-arm it rather than being lost.
    RELOAD_PENDING.store(false, Ordering::Relaxed);

    let (bundle, config) = match load_from_disk(&dir) {
        Ok(v) => v,
        Err(_) => {
            // Vite mid-write or transiently empty dist — retry shortly.
            RELOAD_PENDING.store(true, Ordering::Relaxed);
            arm_wake_timer(Duration::from_millis(200));
            return false;
        }
    };
    let entry = config.entry.expect("load_from_disk guarantees entry");

    let hash = hash_bytes(&bundle);
    if is_last_loaded(&entry, hash) {
        return false; // already running this exact build
    }

    // Commands from the old context must flush into the old tree first; defer.
    if state::command_queue_len() > 0 {
        RELOAD_PENDING.store(true, Ordering::Relaxed);
        arm_wake_timer(Duration::from_millis(50));
        return false;
    }

    let raf_len = state::raf_queue_len();

    // Build and eval the new context while the old stays installed in BOA,
    // so a broken bundle rolls back without touching the running app.
    let mut new_ctx = match crate::create_js_context() {
        Ok(ctx) => ctx,
        Err(err) => {
            eprintln!("[gluxe] reload failed; keeping previous bundle: {err}");
            record_loaded(&entry, &bundle);
            return false;
        }
    };
    // `registerRootComponent` mounts during eval, pushing fresh-id commands onto
    // the global queue. Must run via `eval_on_parser_stack` (heap-backed stack)
    // because Boa's parser recursion is unguarded (see stack.rs).
    if let Err(err) = crate::stack::eval_on_parser_stack(&mut new_ctx, &bundle) {
        eprintln!("[gluxe] reload failed; keeping previous bundle: {err}");
        state::clear_command_queue(); // discard partial mount
        state::raf_truncate(raf_len); // drop rAFs owned by the failing context
        drop(new_ctx);
        // Record so the same broken save doesn't re-eval forever.
        record_loaded(&entry, &bundle);
        return false;
    }

    // Committed from here on.
    if let Err(err) = crate::stack::with_js_stack(|| new_ctx.run_jobs()) {
        // Sync mount already happened; log and continue (don't roll back).
        eprintln!("[gluxe] uncaught JS error during reload: {err}");
    }
    let old_ctx = state::take_boa();
    state::set_boa(new_ctx);
    // Dropping the old context kills its job executor and timers — old
    // setInterval/setTimeout callbacks can never double-fire.
    drop(old_ctx);
    // Drop rAF entries from the old context; keep those the new eval registered.
    state::raf_drop_front(raf_len);
    // Reset tree + per-element registries; command queue stays intact so this
    // pump pass flushes the new mount.
    state::reset_for_reload();
    record_loaded(&entry, &bundle);
    eprintln!("[gluxe] reloaded {entry}");
    true
}
