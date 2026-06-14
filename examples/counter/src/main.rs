// counter-example — per-project gluxe app entry point.
//
// `embedded_dist!()` embeds the Vite `dist/` tree; the bundle entry and the
// runtime settings from app.json travel inside its gluxe.manifest.json.
// Keeping an ordinary `fn main()` leaves room for app-specific startup code.

fn main() {
    gluxe::RuntimeBuilder::new()
        .plugin(gluxe_plugin_fs::plugin())
        .run(gluxe::embedded_dist!());
}
