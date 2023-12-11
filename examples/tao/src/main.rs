// Copyright 2020-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use drag::{start_drag, DragItem, DropResult, Image};
use tao::{
    dpi::LogicalSize,
    event::{ElementState, Event, MouseButton, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(400., 100.))
        .with_title("Drag Example")
        .build(&event_loop)
        .unwrap();

    event_loop.run(move |event, _target, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => println!("Wry application started!"),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }

            Event::WindowEvent {
                window_id: _,
                event:
                    WindowEvent::MouseInput {
                        device_id: _,
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    },
                ..
            } => {
                start_drag(
                    #[cfg(target_os = "linux")]
                    {
                        use tao::platform::unix::WindowExtUnix;
                        window.gtk_window()
                    },
                    #[cfg(not(target_os = "linux"))]
                    &window,
                    DragItem::Files(vec![std::fs::canonicalize("./examples/icon.png").unwrap()]),
                    Image::Raw(include_bytes!("../../icon.png").to_vec()),
                    // Image::File("./examples/icon.png".into()),
                    |result: DropResult| {
                        println!("--> Drop Result: [{:?}]", result);
                    },
                )
                .unwrap();
            }

            _ => (),
        }
    });
}
