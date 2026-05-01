use std::{
    fs::read,
    io::Write,
    path::PathBuf,
    sync::{mpsc::channel, Arc, Mutex},
};

use base64::Engine;
use serde::{ser::Serializer, Deserialize, Serialize};
use tauri::{
    command, ipc::Channel, AppHandle, DragDropEvent, Manager, Runtime, Webview, WebviewEvent,
    Window, WindowEvent,
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

#[derive(Clone, Serialize)]
pub struct CallbackResult {
    result: drag::DragResult,
    #[serde(rename = "cursorPos")]
    cursor_pos: drag::CursorPosition,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum DragMode {
    #[default]
    Copy,
    Move,
}

impl From<DragMode> for drag::DragMode {
    fn from(value: DragMode) -> Self {
        match value {
            DragMode::Copy => Self::Copy,
            DragMode::Move => Self::Move,
        }
    }
}

#[derive(Default, Deserialize)]
pub struct DragOptions {
    #[serde(default)]
    mode: DragMode,
}

impl From<DragOptions> for drag::Options {
    fn from(options: DragOptions) -> Self {
        Self {
            skip_animatation_on_cancel_or_failure: true,
            mode: options.mode.into(),
        }
    }
}

#[command]
pub async fn on_drop<R: Runtime>(
    window: Window<R>,
    webview: Webview<R>,
    handler: Channel<serde_json::Value>,
) -> Result<()> {
    let webview_handler = handler.clone();

    window.on_window_event(move |event| {
        if let WindowEvent::DragDrop(DragDropEvent::Drop { paths, position: _ }) = event {
            handle_drop(paths, &handler);
        }
    });

    webview.on_webview_event(move |event| {
        if let WebviewEvent::DragDrop(DragDropEvent::Drop { paths, position: _ }) = event {
            handle_drop(paths, &webview_handler);
        }
    });
    Ok(())
}

fn handle_drop(paths: &[PathBuf], handler: &Channel<serde_json::Value>) {
    let Some(path) = paths.first() else {
        return;
    };

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
            let _ = handler.send(data);
        } else {
            eprintln!("failed to read {}", path.display());
        }
    }
}

#[command]
pub async fn drag_new_window<R: Runtime>(
    app: AppHandle<R>,
    window: Window<R>,
    image_base64: String,
    on_event: Channel<CallbackResult>,
    options: Option<DragOptions>,
) -> Result<()> {
    perform_drag(
        app,
        window,
        DragData::Data,
        image_base64,
        options.unwrap_or_default(),
        on_event,
        || {},
    )
}

#[command]
pub async fn drag_back<R: Runtime>(
    app: AppHandle<R>,
    window: Window<R>,
    data: serde_json::Value,
    image_base64: String,
    on_event: Channel<CallbackResult>,
    options: Option<DragOptions>,
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
        options.unwrap_or_default(),
        on_event,
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
    drag_options: DragOptions,
    on_event: Channel<CallbackResult>,
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
                        types: vec![window.config().identifier.clone()],
                    },
                },
                image,
                move |result, cursor_pos| {
                    let callback_result = CallbackResult { result, cursor_pos };
                    let _ = on_event.send(callback_result);

                    handler();
                },
                drag::Options {
                    skip_animatation_on_cancel_or_failure: true,
                    ..drag_options.into()
                },
            )
            .map_err(Into::into),
            Err(e) => Err(e.into()),
        };
        tx.send(r).unwrap();
    })?;

    rx.recv().unwrap()
}
