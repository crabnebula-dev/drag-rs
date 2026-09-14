#[cfg_attr(feature = "cef", tauri_runtime_cef::cef_entry_point)]
pub fn run() {
  #[cfg(feature = "cef")]
  let builder = tauri::Builder::default().runtime(tauri_runtime_cef::Cef::default());
  #[cfg(not(feature = "cef"))]
  let builder = tauri::Builder::default().runtime(tauri_runtime_wry::Wry::default());

    builder
        .plugin(tauri_plugin_drag_as_window::init())
        .run(tauri::generate_context!("./tauri.conf.json"))
        .expect("failed to run app");
}
