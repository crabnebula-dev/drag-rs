const COMMANDS: &[&str] = &["start_drag"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
