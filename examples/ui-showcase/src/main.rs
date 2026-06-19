// ui-showcase — demonstrates the headless @gluxe/ui components with styling.
//
// `embedded_dist!()` embeds the Vite `dist/` tree; the bundle entry and the
// runtime settings from app.json travel inside its gluxe.manifest.json.

fn main() {
    gluxe::RuntimeBuilder::new().run(gluxe::embedded_dist!());
}
