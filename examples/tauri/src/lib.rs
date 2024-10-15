fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_drag::init())
        .run(tauri::generate_context!("./tauri.conf.json"))
        .expect("failed to run app");
}
