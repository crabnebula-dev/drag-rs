// Copyright 2020-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use base64::Engine;
use drag::{start_drag, CursorPosition, DragItem, DragResult, Image};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::path::PathBuf;
use tao::dpi::LogicalPosition;
use tao::event_loop::{EventLoopBuilder, EventLoopWindowTarget};
use tao::window::WindowId;
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopProxy},
    window::{Window, WindowBuilder},
};
use wry::http::Request;
use wry::WebView;
use wry::{DragDropEvent, WebViewBuilder};

enum UserEvent {
    StartDragOut(WindowId, String, Option<drag::Image>),
    StartDragBack(WindowId, String, Option<drag::Image>),
    PopulateElement(WindowId, String),
    RemoveElement(WindowId, String),
    CloseWindow(WindowId),
    NewWindow(CursorPosition, String),
}

#[derive(Debug, Deserialize)]
struct Payload {
    action: String,
    item: String,
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
    let event_loop = EventLoopBuilder::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let mut webviews = HashMap::new();

    let (window, webview) = create_main_window(
        String::from("Drag Example - First Window"),
        &event_loop,
        proxy.clone(),
        None,
    )?;
    webviews.insert(window.id(), (window, webview));

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
            Event::UserEvent(UserEvent::NewWindow(cursor_pos, item)) => {
                let (window, webview) = create_new_window(
                    format!("Window {}", webviews.len() + 1),
                    event_loop,
                    proxy.clone(),
                    Some(cursor_pos),
                    item,
                )
                .unwrap();
                webviews.insert(window.id(), (window, webview));
            }
            Event::UserEvent(UserEvent::CloseWindow(id)) => {
                webviews.remove(&id);
                if webviews.is_empty() {
                    *control_flow = ControlFlow::Exit
                }
            }
            Event::UserEvent(UserEvent::PopulateElement(id, item)) => {
                let (_window, webview) = &webviews.get(&id).unwrap();
                let mut js = "window.appendElement('".to_owned();
                js.push_str(&item);
                js.push_str("')");
                let _ = webview.evaluate_script(&js);
            }
            Event::UserEvent(UserEvent::RemoveElement(id, item)) => {
                let (_window, webview) = &webviews.get(&id).unwrap();
                let mut js = "window.removeElement('".to_owned();
                js.push_str(&item);
                js.push_str("')");
                let _ = webview.evaluate_script(&js);
            }

            Event::UserEvent(UserEvent::StartDragOut(id, item, icon)) => {
                let (window, _webview) = &webviews.get(&id).unwrap();
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
                        use tao::platform::unix::WindowExtUnix;
                        window.gtk_window()
                    },
                    #[cfg(not(target_os = "linux"))]
                    &window,
                    DragItem::Data {
                        provider: Box::new(|_| Some(Vec::new())),
                        types: vec!["com.app.myapp.v2".into()],
                    },
                    icon,
                    move |result: DragResult, cursor_pos: CursorPosition| {
                        println!(
                            "--> Drop Result: [{:?}], Cursor Pos:[{:?}]",
                            result, cursor_pos
                        );
                        let _ = proxy.send_event(UserEvent::NewWindow(cursor_pos, item.clone()));
                        let _ = proxy.send_event(UserEvent::RemoveElement(id, item.clone()));
                    },
                    Default::default(),
                )
                .unwrap();
            }
            Event::UserEvent(UserEvent::StartDragBack(id, item, icon)) => {
                let (window, _webview) = &webviews.get(&id).unwrap();
                let proxy = proxy.clone();

                let icon = match icon {
                    Some(i) => i,
                    None => {
                        Image::Raw(include_bytes!("../../icon.png").to_vec())
                        // Image::File("./examples/icon.png".into())
                    }
                };
                let mut paths = Vec::new();
                let dummy_path = "./examples/wry-dragout/dummy/".to_owned() + &item;
                paths.push(PathBuf::from(dummy_path).canonicalize().unwrap());
                start_drag(
                    #[cfg(target_os = "linux")]
                    {
                        use tao::platform::unix::WindowExtUnix;
                        window.gtk_window()
                    },
                    #[cfg(not(target_os = "linux"))]
                    &window,
                    DragItem::Files(paths),
                    icon,
                    move |result: DragResult, cursor_pos: CursorPosition| {
                        println!(
                            "--> Drop Result: [{:?}], Cursor Pos:[{:?}]",
                            result, cursor_pos
                        );
                        let _ = proxy.send_event(UserEvent::CloseWindow(id));
                    },
                    Default::default(),
                )
                .unwrap();
            }

            _ => (),
        }
    })
}

fn create_main_window(
    title: String,
    event_loop: &EventLoopWindowTarget<UserEvent>,
    proxy: EventLoopProxy<UserEvent>,
    position: Option<CursorPosition>,
) -> wry::Result<(Window, WebView)> {
    const HTML: &str = r#"
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta http-equiv="X-UA-Compatible" content="IE=edge" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <script src="https://cdnjs.cloudflare.com/ajax/libs/html2canvas/1.4.1/html2canvas.min.js" integrity="sha512-BNaRQnYJYiPSqHHDb58B0yaPfCu+Wgds8Gp/gU33kqBtgNS4tSPHuGibyoeqMV/TJlSKda6FXzoEyYGjTe+vXA==" crossorigin="anonymous" referrerpolicy="no-referrer"></script>
    <style>
    .drag-item {
        border: 2px solid black;
        border-radius: 3px;
        width: 100%;
        height: 80px;
        display: flex;
        align-items: center;
        justify-content: center;
        -webkit-user-select: none;
      }
    </style>
  </head>

  <body>
    <div id="drop-zone" class="drag-zone"></div>
    <script type="text/javascript">
      appendItem('drag-1');
      appendItem('drag-2');
      appendItem('drag-3');
      function appendItem(id) {
        const dropZoneEl = document.getElementById('drop-zone');
        const dragEl = document.createElement('div');
        dragEl.setAttribute('draggable', 'true');
        dragEl.id = id;
        dragEl.className = 'drag-item';
        dragEl.innerText = `Drag me ${id}`;
        dropZoneEl.appendChild(dragEl);
        dragEl.ondragstart = dragHandler;
      }
      async function dragHandler(event) {
        event.preventDefault();

        const el = event.target;
        const canvas = await html2canvas(el, { logging: false });
        const iconDataURL = canvas.toDataURL('image/png');

        const dragItem = {
              data: '',
              types: ['new.window.type'],
            };

        const payload = {
            action: 'start-drag',
            item: el.id,
            iconDataURL,
            };
    
        console.debug({payload});

        window.ipc.postMessage(JSON.stringify(payload));
      }
      window.removeElement = (id) => {
        document.getElementById(`${id}`).remove();
        console.log(`${id}`);
      }
      window.appendElement = (id) => {
        appendItem(`${id}`);
        console.log(`${id}`);
      }      
    </script>
  </body>
</html>
  "#;

    let mut window_builder = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(400., 300.))
        .with_title(title);

    if let Some(position) = position {
        window_builder = window_builder.with_position(LogicalPosition::new(position.x, position.y));
    }

    let window = window_builder.build(event_loop).unwrap();
    let window_id = window.id();

    let drag_drop_proxy = proxy.clone();
    let handler = move |req: Request<String>| {
        if let Ok(payload) = serde_json::from_str::<Payload>(req.body()) {
            if payload.action == "start-drag" {
                let icon = drag::Image::Raw(
                    base64::engine::general_purpose::STANDARD
                        .decode(payload.icon_data_url.0)
                        .unwrap(),
                );
                let _ =
                    proxy.send_event(UserEvent::StartDragOut(window_id, payload.item, Some(icon)));
            }
        } else if req.body() == "close" {
            let _ = proxy.send_event(UserEvent::CloseWindow(window_id));
        }
    };
    let drag_drop_handler = move |req: DragDropEvent| {
        if let DragDropEvent::Drop { paths, position: _ } = req {
            for f in paths {
                let _ = drag_drop_proxy.send_event(UserEvent::PopulateElement(
                    window_id,
                    dunce::canonicalize(f)
                        .unwrap()
                        .file_name()
                        .unwrap()
                        .to_os_string()
                        .into_string()
                        .unwrap(),
                ));
            }
        }
        // need to return true to prevent triggering OS drop behavior
        true
    };
    let webview = WebViewBuilder::new()
        .with_html(HTML)
        .with_ipc_handler(handler)
        .with_accept_first_mouse(true)
        .with_drag_drop_handler(drag_drop_handler)
        .build(&window)?;
    Ok((window, webview))
}

fn create_new_window(
    title: String,
    event_loop: &EventLoopWindowTarget<UserEvent>,
    proxy: EventLoopProxy<UserEvent>,
    position: Option<CursorPosition>,
    id: String,
) -> wry::Result<(Window, WebView)> {
    let html: String = r#"
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta http-equiv="X-UA-Compatible" content="IE=edge" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <script src="https://cdnjs.cloudflare.com/ajax/libs/html2canvas/1.4.1/html2canvas.min.js" integrity="sha512-BNaRQnYJYiPSqHHDb58B0yaPfCu+Wgds8Gp/gU33kqBtgNS4tSPHuGibyoeqMV/TJlSKda6FXzoEyYGjTe+vXA==" crossorigin="anonymous" referrerpolicy="no-referrer"></script>
    <style>
      .drag-item {
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
    <div id="drop-zone" class="drag-zone"></div>
    <script type="text/javascript">
      appendItem('"#.to_owned() + &id + r#"');
      function appendItem(id) {
        const dropZoneEl = document.getElementById('drop-zone');
        const dragEl = document.createElement('div');
        dragEl.setAttribute('draggable', 'true');
        dragEl.id = id;
        dragEl.className = 'drag-item';
        dragEl.innerText = `Drag me ${id}`;
        dropZoneEl.appendChild(dragEl);
        dragEl.ondragstart = dragHandler;
      }
      async function dragHandler(event) {
        event.preventDefault();

        const el = event.target;
        const canvas = await html2canvas(el, { logging: false });
        const iconDataURL = canvas.toDataURL('image/png');

        const dragItem = {
              data: '',
              types: ['new.window.type'],
            };

            const payload = {
                action: 'start-drag',
                item: el.id,
                iconDataURL,
              };
      
              console.debug({payload});
      
              window.ipc.postMessage(JSON.stringify(payload));
      }      
    </script>
  </body>
</html>
  "#;

    let mut window_builder = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(400., 100.))
        .with_title(title);

    if let Some(position) = position {
        window_builder = window_builder.with_position(LogicalPosition::new(position.x, position.y));
    }

    let window = window_builder.build(event_loop).unwrap();
    let window_id = window.id();

    let handler = move |request: Request<String>| {
        if let Ok(payload) = serde_json::from_str::<Payload>(request.body()) {
            if payload.action == "start-drag" {
                let icon = drag::Image::Raw(
                    base64::engine::general_purpose::STANDARD
                        .decode(payload.icon_data_url.0)
                        .unwrap(),
                );
                let _ = proxy.send_event(UserEvent::StartDragBack(
                    window_id,
                    payload.item,
                    Some(icon),
                ));
            }
        } else if request.body() == "close" {
            let _ = proxy.send_event(UserEvent::CloseWindow(window_id));
        }
    };

    let webview = WebViewBuilder::new()
        .with_html(html)
        .with_ipc_handler(handler)
        .with_accept_first_mouse(true)
        .build(&window)?;

    Ok((window, webview))
}
