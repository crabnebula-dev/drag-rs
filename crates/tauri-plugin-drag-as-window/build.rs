const COMMANDS: &[&str] = &["drag_new_window", "drag_back", "on_drop"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .global_api_script_path("./src/api-iife.js")
        .build();
}
