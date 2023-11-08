#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

use std::path::PathBuf;

mod platform_impl;

pub use platform_impl::start_drag;

/// Item to be dragged.
pub enum DragItem {
    /// A list of files to be dragged.
    ///
    /// The paths must be absolute.
    Files(Vec<PathBuf>),
}

/// An image definition.
pub enum Image {
    /// A path to a image.
    File(PathBuf),
    /// Raw bytes of the image.
    Raw(Vec<u8>),
}
