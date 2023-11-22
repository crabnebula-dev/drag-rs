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
