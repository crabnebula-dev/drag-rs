// Copyright 2020-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use base64::Engine;
use drag::{start_drag, CursorPosition, DragItem, DragResult, Image};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use wry::application::event_loop::EventLoopWindowTarget;
use wry::application::window::WindowId;
use wry::webview::WebViewBuilder;
use wry::{
    application::{
        dpi::LogicalSize,
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopProxy},
        window::{Window, WindowBuilder},
    },
    webview::WebView,
};

enum UserEvent {
    StartDrag(WindowId, Option<drag::Image>),
    CloseWindow(WindowId),
    NewTitle(WindowId, String),
    NewWindow,
}

#[derive(Debug, Deserialize)]
struct Payload {
    action: String,
    #[serde(rename = "iconDataURL")]
    icon_data_url: Base64Image,
}

#[derive(Debug)]
struct Base64Image(String);

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

fn main() -> wry::Result<()> {
    let event_loop = EventLoop::with_user_event();
    let proxy = event_loop.create_proxy();

    let mut webviews = HashMap::new();

    let webview = create_new_window(
        String::from("Drag Example - First Window"),
        &event_loop,
        proxy.clone(),
    )?;
    webviews.insert(webview.window().id(), webview);

    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
                ..
            } => {
                webviews.remove(&window_id);
                if webviews.is_empty() {
                    *control_flow = ControlFlow::Exit
                }
            }
            Event::UserEvent(UserEvent::NewWindow) => {
                let webview = create_new_window(
                    format!("Window {}", webviews.len() + 1),
                    event_loop,
                    proxy.clone(),
                )
                .unwrap();
                webviews.insert(webview.window().id(), webview);
            }
            Event::UserEvent(UserEvent::CloseWindow(id)) => {
                webviews.remove(&id);
                if webviews.is_empty() {
                    *control_flow = ControlFlow::Exit
                }
            }
            Event::UserEvent(UserEvent::NewTitle(id, title)) => {
                webviews.get(&id).unwrap().window().set_title(&title);
            }
            Event::UserEvent(UserEvent::StartDrag(id, icon)) => {
                let webview = &webviews.get(&id).unwrap();
                let proxy = proxy.clone();

                let icon = match icon {
                    Some(i) => i,
                    None => {
                        Image::Raw(include_bytes!("../../icon.png").to_vec())
                        // Image::File("./examples/icon.png".into())
                    }
                };

                start_drag(
                    #[cfg(target_os = "linux")]
                    {
                        use wry::application::platform::unix::WindowExtUnix;
                        webview.window().gtk_window()
                    },
                    #[cfg(not(target_os = "linux"))]
                    &webview.window(),
                    DragItem::Files(vec![
                        std::fs::canonicalize("./examples/icon.png").unwrap(),
                        std::fs::canonicalize("./examples/icon.bmp").unwrap(),
                    ]),
                    icon,
                    move |result: DragResult, cursor_pos: CursorPosition| {
                        println!(
                            "--> Drop Result: [{:?}], Cursor Pos:[{:?}]",
                            result, cursor_pos
                        );
                        let _ = proxy.send_event(UserEvent::NewWindow);
                    },
                )
                .unwrap();
            }
            _ => (),
        }
    })
}

fn create_new_window(
    title: String,
    event_loop: &EventLoopWindowTarget<UserEvent>,
    proxy: EventLoopProxy<UserEvent>,
) -> wry::Result<WebView> {
    const HTML: &str = r#"
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta http-equiv="X-UA-Compatible" content="IE=edge" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <script src="https://cdnjs.cloudflare.com/ajax/libs/html2canvas/1.4.1/html2canvas.min.js" integrity="sha512-BNaRQnYJYiPSqHHDb58B0yaPfCu+Wgds8Gp/gU33kqBtgNS4tSPHuGibyoeqMV/TJlSKda6FXzoEyYGjTe+vXA==" crossorigin="anonymous" referrerpolicy="no-referrer"></script>
    <style>
      #drag {
        border:2px solid black;
        border-radius:3px;
        width: 100%;
        height: calc(100vh - 20px);
        display: flex;
        align-items: center;
        justify-content: center;
        -webkit-user-select: none;
      }
    </style>
  </head>

  <body>
    <div draggable="true" id="drag">
      Drag me
    </div>
    <script type="text/javascript">
      document.getElementById('drag').ondragstart = async (event) => {
        event.preventDefault();

        /* get dom snapshot */
        const dragger = document.getElementById('drag');
        const canvas = await html2canvas(dragger, { logging: false });
        const iconDataURL = canvas.toDataURL('image/png');

        const payload = {
          action: 'start-drag',
          iconDataURL,
        };

        console.debug({payload});

        window.ipc.postMessage(JSON.stringify(payload));
      };
    </script>
  </body>
</html>
  "#;

    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(400., 100.))
        .with_title(title)
        .build(event_loop)?;
    let window_id = window.id();

    let handler = move |_w: &Window, req: String| {
        if let Ok(payload) = serde_json::from_str::<Payload>(&req) {
            if payload.action == "start-drag" {
                let icon = drag::Image::Raw(
                    base64::engine::general_purpose::STANDARD
                        .decode(payload.icon_data_url.0)
                        .unwrap(),
                );
                let _ = proxy.send_event(UserEvent::StartDrag(window_id, Some(icon)));
            }
        } else {
            match req.as_str() {
                "new-window" => {
                    let _ = proxy.send_event(UserEvent::NewWindow);
                }
                "close" => {
                    let _ = proxy.send_event(UserEvent::CloseWindow(window_id));
                }
                _ if req.starts_with("change-title") => {
                    let title = req.replace("change-title:", "");
                    let _ = proxy.send_event(UserEvent::NewTitle(window_id, title));
                }
                _ => {}
            }
        }
    };

    let webview = WebViewBuilder::new(window)?
        .with_html(HTML)?
        .with_ipc_handler(handler)
        .with_accept_first_mouse(true)
        .build()?;

    Ok(webview)
}
