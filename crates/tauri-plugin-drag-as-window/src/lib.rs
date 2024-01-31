// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{
    fs::read,
    io::Write,
    path::PathBuf,
    sync::{mpsc::channel, Arc, Mutex},
};

use base64::Engine;
use serde::{ser::Serializer, Serialize};
use tauri::{
    api::ipc::CallbackFn,
    command,
    plugin::{Builder, TauriPlugin},
    AppHandle, FileDropEvent, Manager, Runtime, Window, WindowEvent,
};

type Result<T> = std::result::Result<T, Error>;

const FILE_PREFIX: &str = "qow3ciuh";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Drag(#[from] drag::Error),
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
    #[error(transparent)]
    Base64(#[from] base64::DecodeError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("invalid base64, expected image/png format")]
    InvalidBase64,
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

#[derive(Serialize)]
struct CallbackResult {
    result: drag::DragResult,
    #[serde(rename = "cursorPos")]
    cursor_pos: drag::CursorPosition,
}

#[command]
async fn on_drop<R: Runtime>(window: Window<R>, handler: CallbackFn) -> Result<()> {
    let window_ = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::FileDrop(FileDropEvent::Dropped(paths)) = event {
            let path = paths.first().unwrap();
            if path
                .file_name()
                .and_then(|f| f.to_str())
                .map(|f| f.starts_with(FILE_PREFIX))
                .unwrap_or_default()
            {
                if let Some(data) = read(path)
                    .ok()
                    .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
                {
                    let js = tauri::api::ipc::format_callback(handler, &data)
                        .expect("unable to serialize DragResult");

                    let _ = window_.eval(js.as_str());
                } else {
                    eprintln!("failed to read {}", path.display());
                }
            }
        }
    });
    Ok(())
}

#[command]
async fn drag_new_window<R: Runtime>(
    app: AppHandle<R>,
    window: Window<R>,
    image_base64: String,
    on_event_fn: Option<CallbackFn>,
) -> Result<()> {
    perform_drag(
        app,
        window,
        DragData::Data,
        image_base64,
        on_event_fn,
        || {},
    )
}

#[command]
async fn drag_back<R: Runtime>(
    app: AppHandle<R>,
    window: Window<R>,
    data: serde_json::Value,
    image_base64: String,
    on_event_fn: Option<CallbackFn>,
) -> Result<()> {
    let data = serde_json::to_vec(&data)?;

    let mut file = tempfile::Builder::new().prefix(FILE_PREFIX).tempfile()?;
    file.write_all(&data)?;
    file.flush()?;
    let path = file.path().to_path_buf();

    let file = Arc::new(Mutex::new(Some(file)));

    perform_drag(
        app,
        window,
        DragData::Path(path),
        image_base64,
        on_event_fn,
        move || {
            let file_ = file.clone();
            // wait a litle to delete the file
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(3));
                file_.lock().unwrap().take();
            });
        },
    )
}

enum DragData {
    Path(PathBuf),
    Data,
}

fn perform_drag<R: Runtime, F: Fn() + Send + Sync + 'static>(
    app: AppHandle<R>,
    window: Window<R>,
    data: DragData,
    image_base64: String,
    on_event_fn: Option<CallbackFn>,
    handler: F,
) -> Result<()> {
    let (tx, rx) = channel();

    let image = drag::Image::Raw(
        base64::engine::general_purpose::STANDARD.decode(
            image_base64
                .strip_prefix("data:image/png;base64,")
                .ok_or(Error::InvalidBase64)?,
        )?,
    );

    app.run_on_main_thread(move || {
        #[cfg(target_os = "linux")]
        let raw_window = window.gtk_window();
        #[cfg(not(target_os = "linux"))]
        let raw_window = tauri::Result::Ok(window.clone());

        let r = match raw_window {
            Ok(w) => drag::start_drag(
                &w,
                match data {
                    DragData::Path(p) => drag::DragItem::Files(vec![p]),
                    DragData::Data => drag::DragItem::Data {
                        provider: Box::new(|_type| Some(Vec::new())),
                        types: vec![window.config().tauri.bundle.identifier.clone()],
                    },
                },
                image,
                move |result, cursor_pos| {
                    if let Some(on_event_fn) = on_event_fn {
                        let callback_result = CallbackResult { result, cursor_pos };
                        let js = tauri::api::ipc::format_callback(on_event_fn, &callback_result)
                            .expect("unable to serialize CallbackResult");

                        let _ = window.eval(js.as_str());
                    }

                    handler();
                },
                drag::Options {
                    skip_animatation_on_cancel_or_failure: true,
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
    #[allow(unused_mut)]
    let mut builder = Builder::new("drag-as-window");

    #[cfg(feature = "global-js")]
    {
        builder = builder.js_init_script(include_str!("./api-iife.js").to_string());
    }

    builder
        .invoke_handler(tauri::generate_handler![
            drag_new_window,
            drag_back,
            on_drop
        ])
        .build()
}
