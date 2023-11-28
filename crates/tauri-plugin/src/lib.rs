// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::sync::mpsc::channel;

use serde::{ser::Serializer, Serialize};
use tauri::{
    command,
    plugin::{Builder, TauriPlugin},
    AppHandle, Runtime, Window,
};

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Drag(#[from] drag::Error),
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

#[command]
async fn start_drag<R: Runtime>(
    app: AppHandle<R>,
    window: Window<R>,
    item: drag::DragItem,
    image: drag::Image,
) -> Result<()> {
    let (tx, rx) = channel();

    app.run_on_main_thread(move || {
        #[cfg(target_os = "linux")]
        let raw_window = window.gtk_window();
        #[cfg(not(target_os = "linux"))]
        let raw_window = tauri::Result::Ok(window);

        let r = match raw_window {
            Ok(w) => drag::start_drag(&w, item, image).map_err(Into::into),
            Err(e) => Err(e.into()),
        };
        tx.send(r).unwrap();
    })?;

    rx.recv().unwrap()
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("drag")
        .invoke_handler(tauri::generate_handler![start_drag])
        .js_init_script(include_str!("./api-iife.js").to_string())
        .build()
}
