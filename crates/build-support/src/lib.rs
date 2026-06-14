// gluxe-build — build-script helper for gluxe projects.
//
// Call `gluxe_build::configure()` from `build.rs`. It:
//   1. Sets Cargo rebuild triggers (app.json, bundle manifest, dist dir).
//   2. Panics with a hint if dist/gluxe.manifest.json is missing (JS build must run first).
//   3. Embeds app.json's `icon` (.ico) as Win32 resource ID 1 when targeting Windows —
//      gpui loads resource ID 1 for the window class (titlebar, taskbar, Explorer).
//   4. Sets /stack:8388608 on Windows/MSVC for GPUI's recursive layout/paint,
//      mirroring upstream Zed (macOS/Linux default to 8 MB; Boa does not need it —
//      JS entry points switch to a heap-backed stack via core's stack.rs).

use std::{
    env,
    path::{Path, PathBuf},
};

const BUNDLE_MANIFEST_FILE: &str = "gluxe.manifest.json";

/// Configure the Cargo build for a gluxe project.
///
/// Call this as the sole line of your `build.rs`:
/// ```rust,no_run
/// fn main() { gluxe_build::configure(); }
/// ```
pub fn configure() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));

    let build_config = read_build_config(&manifest_dir);
    let out_dir = manifest_dir.join(&build_config.bundle_out_dir);
    let manifest_path = out_dir.join(BUNDLE_MANIFEST_FILE);

    // Trigger on app.json, the manifest, and the dist dir.
    // JS source changes are handled by the bundle command outside Cargo.
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("app.json").display()
    );
    println!("cargo:rerun-if-changed={}", manifest_path.display());
    println!("cargo:rerun-if-changed={}", out_dir.display());

    // Verify the manifest exists: fail early with a friendly hint rather than an
    // obscure include_dir!/runtime error. Contents are read at runtime, not here.
    if !manifest_path.is_file() {
        let js_build_cmd = format_build_command(&build_config);
        panic!(
            "\n\ngluxe bundle manifest not found at {}.\n\
             Run the JS bundle build before building the native binary.\n\
             Configured command: `{js_build_cmd}`.\n",
            manifest_path.display()
        );
    }

    if let Some(icon_rel) = &build_config.icon {
        embed_windows_icon(&manifest_dir, icon_rel);
    }

    // Set /stack:8388608 (8 MB) for GPUI's recursive layout/paint on Windows/MSVC,
    // mirroring Zed's own build.rs. Keyed off CARGO_CFG_TARGET_* (not cfg!) so
    // cross-compilation targets the right platform. GNU toolchain omitted: /stack: is
    // MSVC link.exe syntax and GPUI on windows-gnu is untested upstream.
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os == "windows" && target_env == "msvc" {
        println!("cargo:rustc-link-arg-bins=/stack:{}", 8 * 1024 * 1024);
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

struct BuildConfig {
    bundle_out_dir: PathBuf,
    bundle_build_command: Option<String>,
    /// `None` = `args` absent in app.json (CLI then defaults to `["run", "build"]`);
    /// `Some(vec![])` = explicit empty array (CLI runs the bare command).
    bundle_build_args: Option<Vec<String>>,
    icon: Option<PathBuf>,
}

fn read_build_config(manifest_dir: &PathBuf) -> BuildConfig {
    let path = manifest_dir.join("app.json");
    let default = BuildConfig {
        bundle_out_dir: PathBuf::from("dist"),
        bundle_build_command: None,
        bundle_build_args: None,
        icon: None,
    };

    let Ok(content) = std::fs::read_to_string(&path) else {
        return default;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
        return default;
    };

    let bundle_out_dir = json["bundle"]["outDir"]
        .as_str()
        .map(PathBuf::from)
        .unwrap_or(default.bundle_out_dir);

    let bundle_build_command = json["bundle"]["build"]["command"]
        .as_str()
        .map(str::to_owned);

    let bundle_build_args = json["bundle"]["build"]["args"].as_array().map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect()
    });

    let icon = json["icon"].as_str().map(PathBuf::from);

    BuildConfig {
        bundle_out_dir,
        bundle_build_command,
        bundle_build_args,
        icon,
    }
}

/// Format the JS build command from the app.json config for use in error messages.
///
/// Mirrors the CLI's `parseCommandSpec` (packages/cli/src/core.ts): `command`
/// and `args` default *independently* — `{"command": "pnpm"}` runs
/// `pnpm run build`, `{"args": ["build"]}` runs `npm build` — so the hint
/// matches what `gluxe build` would actually execute.
fn format_build_command(config: &BuildConfig) -> String {
    let command = config.bundle_build_command.as_deref().unwrap_or("npm");
    let mut parts = vec![command.to_owned()];
    match &config.bundle_build_args {
        Some(args) => parts.extend(args.iter().cloned()),
        None => parts.extend(["run".to_owned(), "build".to_owned()]),
    }
    parts.join(" ")
}

fn embed_windows_icon(manifest_dir: &Path, icon_rel: &Path) {
    // CARGO_MANIFEST_DIR is already absolute; do NOT canonicalize() — on
    // Windows that yields a \\?\ extended path that rc.exe rejects.
    let icon_path = manifest_dir.join(icon_rel);
    println!("cargo:rerun-if-changed={}", icon_path.display());

    if !icon_path.is_file() {
        panic!(
            "\n\napp.json `icon` points to {}, which does not exist.\n\
             Provide a .ico file at that path (relative to app.json).\n",
            icon_path.display()
        );
    }
    let is_ico = icon_path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("ico"));
    if !is_ico {
        panic!(
            "\n\napp.json `icon` must be a .ico file (got {}).\n\
             PNG is not auto-converted; provide a multi-size .ico.\n",
            icon_path.display()
        );
    }

    // embed-resource handles both msvc (rc.exe) and gnu (windres). Keyed off
    // CARGO_CFG_TARGET_OS for cross-compilation correctness.
    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "windows" {
        return;
    }

    // .rc string literals require doubled backslashes.
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let escaped = icon_path.display().to_string().replace('\\', "\\\\");
    let rc_path = out_dir.join("gluxe-icon.rc");
    std::fs::write(&rc_path, format!("1 ICON \"{escaped}\"\n"))
        .expect("failed to write gluxe-icon.rc");

    match embed_resource::compile(&rc_path, embed_resource::NONE) {
        embed_resource::CompilationResult::Ok | embed_resource::CompilationResult::NotWindows => {}
        embed_resource::CompilationResult::NotAttempted(why) => {
            // The icon was explicitly configured; a silent skip would be
            // surprising, but a missing resource compiler shouldn't fail
            // builds that otherwise work.
            println!(
                "cargo:warning=gluxe: window icon not embedded \
                 (no resource compiler available): {why}"
            );
        }
        err @ embed_resource::CompilationResult::Failed(_) => {
            panic!("gluxe: failed to embed window icon from app.json: {err}");
        }
    }
}
