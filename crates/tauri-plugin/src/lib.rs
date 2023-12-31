// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{collections::HashMap, path::PathBuf, sync::mpsc::channel};

use serde::{ser::Serializer, Deserialize, Serialize};
use tauri::{
    api::ipc::CallbackFn,
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

#[derive(Deserialize)]
#[serde(untagged)]
enum DragItem {
    /// A list of files to be dragged.
    ///
    /// The paths must be absolute.
    Files(Vec<PathBuf>),
    /// Data to share with another app.
    Data {
        data: SharedData,
        types: Vec<String>,
    },
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SharedData {
    Fixed(String),
    Map(HashMap<String, String>),
}

#[command]
async fn start_drag<R: Runtime>(
    app: AppHandle<R>,
    window: Window<R>,
    item: DragItem,
    image: drag::Image,
    on_event_fn: Option<CallbackFn>,
) -> Result<()> {
    let (tx, rx) = channel();

    app.run_on_main_thread(move || {
        #[cfg(target_os = "linux")]
        let raw_window = window.gtk_window();
        #[cfg(not(target_os = "linux"))]
        let raw_window = tauri::Result::Ok(window.clone());

        let r = match raw_window {
            Ok(w) => drag::start_drag(
                &w,
                match item {
                    DragItem::Files(f) => drag::DragItem::Files(f),
                    DragItem::Data { data, types } => drag::DragItem::Data {
                        provider: Box::new(move |data_type| match &data {
                            SharedData::Fixed(d) => Some(d.as_bytes().to_vec()),
                            SharedData::Map(m) => m.get(data_type).map(|d| d.as_bytes().to_vec()),
                        }),
                        types,
                    },
                },
                image,
                move |result| {
                    if let Some(on_event_fn) = on_event_fn {
                        let js = tauri::api::ipc::format_callback(on_event_fn, &result)
                            .expect("unable to serialize DragResult");

                        let _ = window.eval(js.as_str());
                    }
                },
            )
            .map_err(Into::into),
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
