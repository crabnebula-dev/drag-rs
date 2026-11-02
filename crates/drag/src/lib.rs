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
//! - Use the `drag::start_drag` function. It takes a `&T: raw_window_handle::HasWindowHandle` type on macOS and Windows, and a `&gtk::ApplicationWindow` on Linux:
//!
//! - tao:
//!   ```rust,no_run
//!   let event_loop = tao::event_loop::EventLoop::new();
//!   let window = tao::window::WindowBuilder::new().build(&event_loop).unwrap();
//!
//!   let item = drag::DragItem::Files(vec![std::fs::canonicalize("./examples/icon.png").unwrap()]);
//!   let preview_icon = drag::Image::File("./examples/icon.png".into());
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
//!     |result, cursor_position| {
//!       println!("drag result: {result:?}");
//!     },
//!     drag::Options::default(),
//!   );
//!   ```
//!
//!   - wry:
//!   ```rust,no_run
//!   let event_loop = tao::event_loop::EventLoop::new();
//!   let window = tao::window::WindowBuilder::new().build(&event_loop).unwrap();
//!   let webview = wry::WebViewBuilder::new().build(&window).unwrap();
//!
//!   let item = drag::DragItem::Files(vec![std::fs::canonicalize("./examples/icon.png").unwrap()]);
//!   let preview_icon = drag::Image::File("./examples/icon.png".into());
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
//!     |result, cursor_position| {
//!       println!("drag result: {result:?}");
//!     },
//!     drag::Options::default(),
//!   );
//!   ```
//!
//!   - winit:
//!   ```rust,ignore
//!   let window = ...winit window;
//!
//!   let item = drag::DragItem::Files(vec![std::fs::canonicalize("./examples/icon.png").unwrap()]);
//!   let preview_icon = drag::Image::File("./examples/icon.png".into());
//!
//!   # #[cfg(not(target_os = "linux"))]
//!   let _ = drag::start_drag(&window, item, preview_icon, |result, cursor_position| {
//!     println!("drag result: {result:?}");
//!   }, Default::default());
//!   ```

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
    #[error("drag image not found")]
    ImageNotFound,
    #[cfg(target_os = "linux")]
    #[error("empty drag target list")]
    EmptyTargetList,
    #[error("failed to drop items")]
    FailedToDrop,
    #[error("failed to get cursor position")]
    FailedToGetCursorPosition,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum DragResult {
    /// The data was dropped on a target, which negotiated the enclosed
    /// [`DropOperation`] — what the source must now do with the original data
    /// (delete it after a move, leave it alone after a copy, …).
    ///
    /// A target that accepted the drop and then performed nothing yields an
    /// empty mask ([`DropOperation::is_empty`]); that is still a `Dropped`,
    /// not a `Cancel`.
    Dropped(DropOperation),
    Cancel,
}

/// The operation a drop target negotiated for a completed drop
/// ([`DragResult::Dropped`]).
///
/// A **mask**, not a single value, because two of the three platform values
/// are masks and none of them promises a single bit:
///
/// - Windows: `DoDragDrop`'s out `DROPEFFECT`. Its documentation is explicit
///   that callers must bit-test rather than compare ("Your application should
///   always mask values from the DROPEFFECT enumeration to ensure
///   compatibility with future implementations").
/// - macOS: `draggingSession:endedAtPoint:operation:`'s `NSDragOperation`, an
///   `NS_OPTIONS` bit mask.
/// - Linux (GTK): the drag context's `selected_action`, a `GdkDragAction` bit
///   mask.
///
/// The bits below are the crate's own portable set; each platform normalizes
/// its native value into them. Platform bits with no portable counterpart are
/// folded onto their closest neighbour, or dropped when they carry no
/// source-side obligation:
///
/// | platform bit | portable bit | why |
/// | --- | --- | --- |
/// | `DROPEFFECT_SCROLL` | — | target-scroll feedback, not an operation |
/// | `NSDragOperationDelete` | [`Self::MOVE`] | drag-to-Trash: the source must delete |
/// | `NSDragOperationGeneric` | [`Self::COPY`] | unspecified accept; the source keeps its data |
/// | `NSDragOperationPrivate` | — | receiver-internal; no source-side obligation |
/// | `GdkDragAction::ASK` / `PRIVATE` / `DEFAULT` | — | not a settled operation |
///
/// [`Self::NONE`] (no bits) means the target performed nothing — ask with
/// [`Self::is_empty`].
///
/// With the `serde` feature the mask is a plain `u32` on the wire, routed
/// through [`Self::from_bits_truncate`] on the way in, so no deserialized
/// mask can carry a bit the constructors cannot produce.
//
// Deliberately no `Default`: it could only be `NONE`, and `NONE` as an
// `Options::allowed_operations` is a drag no target can accept — not a value
// to arrive at by omitting a field. `Options` carries its own `Default`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(from = "u32", into = "u32"))]
pub struct DropOperation(u32);

impl DropOperation {
    /// No operation was performed.
    pub const NONE: Self = Self(0);
    /// The target took a copy; the source data is untouched.
    pub const COPY: Self = Self(1 << 0);
    /// The target took the data; the source should delete the original.
    pub const MOVE: Self = Self(1 << 1);
    /// The target created a link to the original data.
    pub const LINK: Self = Self(1 << 2);

    /// `true` if `self` and `other` share at least one bit.
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// `true` if no bits are set, i.e. the target performed nothing
    /// ([`Self::NONE`]).
    ///
    /// This is the one question equality answers correctly for a mask; every
    /// other question should go through [`Self::intersects`].
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The raw bits, for consumers that keep their own mapping table.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Rebuild a mask from [`Self::bits`], dropping anything that is not one
    /// of the defined bits.
    pub const fn from_bits_truncate(bits: u32) -> Self {
        Self(bits & (Self::COPY.0 | Self::MOVE.0 | Self::LINK.0))
    }
}

impl std::ops::BitOr for DropOperation {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for DropOperation {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Truncating, exactly like [`DropOperation::from_bits_truncate`]. This is
/// also the `serde` `Deserialize` path, which keeps a wire value like
/// `4294967295` from becoming a mask that intersects everything.
impl From<u32> for DropOperation {
    fn from(bits: u32) -> Self {
        Self::from_bits_truncate(bits)
    }
}

/// The raw bits, exactly like [`DropOperation::bits`]; the `serde`
/// `Serialize` path — the wire form stays a bare number.
impl From<DropOperation> for u32 {
    fn from(operation: DropOperation) -> Self {
        operation.bits()
    }
}

pub type DataProvider = Box<dyn Fn(&str) -> Option<Vec<u8>>>;

/// Item to be dragged.
pub enum DragItem {
    /// A list of files to be dragged.
    ///
    /// The paths must be absolute.
    Files(Vec<PathBuf>),
    /// Data to share with another app.
    ///
    /// - **Windows**: Not supported. Will result in a dummy drag operation of current folder that will be cancelled upon dropping.
    /// - **Linux (gtk)**: Not supported. Will result in a dummy drag operation that contains nothing to drop.
    Data {
        provider: DataProvider,
        types: Vec<String>,
    },
}

pub struct Options {
    // TODO: Fix typo in v3
    pub skip_animatation_on_cancel_or_failure: bool,
    /// The operations this source permits the drop target to negotiate.
    ///
    /// Handed to the platform's own permission channel — `DoDragDrop`'s
    /// `dwOKEffects` on Windows, the `NSDraggingSource`
    /// `draggingSession:sourceOperationMaskForDraggingContext:` return on
    /// macOS, the source's `GdkDragAction` on GTK. The operation the target
    /// actually performs comes back in [`DragResult::Dropped`].
    ///
    /// [`DropOperation::NONE`] permits nothing, so no target can accept the
    /// drop. The default is [`DropOperation::COPY`].
    pub allowed_operations: DropOperation,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            skip_animatation_on_cancel_or_failure: false,
            allowed_operations: DropOperation::COPY,
        }
    }
}

/// An image definition.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum Image {
    /// A path to a image.
    File(PathBuf),
    /// Raw bytes of the image.
    Raw(Vec<u8>),
}

/// Logical position of the cursor.
///
/// - **Windows**: Currently the win32 API for logical position reports physical position as well, due to the complicated nature of potential multiple monitor with different scaling there's no trivial solution to be incorporated.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct CursorPosition {
    pub x: i32,
    pub y: i32,
}
