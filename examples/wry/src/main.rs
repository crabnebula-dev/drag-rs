// Copyright 2020-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use drag::{start_drag, DragItem, DragResult, Image, CursorPosition};
use wry::application::{
    dpi::LogicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
use wry::webview::WebViewBuilder;

enum UserEvent {
    StartDrag,
}

fn main() -> wry::Result<()> {
    let event_loop = EventLoop::with_user_event();
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(400., 100.))
        .with_title("Drag Example")
        .build(&event_loop)
        .unwrap();

    const HTML: &str = r#"
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta http-equiv="X-UA-Compatible" content="IE=edge" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
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
      document.getElementById('drag').ondragstart = (event) => {
        event.preventDefault();
        window.ipc.postMessage('startDrag');
      };
    </script>
  </body>
</html>
  "#;

    let proxy = event_loop.create_proxy();
    let handler = move |_w: &Window, req: String| {
        if req == "startDrag" {
            let _ = proxy.send_event(UserEvent::StartDrag);
        }
    };

    let webview = WebViewBuilder::new(window)?
        .with_html(HTML)?
        .with_ipc_handler(handler)
        .with_accept_first_mouse(true)
        .build()?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => println!("Wry application started!"),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,

            Event::UserEvent(e) => match e {
                UserEvent::StartDrag => {
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
                        Image::Raw(include_bytes!("../../icon.png").to_vec()),
                        // Image::File("./examples/icon.png".into()),
                        |result: DragResult, _cursor_pos: CursorPosition| {
                            println!("--> Drop Result: [{:?}]", result);
                        },
                    )
                    .unwrap();
                }
            },
            _ => (),
        }
    })
}
