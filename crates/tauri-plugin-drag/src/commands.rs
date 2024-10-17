use std::{collections::HashMap, path::PathBuf, sync::mpsc::channel};

use serde::{ser::Serializer, Deserialize, Deserializer, Serialize};
use tauri::{command, ipc::Channel, AppHandle, Runtime, WebviewWindow};

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Drag(#[from] drag::Error),
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
    #[error(transparent)]
    Base64(#[from] base64::DecodeError),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

pub struct Base64Image(String);

impl<'de> Deserialize<'de> for Base64Image {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if let Some(data) = value.strip_prefix("data:image/png;base64,") {
            return Ok(Self(data.into()));
        }
        Err(serde::de::Error::custom(
            "expected an image/png base64 image string",
        ))
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum Image {
    Base64(Base64Image),
    Raw(drag::Image),
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum DragItem {
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
pub enum SharedData {
    Fixed(String),
    Map(HashMap<String, String>),
}

#[derive(Serialize, Clone)]
pub struct CallbackResult {
    result: drag::DragResult,
    #[serde(rename = "cursorPos")]
    cursor_pos: drag::CursorPosition,
}

#[command]
pub async fn start_drag<R: Runtime>(
    app: AppHandle<R>,
    window: WebviewWindow<R>,
    item: DragItem,
    image: Image,
    on_event: Channel<CallbackResult>,
) -> Result<()> {
    let (tx, rx) = channel();

    let image = match image {
        Image::Raw(r) => r,
        Image::Base64(b) => {
            use base64::Engine;
            drag::Image::Raw(base64::engine::general_purpose::STANDARD.decode(b.0)?)
        }
    };

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
                move |result, cursor_pos| {
                    let callback_result = CallbackResult { result, cursor_pos };
                    let _ = on_event.send(callback_result);
                },
                Default::default(),
            )
            .map_err(Into::into),
            Err(e) => Err(e.into()),
        };
        tx.send(r).unwrap();
    })?;

    rx.recv().unwrap()
}
