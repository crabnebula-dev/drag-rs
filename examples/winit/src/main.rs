// Copyright 2020-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

#[cfg(target_os = "linux")]
fn main() {
    eprintln!("This example is not supported on Linux");
}

#[cfg(not(target_os = "linux"))]
fn main() {
    use drag::{start_drag, CursorPosition, DragItem, DragResult, Image};
    use std::collections::HashMap;

    use winit::{
        application::ApplicationHandler,
        dpi::LogicalSize,
        event::{DeviceEvent, ElementState, WindowEvent},
        event_loop::EventLoop,
        window::{Window, WindowId},
    };

    let event_loop = EventLoop::new().unwrap();

    struct Application {
        windows: HashMap<WindowId, Window>,
    }

    impl ApplicationHandler for Application {
        fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
            let window = event_loop
                .create_window(
                    Window::default_attributes()
                        .with_inner_size(LogicalSize::new(400., 100.))
                        .with_title("Drag Example"),
                )
                .unwrap();

            self.windows.insert(window.id(), window);
        }

        fn window_event(
            &mut self,
            event_loop: &winit::event_loop::ActiveEventLoop,
            _window_id: winit::window::WindowId,
            event: winit::event::WindowEvent,
        ) {
            if let WindowEvent::CloseRequested = event {
                event_loop.exit()
            }
        }

        fn device_event(
            &mut self,
            _event_loop: &winit::event_loop::ActiveEventLoop,
            _device_id: winit::event::DeviceId,
            event: DeviceEvent,
        ) {
            if let DeviceEvent::Button {
                button: 0,
                state: ElementState::Pressed,
            } = event
            {
                start_drag(
                    &self.windows.values().next().unwrap(),
                    DragItem::Files(vec![std::fs::canonicalize("./examples/icon.png").unwrap()]),
                    Image::Raw(include_bytes!("../../icon.png").to_vec()),
                    // Image::File("./examples/icon.png".into()),
                    |result: DragResult, cursor_pos: CursorPosition| {
                        println!(
                            "--> Drop Result: [{:?}], Cursor Pos:[{:?}]",
                            result, cursor_pos
                        );
                    },
                    Default::default(),
                )
                .unwrap();
            }
        }
    }

    let mut app = Application {
        windows: Default::default(),
    };
    event_loop.run_app(&mut app).unwrap();
}
