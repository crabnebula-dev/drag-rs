// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//!Start a drag operation out of a window on macOS, Windows and Linux (via GTK).
//!
//! Tested for [tao](https://github.com/tauri-apps/tao) (latest),
//! [winit](https://github.com/rust-windowing/winit) (latest),
//! [wry](https://github.com/tauri-apps/wry) (v0.24) and
//! [tauri](https://github.com/tauri-apps/tauri) (v1) windows.
//!
//! Due to the GTK-based implementation, winit currently cannot leverage this crate on Linux yet.
//!
//! - Add the `drag` dependency:
//!
//! `$ cargo add drag`
//!
//! - Use the `drag::start_drag` function. It takes a `&T: raw_window_handle::HasRawWindowHandle` type on macOS and Windows, and a `&gtk::ApplicationWindow` on Linux:
//!
//! - tao:
//!   ```rust,no_run
//!   let event_loop = tao::event_loop::EventLoop::new();
//!   let window = tao::window::WindowBuilder::new().build(&event_loop).unwrap();
//!
//!   let item = drag::DragItem::Files(vec![std::fs::canonicalize("./examples/icon.png").unwrap()]);
//!   let preview_icon = drag::Image::File("../../icon.png".into());
//!
//!   drag::start_drag(
//!     #[cfg(target_os = "linux")]
//!     {
//!       use tao::platform::unix::WindowExtUnix;
//!       window.gtk_window()
//!     },
//!     #[cfg(not(target_os = "linux"))]
//!     &window,
//!     item,
//!     preview_icon,
//!   );
//!   ```
//!
//!   - wry:
//!   ```rust,no_run
//!   let event_loop = wry::application::event_loop::EventLoop::new();
//!   let window = wry::application::window::WindowBuilder::new().build(&event_loop).unwrap();
//!   let webview = wry::webview::WebViewBuilder::new(window).unwrap().build().unwrap();
//!
//!   let item = drag::DragItem::Files(vec![std::fs::canonicalize("./examples/icon.png").unwrap()]);
//!   let preview_icon = drag::Image::File("../../icon.png".into());
//!
//!   drag::start_drag(
//!     #[cfg(target_os = "linux")]
//!     {
//!       use wry::application::platform::unix::WindowExtUnix;
//!       webview.window().gtk_window()
//!     },
//!     #[cfg(not(target_os = "linux"))]
//!     &webview.window(),
//!     item,
//!     preview_icon,
//!   );
//!   ```
//!
//!   - winit:
//!   ```rust,no_run
//!   let event_loop = winit::event_loop::EventLoop::new().unwrap();
//!   let window = winit::window::WindowBuilder::new().build(&event_loop).unwrap();
//!
//!   let item = drag::DragItem::Files(vec![std::fs::canonicalize("./examples/icon.png").unwrap()]);
//!   let preview_icon = drag::Image::File("../../icon.png".into());
//!
//!   # #[cfg(not(target_os = "linux"))]
//!   let _ = drag::start_drag(&window, item, preview_icon);
//!   ```

#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

use std::path::PathBuf;

mod platform_impl;

pub use platform_impl::start_drag;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[cfg(windows)]
    #[error("{0}")]
    WindowsError(#[from] windows::core::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("unsupported window handle")]
    UnsupportedWindowHandle,
    #[error("failed to start drag")]
    FailedToStartDrag,
    #[cfg(target_os = "linux")]
    #[error("empty drag target list")]
    EmptyTargetList,
}

/// Item to be dragged.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum DragItem {
    /// A list of files to be dragged.
    ///
    /// The paths must be absolute.
    Files(Vec<PathBuf>),
}

/// An image definition.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum Image {
    /// A path to a image.
    File(PathBuf),
    /// Raw bytes of the image.
    Raw(Vec<u8>),
}
