// Copyright 2020-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

#![cfg(not(target_os = "linux"))]

use drag::{start_drag, DragItem, Image};
use winit::{
    dpi::LogicalSize,
    event::{DeviceEvent, ElementState, Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(400., 100.))
        .with_title("Drag Example")
        .build(&event_loop)
        .unwrap();

    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Wait);

            match event {
                Event::NewEvents(StartCause::Init) => println!("Wry application started!"),
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => elwt.exit(),

                Event::DeviceEvent {
                    device_id: _,
                    event:
                        DeviceEvent::Button {
                            button: 0,
                            state: ElementState::Pressed,
                        },
                } => {
                    start_drag(
                        &window,
                        DragItem::Files(
                            vec![std::fs::canonicalize("./examples/icon.png").unwrap()],
                        ),
                        Image::Raw(include_bytes!("../../icon.png").to_vec()),
                        // Image::File("./examples/icon.png".into()),
                    )
                    .unwrap();
                }

                _ => (),
            }
        })
        .unwrap();
}
