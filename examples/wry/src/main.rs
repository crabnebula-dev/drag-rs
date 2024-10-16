// Copyright 2020-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use drag::{start_drag, CursorPosition, DragItem, DragResult, Image};
use tao::{
    dpi::LogicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use wry::{http::Request, WebViewBuilder};

enum UserEvent {
    StartDrag,
}

fn main() -> wry::Result<()> {
    let event_loop = EventLoopBuilder::with_user_event().build();
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
    let handler = move |req: Request<String>| {
        if req.body() == "startDrag" {
            let _ = proxy.send_event(UserEvent::StartDrag);
        }
    };

    let _webview = WebViewBuilder::new()
        .with_html(HTML)
        .with_ipc_handler(handler)
        .with_accept_first_mouse(true)
        .build(&window)?;

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
                            use tao::platform::unix::WindowExtUnix;
                            window.gtk_window()
                        },
                        #[cfg(not(target_os = "linux"))]
                        &window,
                        DragItem::Files(vec![
                            std::fs::canonicalize("./examples/icon.png").unwrap(),
                            std::fs::canonicalize("./examples/icon.bmp").unwrap(),
                        ]),
                        Image::Raw(include_bytes!("../../icon.png").to_vec()),
                        // Image::File("./examples/icon.png".into()),
                        |result: DragResult, _cursor_pos: CursorPosition| {
                            println!("--> Drop Result: [{:?}]", result);
                        },
                        Default::default(),
                    )
                    .unwrap();
                }
            },
            _ => (),
        }
    })
}
