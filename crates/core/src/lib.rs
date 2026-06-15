// gluxe — Boa + GPUI engine library
//
// Public API:
//   BundleSource   — where to load the JS bundle from
//   RuntimeOptions — window / behaviour configuration
//   RuntimeBuilder — builder for runtime configuration
//   AppConfig      — runtime settings read from the bundle manifest
//   Plugin / PluginBuilder — plugin system (Tauri-inspired)
//   run(source, options) — start the engine; blocks until the window closes

// Let `#[gluxe::command]`-generated code — which references `::gluxe::…` — resolve
// inside this crate too, so the macro can be exercised by our own unit tests.
extern crate self as gluxe;

mod anim;
mod assets;
mod bridge;
mod component;
// Dev-mode hot reload (GLUXE_DEV_DIST + file watcher). Debug builds only;
// release builds always run the embedded bundle.
#[cfg(debug_assertions)]
mod dev;
mod jobs;
mod macro_support;
mod model;
mod plugin;
mod render;
mod stack;
mod state;
mod style;
mod text_input;

// Re-exported so host apps and native-component authors use our pinned crates
// (`gluxe::include_dir!`, `gluxe::gpui::…`) without their own deps — one source
// of truth keeps `Dir`/gpui types from drifting between core and the app.
pub use ::gpui;
pub use ::include_dir;
pub use ::include_dir::{Dir, include_dir};
pub use component::{Component, NativeRenderContext, NativeRenderFn};
pub use plugin::{CommandResult, Plugin, PluginBuilder, StreamCommandHandler, StreamSink};

// The `#[gluxe::command]` attribute. Its generated code references the hidden
// `gluxe::__macro` module below; the `commands!` declarative macro (defined via
// `#[macro_export]` in this crate, callable as `gluxe::commands!`) collects the
// generated specs for `PluginBuilder::commands`.
pub use gluxe_macros::command;

/// Support items for `#[gluxe::command]`-generated code. Not a stable API —
/// referenced only by the macro and `commands!`.
#[doc(hidden)]
pub mod __macro {
    pub use crate::macro_support::{CommandSpec, IntoCommandResult, Value, extract};
}

/// Collect `#[command]`-annotated functions into specs for
/// [`PluginBuilder::commands`]:
///
/// ```ignore
/// PluginBuilder::new("fs")
///     .commands(gluxe::commands![read_text_file, write_text_file])
///     .build()
/// ```
#[macro_export]
macro_rules! commands {
    ($($name:ident),* $(,)?) => {
        [ $( $name::__spec() ),* ]
    };
}

use std::rc::Rc;

use boa_engine::Context as JsContext;
use gpui::{
    App, AppContext, Bounds, KeyBinding, SharedString, TitlebarOptions, WindowBounds,
    WindowDecorations, WindowHandle, WindowOptions, px, size,
};
use gpui_platform::application;

use crate::{
    render::{FocusNext, FocusPrev, ROOT_KEY_CONTEXT, RootView},
    state::{
        arm_next_timer, clock_now_ms, flush_commands, get_focus_handle, raf_frame_fired,
        raf_try_arm, resolve_pending_invokes, resolve_pending_streams, run_boa_jobs,
        run_raf_callbacks, set_bg_executor, set_boa, take_frame_fired, take_pending_focus,
        take_window_commands, wake_receiver,
    },
    text_input::{
        Backspace, Copy as CopyAction, Cut, Delete, End, Enter, Home, Left, Paste, Right,
        SelectAll, SelectLeft, SelectRight, ShowCharacterPalette,
    },
};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// File name of the bundle manifest emitted into dist/ by the gluxe
/// bundler plugin. Holds the hashed JS entry path plus the runtime-relevant
/// settings transcribed from app.json (e.g. `window`).
const BUNDLE_MANIFEST_FILE: &str = "gluxe.manifest.json";

/// Where to load the JS bundle from.
///
/// The only variant is the manifest-carrying embedded dist tree: there is
/// intentionally no way to point the runtime at a bare JS file, so every
/// runnable bundle must have gone through the bundler plugin that emits the
/// manifest. (Dev mode reads dist/ from disk but still requires the manifest.)
pub enum BundleSource {
    /// Entire dist directory embedded at compile time (JS bundle + assets).
    /// The JS entry path and window settings are read at startup from the
    /// `gluxe.manifest.json` inside `dir`:
    ///
    /// ```rust,ignore
    /// static DIST: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/dist");
    /// BundleSource::EmbeddedDir { dir: &DIST }
    /// ```
    EmbeddedDir { dir: &'static Dir<'static> },
}

impl BundleSource {
    /// Resolves the bundle into raw JS bytes, the asset source, and the
    /// manifest-provided runtime settings.
    ///
    /// Dev mode (debug builds, `GLUXE_DEV_DIST` set — see `dev.rs`): the variant
    /// is ignored and everything is loaded from the dist dir on disk, so watch
    /// builds take effect without recompiling the binary.
    fn resolve(self) -> (Vec<u8>, assets::RuntimeAssets, AppConfig) {
        #[cfg(debug_assertions)]
        if let Some(dir) = dev::dev_dist_dir() {
            let (bundle, config) = dev::load_from_disk(&dir).unwrap_or_else(|e| {
                panic!("GLUXE_DEV_DIST: {e}; run the JS bundle build (app.json `bundle.build`, e.g. `gluxe build`) first")
            });
            let entry = config
                .entry
                .as_deref()
                .expect("load_from_disk guarantees entry");
            // Seed the reloader's "current build" marker so the startup
            // build's own fs events don't trigger a redundant reload.
            dev::record_loaded(entry, &bundle);
            return (
                bundle,
                assets::RuntimeAssets::Disk(assets::DiskAssets::new(dir)),
                config,
            );
        }

        match self {
            BundleSource::EmbeddedDir { dir } => {
                let manifest = dir
                    .get_file(BUNDLE_MANIFEST_FILE)
                    .and_then(|f| f.contents_utf8())
                    .unwrap_or_else(|| {
                        panic!(
                            "{BUNDLE_MANIFEST_FILE} not found in embedded dist dir; \
                             run the JS bundle build (app.json `bundle.build`, e.g. `gluxe build`) before `cargo build`"
                        )
                    });
                let config = AppConfig::from_manifest_str(manifest);
                let entry = config
                    .entry
                    .as_deref()
                    .unwrap_or_else(|| panic!("no string `entry` field in {BUNDLE_MANIFEST_FILE}"));
                let file = dir.get_file(entry).unwrap_or_else(|| {
                    panic!("bundle entry '{entry}' not found in embedded dist dir")
                });
                (
                    file.contents().to_vec(),
                    assets::RuntimeAssets::Embedded(assets::EmbeddedAssets::new(dir)),
                    config,
                )
            }
        }
    }
}

/// Default window width when no manifest or builder value is provided.
const DEFAULT_WINDOW_WIDTH: f32 = 800.0;
/// Default window height when no manifest or builder value is provided.
const DEFAULT_WINDOW_HEIGHT: f32 = 600.0;

/// Window creation parameters.
///
/// Every field is `Option` (`None` = unspecified). Fields merge field-level —
/// builder → manifest → hard-coded defaults (800×600) — in
/// [`resolve_window_config`]. `Some(String::new())` forces a blank title;
/// `Some(false)` hides the titlebar.
///
/// Per-platform titlebar notes: `Some(false)` removes WS_CAPTION on Windows
/// (resize border remains), enables NSFullSizeContentView on macOS (traffic-
/// light buttons remain), and requests client-side decorations on Linux/X11.
#[derive(Clone, Default)]
pub struct WindowConfig {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub title: Option<String>,
    pub titlebar: Option<bool>,
}

impl WindowConfig {
    /// Fill every unset (`None`) field from `other`. Destructures `other` so
    /// that adding a field to `WindowConfig` without updating this merge is
    /// a compile error.
    fn merge_unset_from(&mut self, other: WindowConfig) {
        let WindowConfig {
            width,
            height,
            title,
            titlebar,
        } = other;
        self.width = self.width.or(width);
        self.height = self.height.or(height);
        self.title = self.title.take().or(title);
        self.titlebar = self.titlebar.or(titlebar);
    }
}

/// Options passed to the runtime.
#[derive(Default)]
pub struct RuntimeOptions {
    /// Window override; `None` falls back to the manifest then to defaults.
    pub window: Option<WindowConfig>,
    /// Plugins registered before eval; each exposes commands callable via
    /// `invoke()` from JS.
    pub plugins: Vec<Plugin>,
    /// Native GPUI components registered before eval; each becomes a host
    /// element type usable from JSX (e.g. `<Badge .../>`).
    pub components: Vec<Component>,
}

/// Runtime-relevant settings carried by the bundle manifest
/// (`dist/gluxe.manifest.json`). The bundler plugin transcribes these from
/// app.json at JS build time, so the native runtime never reads app.json.
#[derive(Clone, Default)]
pub struct AppConfig {
    /// Dist-root-relative path to the hashed JS entry file.
    pub entry: Option<String>,
    pub window: Option<WindowConfig>,
    /// Dist-root-relative path to the hashed window-icon .ico. Applied on X11
    /// only via `WindowOptions::icon`; ignored on Windows and macOS.
    pub icon: Option<String>,
}

impl AppConfig {
    pub fn from_manifest_str(json: &str) -> Self {
        let Ok(json) = serde_json::from_str::<serde_json::Value>(json) else {
            return Self::default();
        };

        let entry = json["entry"].as_str().map(str::to_owned);
        let window = json.get("window").map(|w| {
            // Reject junk dimensions (negative/zero from a hand-edited
            // manifest); `None` falls through to the defaults at window-open.
            let dim = |v: &serde_json::Value| -> Option<f32> {
                v.as_f64()
                    .filter(|n| n.is_finite() && *n > 0.0)
                    .map(|n| n as f32)
            };
            WindowConfig {
                width: dim(&w["width"]),
                height: dim(&w["height"]),
                title: w["title"].as_str().map(str::to_owned),
                titlebar: w["titlebar"].as_bool(),
            }
        });

        let icon = json["icon"].as_str().map(str::to_owned);

        Self {
            entry,
            window,
            icon,
        }
    }
}

/// Builder for [`RuntimeOptions`].
///
/// Window settings come from the manifest by default; the `window*` methods
/// only override them programmatically.
///
/// ```rust,ignore
/// fn main() {
///     gluxe::RuntimeBuilder::new()
///         .plugin(gluxe_plugin_fs::plugin())
///         .run(gluxe::embedded_dist!());
/// }
/// ```
#[derive(Default)]
pub struct RuntimeBuilder {
    options: RuntimeOptions,
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the entire window-override struct, discarding any earlier
    /// `.window_size()` / `.titlebar()` calls. `None` fields still inherit from
    /// the manifest, so call `.window()` first if you mix it with the setters.
    pub fn window(mut self, window: WindowConfig) -> Self {
        self.options.window = Some(window);
        self
    }

    /// Override width and height only. Composes with `.titlebar()` in any order.
    pub fn window_size(self, width: f32, height: f32) -> Self {
        self.with_window_field(|c| {
            c.width = Some(width);
            c.height = Some(height);
        })
    }

    /// Override titlebar visibility only (`false` = custom-titlebar mode).
    /// Composes with `.window_size()` in any order. See [`WindowConfig`] for
    /// per-platform notes.
    pub fn titlebar(self, visible: bool) -> Self {
        self.with_window_field(|c| c.titlebar = Some(visible))
    }

    /// Mutate one field of the window override without disturbing fields set by
    /// earlier builder calls.
    fn with_window_field(mut self, f: impl FnOnce(&mut WindowConfig)) -> Self {
        let mut w = self.options.window.take().unwrap_or_default();
        f(&mut w);
        self.options.window = Some(w);
        self
    }

    pub fn plugin(mut self, plugin: Plugin) -> Self {
        self.options.plugins.push(plugin);
        self
    }

    pub fn plugins<I>(mut self, plugins: I) -> Self
    where
        I: IntoIterator<Item = Plugin>,
    {
        self.options.plugins.extend(plugins);
        self
    }

    pub fn component(mut self, component: Component) -> Self {
        self.options.components.push(component);
        self
    }

    pub fn components<I>(mut self, components: I) -> Self
    where
        I: IntoIterator<Item = Component>,
    {
        self.options.components.extend(components);
        self
    }

    pub fn options(self) -> RuntimeOptions {
        self.options
    }

    /// Start the gluxe engine with this configuration.
    pub fn run(self, source: BundleSource) {
        run(source, self.options);
    }
}

/// Return the host app's embedded `dist/` bundle source.
///
/// A macro so `include_dir!` expands in the host crate and sees its own
/// `dist/`. Entry path and window settings are read from the manifest inside.
#[macro_export]
macro_rules! embedded_dist {
    () => {{
        use $crate::include_dir;
        static DIST: include_dir::Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/dist");
        $crate::BundleSource::EmbeddedDir { dir: &DIST }
    }};
}

/// Decode the manifest-referenced .ico into the RGBA image gpui's X11 backend
/// applies via _NET_WM_ICON (image's ICO decoder picks the best entry itself).
/// Never fails the app: any problem logs and skips the icon.
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn load_window_icon(
    assets: &assets::RuntimeAssets,
    path: &str,
) -> Option<std::sync::Arc<image::RgbaImage>> {
    use gpui::AssetSource as _;
    let bytes = match assets.load(path) {
        Ok(Some(b)) => b,
        Ok(None) => {
            eprintln!("[gluxe] window icon '{path}' not found in dist; skipping");
            return None;
        }
        Err(e) => {
            eprintln!("[gluxe] failed to read window icon '{path}': {e}; skipping");
            return None;
        }
    };
    match image::load_from_memory_with_format(&bytes, image::ImageFormat::Ico) {
        // into_rgba8 is copy-free for the common 32-bit ICO (already RGBA8).
        Ok(img) => Some(std::sync::Arc::new(img.into_rgba8())),
        Err(e) => {
            eprintln!("[gluxe] failed to decode window icon '{path}': {e}; skipping");
            None
        }
    }
}

/// Start the gluxe engine. Blocks until the window is closed.
///
/// Boa and GPUI share the calling thread on every platform (Boa's `Context` is
/// `!Send`; AppKit requires the main thread on macOS), so call this from
/// `main()`. The thread's stack size is irrelevant to JS — every entry into Boa
/// switches onto a heap-backed stack when short on headroom (see `stack.rs`).
/// The Windows-only 8 MB linker stack from `gluxe-build::configure()` is for
/// GPUI's recursive layout/paint, not Boa.
pub fn run(source: BundleSource, options: RuntimeOptions) {
    let mut js_ctx = create_js_context().expect("JS context init failed");

    plugin::register_plugins(options.plugins);
    // Register native components before eval so `createInstance` can resolve
    // their element-type names during the very first reconciliation pass.
    component::register_components(options.components);

    let (bundle, asset_source, bundle_config) = source.resolve();
    stack::eval_on_parser_stack(&mut js_ctx, &bundle).expect("JS bundle eval failed");
    stack::with_js_stack(|| js_ctx.run_jobs()).expect("initial run_jobs failed");
    flush_commands();

    set_boa(js_ctx);

    // Dev mode: watch the dist dir so saves trigger a hot reload. The callback
    // only touches atomics + the wake channel, so starting it before the pump
    // exists is safe — the flag is simply consumed by the pump's first pass.
    #[cfg(debug_assertions)]
    if let Some(dir) = dev::dev_dist_dir() {
        dev::start_watcher(dir);
    }

    // Defaults (800×600) are applied below at window-open via `unwrap_or`.
    let win_cfg = resolve_window_config(options.window, bundle_config.window);
    // X11 window icon from the manifest. Other platforms get it elsewhere
    // (Windows: exe resource via gluxe-build; macOS/Wayland: unsupported).
    // Loaded before `asset_source` is moved into `with_assets` below.
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    let window_icon = bundle_config
        .icon
        .as_deref()
        .and_then(|p| load_window_icon(&asset_source, p));
    // The asset source backs `<Image src="asset://..."/>`; must be set before `run`.
    application()
        .with_assets(asset_source)
        .run(move |cx: &mut App| {
            // TextInput editing keys, ctrl-* (Win/Linux) + cmd-* (macOS).
            // Scoped to the `TextInput` key context (text_input.rs) so they're
            // only intercepted while a field is focused, not app-wide.
            cx.bind_keys([
                KeyBinding::new("backspace", Backspace, Some("TextInput")),
                KeyBinding::new("delete", Delete, Some("TextInput")),
                KeyBinding::new("left", Left, Some("TextInput")),
                KeyBinding::new("right", Right, Some("TextInput")),
                KeyBinding::new("shift-left", SelectLeft, Some("TextInput")),
                KeyBinding::new("shift-right", SelectRight, Some("TextInput")),
                KeyBinding::new("ctrl-a", SelectAll, Some("TextInput")),
                KeyBinding::new("cmd-a", SelectAll, Some("TextInput")),
                KeyBinding::new("home", Home, Some("TextInput")),
                KeyBinding::new("end", End, Some("TextInput")),
                KeyBinding::new("enter", Enter, Some("TextInput")),
                KeyBinding::new("ctrl-c", CopyAction, Some("TextInput")),
                KeyBinding::new("cmd-c", CopyAction, Some("TextInput")),
                KeyBinding::new("ctrl-v", Paste, Some("TextInput")),
                KeyBinding::new("cmd-v", Paste, Some("TextInput")),
                KeyBinding::new("ctrl-x", Cut, Some("TextInput")),
                KeyBinding::new("cmd-x", Cut, Some("TextInput")),
                KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, Some("TextInput")),
            ]);
            // Tab navigation, handled by the root's `on_action`. Scoped to the root
            // key context so a deeper context (e.g. "TextInput") can override `tab`.
            cx.bind_keys([
                KeyBinding::new("tab", FocusNext, Some(ROOT_KEY_CONTEXT)),
                KeyBinding::new("shift-tab", FocusPrev, Some(ROOT_KEY_CONTEXT)),
            ]);
            // Real HTTP client for remote `<Image src="https://..."/>`; without
            // it GPUI's NullHttpClient silently drops every URI load. Behind the
            // `http` feature so apps without remote images skip the TLS deps.
            #[cfg(feature = "http")]
            cx.set_http_client(std::sync::Arc::new(
                reqwest_client::ReqwestClient::user_agent("gluxe")
                    .expect("failed to build HTTP client"),
            ));

            let bounds = Bounds::centered(
                None,
                size(
                    px(win_cfg.width.unwrap_or(DEFAULT_WINDOW_WIDTH)),
                    px(win_cfg.height.unwrap_or(DEFAULT_WINDOW_HEIGHT)),
                ),
                cx,
            );
            // `titlebar: false` → hide via appears_transparent (WS_CAPTION
            // removal / NSFullSizeContentView), plus client-side decorations on
            // Linux/X11. See WindowConfig for the per-platform breakdown.
            let hide_titlebar = win_cfg.titlebar == Some(false);
            let root: WindowHandle<RootView> = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        titlebar: Some(TitlebarOptions {
                            title: win_cfg.title.clone().map(SharedString::from),
                            appears_transparent: hide_titlebar,
                            ..Default::default()
                        }),
                        // Linux/X11 frameless windows; ignored on Windows/macOS.
                        window_decorations: hide_titlebar.then_some(WindowDecorations::Client),
                        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
                        icon: window_icon,
                        ..Default::default()
                    },
                    |_, cx| cx.new(RootView::new),
                )
                .unwrap();
            cx.activate(true);

            // Hand the background executor to `state` so async plugin commands can
            // be offloaded off the Boa/UI thread. Must happen before the pump runs.
            set_bg_executor(cx.background_executor().clone());

            cx.spawn(async move |cx| {
                let wake_rx = wake_receiver();
                loop {
                    // Dev mode: commit a pending hot reload first, so the rest of
                    // this same pass flushes the new bundle's mount commands into
                    // the fresh tree. The notify is defensive — the mount's
                    // AppendToContainer already dirties the root via the flush.
                    #[cfg(debug_assertions)]
                    if dev::poll_reload() {
                        let _ = root.update(cx, |_, _, cx| cx.notify());
                    }
                    // Settle any completed `invoke` Promises first, so the .then
                    // microtasks they schedule are flushed by run_boa_jobs() below.
                    resolve_pending_invokes();
                    // Deliver any queued stream chunks (`invokeStream`) before
                    // run_boa_jobs() too, for the same reason: the microtasks each
                    // chunk schedules flush this same pass.
                    resolve_pending_streams();
                    // Read the frame flag once so the rAF batch and the transition
                    // tick below both observe the same GPUI frame.
                    let frame_fired = take_frame_fired();
                    // Run the requestAnimationFrame batch (no-op unless a GPUI
                    // frame fired since the last pass) before run_boa_jobs so the
                    // setState microtasks the batch schedules flush this same pass.
                    run_raf_callbacks(frame_fired);
                    run_boa_jobs();
                    let mut outcome = flush_commands().unwrap_or_default();
                    // Advance style transitions once per GPUI frame. Placed after
                    // the flush so a transition started this pass renders its t=0
                    // frame from the dirty mark `apply_command` already produced.
                    if frame_fired {
                        if let Some(now) = clock_now_ms() {
                            outcome.dirty_nodes.extend(anim::tick(now));
                        }
                    }
                    if !outcome.is_empty() {
                        let _ = root.update(cx, |root_view, _, cx| {
                            if root_view.apply_outcome(outcome, cx) {
                                cx.notify();
                            }
                        });
                    }
                    // Drain window commands (e.g. setWindowTitle) and deferred-focus
                    // retries. After outcome so a change in the same pass still applies.
                    let window_cmds = take_window_commands();
                    let pending_focus = take_pending_focus();
                    if !window_cmds.is_empty() || !pending_focus.is_empty() {
                        let _ = root.update(cx, |_, window, cx| {
                            // Retry focus once the handle has been painted.
                            for (id, retries_left) in pending_focus {
                                if let Some(handle) = get_focus_handle(id, cx) {
                                    window.focus(&handle, cx);
                                } else if retries_left > 0 {
                                    state::defer_focus(id, retries_left - 1);
                                }
                            }
                            for cmd in window_cmds {
                                match cmd {
                                    state::WindowCommand::SetTitle(t) => {
                                        window.set_window_title(&t);
                                    }
                                    state::WindowCommand::FocusElement(id) => {
                                        if let Some(handle) = get_focus_handle(id, cx) {
                                            window.focus(&handle, cx);
                                        } else {
                                            // Not painted yet (e.g. focus on mount) — retry.
                                            state::defer_focus(id, state::FOCUS_RETRY_BUDGET);
                                        }
                                    }
                                    state::WindowCommand::BlurElement(id) => {
                                        // Blur only if this element holds focus, so it
                                        // can't steal focus from a sibling.
                                        if get_focus_handle(id, cx)
                                            .is_some_and(|h| h.is_focused(window))
                                        {
                                            window.blur();
                                        }
                                    }
                                }
                            }
                        });
                    }
                    // Re-run after the next frame to retry focus once handles are painted.
                    if state::has_pending_focus() {
                        let _ = root.update(cx, |_, window, cx| {
                            window.on_next_frame(|_, _| state::signal_wake());
                            cx.notify();
                        });
                    }
                    // Hook the next GPUI frame for pending rAF callbacks. After
                    // the flush so rAFs re-requested by the batch re-arm for the
                    // following frame. `on_next_frame` alone never dirties the
                    // window, so pair it with `cx.notify()` (cheap — child
                    // NodeViews only re-render when individually notified) to
                    // guarantee an idle window still produces the frame.
                    if raf_try_arm() {
                        let _ = root.update(cx, |_, window, cx| {
                            window.on_next_frame(|_, _| raf_frame_fired());
                            cx.notify();
                        });
                    }
                    // Arm a background timer for the soonest pending JS timer so the
                    // pump is woken to fire it. No-op when no timers are pending, so
                    // an idle window still costs zero wakeups.
                    arm_next_timer();
                    // Park until something signals work (an event handler ran, or an
                    // invoke result arrived). Event-driven, so an idle window costs
                    // zero wakeups. Draining the channel after recv coalesces a burst
                    // of signals into the single drain pass we already did above.
                    if wake_rx.recv().await.is_err() {
                        break; // all senders dropped → shutting down
                    }
                    while wake_rx.try_recv().is_ok() {}
                }
            })
            .detach();
        });
}

// ---------------------------------------------------------------------------
// Window config helpers
// ---------------------------------------------------------------------------

/// Merge builder and manifest window configs field-level: the builder wins,
/// the manifest fills any `None`, and the defaults (800×600) are applied later
/// at window-open. A partial builder override (e.g. `.titlebar()` only) still
/// inherits the manifest's other fields. Extracted for unit testing.
fn resolve_window_config(
    builder: Option<WindowConfig>,
    manifest: Option<WindowConfig>,
) -> WindowConfig {
    let mut cfg = builder.unwrap_or_default();
    if let Some(m) = manifest {
        cfg.merge_unset_from(m);
    }
    cfg
}

// ---------------------------------------------------------------------------
// Engine internals
// ---------------------------------------------------------------------------

/// Build a Boa context with the executor, runtime extensions, and every bridge
/// global registered — everything a bundle needs before eval.
///
/// Shared by startup and dev-mode hot reload (each reload gets a fresh
/// context; globals registered into the old one die with it). Plugins and
/// native components are *not* part of this: they live in context-independent
/// registries populated once at startup and survive reloads.
pub(crate) fn create_js_context() -> Result<JsContext, String> {
    // Install our custom job executor so `setTimeout`/`setInterval` fire
    // incrementally instead of busy-blocking `run_jobs()`. See `jobs.rs`.
    let mut js_ctx = JsContext::builder()
        .job_executor(Rc::new(jobs::GpuiJobExecutor::default()))
        .build()
        .map_err(|e| format!("Boa context build failed: {e}"))?;

    boa_runtime::register(
        boa_runtime::extensions::ConsoleExtension::default(),
        None,
        &mut js_ctx,
    )
    .map_err(|e| format!("boa_runtime registration failed: {e}"))?;

    bridge::register_bridge(&mut js_ctx).map_err(|e| format!("bridge registration failed: {e}"))?;
    bridge::register_invoke(&mut js_ctx).map_err(|e| format!("invoke registration failed: {e}"))?;
    bridge::register_stream(&mut js_ctx).map_err(|e| format!("stream registration failed: {e}"))?;
    bridge::register_raf(&mut js_ctx).map_err(|e| format!("raf registration failed: {e}"))?;
    bridge::register_performance(&mut js_ctx)
        .map_err(|e| format!("performance registration failed: {e}"))?;

    Ok(js_ctx)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- AppConfig::from_manifest_str ----

    #[test]
    fn manifest_with_title_parses_title() {
        let json = r#"{"version":1,"entry":"assets/index.js","window":{"width":1024,"height":768,"title":"My App"}}"#;
        let config = AppConfig::from_manifest_str(json);
        let window = config.window.expect("window present");
        assert_eq!(window.title.as_deref(), Some("My App"));
        assert_eq!(window.width, Some(1024.0));
        assert_eq!(window.height, Some(768.0));
    }

    #[test]
    fn manifest_window_without_title_gives_none() {
        let json = r#"{"version":1,"entry":"assets/index.js","window":{"width":800,"height":600}}"#;
        let config = AppConfig::from_manifest_str(json);
        let window = config.window.expect("window present");
        assert!(window.title.is_none());
    }

    #[test]
    fn manifest_without_window_key_gives_no_window() {
        let json = r#"{"version":1,"entry":"assets/index.js"}"#;
        let config = AppConfig::from_manifest_str(json);
        assert!(config.window.is_none());
    }

    #[test]
    fn manifest_with_junk_dimensions_gives_none() {
        // Negative / zero dimensions (hand-edited manifest) must not reach
        // px()/size(); they parse to None and the defaults apply at window-open time.
        let json = r#"{"version":1,"entry":"assets/index.js","window":{"width":-100,"height":0}}"#;
        let config = AppConfig::from_manifest_str(json);
        let window = config.window.expect("window present");
        assert_eq!(window.width, None);
        assert_eq!(window.height, None);
    }

    #[test]
    fn manifest_with_title_only_has_none_dimensions() {
        let json = r#"{"version":1,"entry":"assets/index.js","window":{"title":"Titled"}}"#;
        let config = AppConfig::from_manifest_str(json);
        let window = config.window.expect("window present");
        assert_eq!(window.title.as_deref(), Some("Titled"));
        assert_eq!(window.width, None);
        assert_eq!(window.height, None);
    }

    #[test]
    fn manifest_with_icon_parses_icon() {
        let json = r#"{"version":1,"entry":"assets/index.js","icon":"assets/app-icon-Xyz.ico"}"#;
        let config = AppConfig::from_manifest_str(json);
        assert_eq!(config.icon.as_deref(), Some("assets/app-icon-Xyz.ico"));
    }

    #[test]
    fn manifest_without_icon_gives_none() {
        let json = r#"{"version":1,"entry":"assets/index.js"}"#;
        let config = AppConfig::from_manifest_str(json);
        assert!(config.icon.is_none());
    }

    #[test]
    fn manifest_with_icon_not_string_gives_none() {
        let json = r#"{"version":1,"entry":"assets/index.js","icon":5}"#;
        let config = AppConfig::from_manifest_str(json);
        assert!(config.icon.is_none());
    }

    // ---- AppConfig::from_manifest_str — titlebar ----

    #[test]
    fn manifest_titlebar_false_parses_some_false() {
        let json = r#"{"version":1,"entry":"assets/index.js","window":{"width":800,"height":600,"titlebar":false}}"#;
        let config = AppConfig::from_manifest_str(json);
        let window = config.window.expect("window present");
        assert_eq!(window.titlebar, Some(false));
    }

    #[test]
    fn manifest_titlebar_true_parses_some_true() {
        let json = r#"{"version":1,"entry":"assets/index.js","window":{"width":800,"height":600,"titlebar":true}}"#;
        let config = AppConfig::from_manifest_str(json);
        let window = config.window.expect("window present");
        assert_eq!(window.titlebar, Some(true));
    }

    #[test]
    fn manifest_titlebar_absent_gives_none() {
        let json = r#"{"version":1,"entry":"assets/index.js","window":{"width":800,"height":600}}"#;
        let config = AppConfig::from_manifest_str(json);
        let window = config.window.expect("window present");
        assert!(window.titlebar.is_none());
    }

    #[test]
    fn manifest_titlebar_non_bool_gives_none() {
        // Non-bool values (numbers, strings, null) must be ignored — only actual
        // JSON booleans are valid for the titlebar field.
        let json = r#"{"version":1,"entry":"assets/index.js","window":{"width":800,"height":600,"titlebar":"false"}}"#;
        let config = AppConfig::from_manifest_str(json);
        let window = config.window.expect("window present");
        assert!(window.titlebar.is_none());
    }

    // ---- resolve_window_config ----

    #[test]
    fn resolve_builder_none_manifest_some_uses_manifest() {
        let manifest = Some(WindowConfig {
            width: Some(1024.0),
            height: Some(768.0),
            title: Some("Manifest Title".to_string()),
            titlebar: Some(false),
        });
        let result = resolve_window_config(None, manifest);
        assert_eq!(result.width, Some(1024.0));
        assert_eq!(result.title.as_deref(), Some("Manifest Title"));
        assert_eq!(result.titlebar, Some(false));
    }

    #[test]
    fn resolve_builder_overrides_manifest_dimensions_inherits_title_and_titlebar() {
        // .window_size() only sets width/height, leaving title and titlebar as None.
        // Those fields should be inherited from the manifest.
        let builder = Some(WindowConfig {
            width: Some(1280.0),
            height: Some(720.0),
            title: None,
            titlebar: None,
        });
        let manifest = Some(WindowConfig {
            width: Some(800.0),
            height: Some(600.0),
            title: Some("My App".to_string()),
            titlebar: Some(false),
        });
        let result = resolve_window_config(builder, manifest);
        // Dimensions come from the builder (it wins over manifest).
        assert_eq!(result.width, Some(1280.0));
        assert_eq!(result.height, Some(720.0));
        // title and titlebar are inherited from the manifest because the builder
        // left them as None (field-level merge).
        assert_eq!(result.title.as_deref(), Some("My App"));
        assert_eq!(result.titlebar, Some(false));
    }

    #[test]
    fn resolve_builder_some_true_titlebar_beats_manifest_false() {
        // An explicit builder `titlebar: Some(true)` must win over a manifest
        // `titlebar: Some(false)` — the builder is highest priority.
        let builder = Some(WindowConfig {
            width: Some(800.0),
            height: Some(600.0),
            title: None,
            titlebar: Some(true),
        });
        let manifest = Some(WindowConfig {
            width: Some(800.0),
            height: Some(600.0),
            title: None,
            titlebar: Some(false),
        });
        let result = resolve_window_config(builder, manifest);
        assert_eq!(result.titlebar, Some(true));
    }

    #[test]
    fn resolve_both_none_gives_all_none() {
        // Defaults (800×600) are applied at window-open time, not here.
        let result = resolve_window_config(None, None);
        assert_eq!(result.width, None);
        assert_eq!(result.height, None);
        assert!(result.title.is_none());
        assert!(result.titlebar.is_none());
    }

    // ---- resolve_window_config — regression tests for builder bugs ----

    #[test]
    fn resolve_titlebar_only_builder_inherits_manifest_size_and_title() {
        // Bug 1 regression: `.titlebar(false)` alone must NOT fabricate 800×600.
        // Width/height/title should come from the manifest.
        let builder = Some(WindowConfig {
            titlebar: Some(false),
            ..Default::default()
        });
        let manifest = Some(WindowConfig {
            width: Some(1024.0),
            height: Some(768.0),
            title: Some("App".to_string()),
            titlebar: None,
        });
        let result = resolve_window_config(builder, manifest);
        assert_eq!(result.width, Some(1024.0));
        assert_eq!(result.height, Some(768.0));
        assert_eq!(result.title.as_deref(), Some("App"));
        assert_eq!(result.titlebar, Some(false));
    }

    #[test]
    fn builder_titlebar_then_window_size_compose() {
        // `.titlebar(false).window_size(1280.0, 720.0)` — order 1
        let result = RuntimeBuilder::new()
            .titlebar(false)
            .window_size(1280.0, 720.0)
            .options
            .window
            .expect("window override set");
        assert_eq!(result.width, Some(1280.0));
        assert_eq!(result.height, Some(720.0));
        assert_eq!(result.titlebar, Some(false));
    }

    #[test]
    fn builder_window_size_then_titlebar_compose() {
        // `.window_size(1280.0, 720.0).titlebar(false)` — order 2 (must equal order 1)
        let result = RuntimeBuilder::new()
            .window_size(1280.0, 720.0)
            .titlebar(false)
            .options
            .window
            .expect("window override set");
        assert_eq!(result.width, Some(1280.0));
        assert_eq!(result.height, Some(720.0));
        assert_eq!(result.titlebar, Some(false));
    }

    #[test]
    fn builder_window_replaces_earlier_titlebar() {
        // `.titlebar(false).window(cfg)` — `.window()` is a full struct replace.
        let cfg = WindowConfig {
            width: Some(640.0),
            height: Some(480.0),
            ..Default::default()
        };
        let result = RuntimeBuilder::new()
            .titlebar(false)
            .window(cfg)
            .options
            .window
            .expect("window override set");
        // The `.window()` call replaced the entire override, so titlebar is now None.
        assert_eq!(result.width, Some(640.0));
        assert_eq!(result.height, Some(480.0));
        assert_eq!(result.titlebar, None);
    }
}
