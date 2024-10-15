// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use tauri::{
    plugin::{Builder, TauriPlugin},
    Runtime,
};

mod commands;

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    #[allow(unused_mut)]
    let mut builder = Builder::new("drag-as-window");

    #[cfg(feature = "global-js")]
    {
        builder = builder.js_init_script(include_str!("./api-iife.js").to_string());
    }

    builder
        .invoke_handler(tauri::generate_handler![
            commands::drag_new_window,
            commands::drag_back,
            commands::on_drop
        ])
        .build()
}
