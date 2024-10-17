const COMMANDS: &[&str] = &["start_drag"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .global_api_script_path("./src/api-iife.js")
        .build();
}
