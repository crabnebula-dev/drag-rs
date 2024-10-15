pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_drag_as_window::init())
        .run(tauri::generate_context!("./tauri.conf.json"))
        .expect("failed to run app");
}
