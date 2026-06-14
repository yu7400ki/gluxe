// file-explorer — per-project gluxe app entry point.
//
// The explicit `fn main()` form keeps startup code editable while still using
// the embedded Vite `dist/` tree (whose manifest carries the app.json window
// settings).

fn main() {
    gluxe::RuntimeBuilder::new()
        .plugin(gluxe_plugin_fs::plugin())
        .run(gluxe::embedded_dist!());
}
