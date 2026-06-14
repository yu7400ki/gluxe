#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai;
mod board;
mod plugin;

fn main() {
    gluxe::RuntimeBuilder::new()
        .plugin(plugin::plugin())
        .run(gluxe::embedded_dist!());
}
