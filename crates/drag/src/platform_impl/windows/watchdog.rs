// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Recovery for a `DoDragDrop` that stops responding.
//!
//! `DoDragDrop` runs its own modal message loop on the calling thread and holds
//! the mouse capture until the drag ends. When it fails to exit, the caller's
//! thread never returns, which for a UI thread means the whole application goes
//! with it: no repaint, no input, no timers, while the process still reports as
//! responding because it *is* pumping messages, just not the application's.
//!
//! ## Why this cannot live in `IDropSource::QueryContinueDrag`
//!
//! That is the obvious place to put a timeout and it does not work. In the stuck
//! state `DoDragDrop` is spinning inside `PeekMessage` on an empty queue and is
//! not calling `QueryContinueDrag` at all, so a deadline checked there is never
//! evaluated. Measured on a real occurrence: 24 of 24 instruction-pointer samples
//! of the wedged thread landed inside `NtUserPeekMessage`, with an 84% kernel /
//! 2.2% user split, which is a spin on an empty queue rather than a wait on a
//! slow drop target.
//!
//! The recovery therefore has to come from outside the drag. Posting a
//! `VK_ESCAPE` pair to the window holding the capture gives the modal loop a
//! message to consume, at which point it calls `QueryContinueDrag` again and the
//! existing escape branch returns `DRAGDROP_S_CANCEL` through the normal path.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::{
        Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE, VK_LBUTTON, VK_RBUTTON},
        WindowsAndMessaging::{
            GetClassNameW, GetGUIThreadInfo, PostMessageW, GUITHREADINFO, WM_KEYDOWN, WM_KEYUP,
        },
    },
};

/// The window class OLE uses for the drag's modal loop. The watchdog will not
/// post to anything else, so it can never disturb an unrelated capture.
const OLE_DRAG_CLASS: &str = "CLIPBRDWNDCLASS";

/// How often the state is sampled. Small relative to any sane timeout, and the
/// work per tick is three cheap syscalls.
const POLL: Duration = Duration::from_millis(250);

/// Watches the dragging thread and cancels a drag that is holding the mouse
/// capture with no button pressed for longer than `timeout`.
///
/// `alive` is cleared by the caller as soon as `DoDragDrop` returns, so a normal
/// drag of any length stops this thread without it ever acting.
pub(crate) fn watch(thread_id: u32, alive: Arc<AtomicBool>, timeout: Duration) {
    // Measured from when the drag first looks stuck, NOT from when it started,
    // because a legitimate slow `IDropTarget::Drop` also holds the capture with
    // no button down. Erring long costs the user a few seconds; erring short
    // cancels a real drop and loses them the file.
    let mut stuck_since: Option<Instant> = None;

    while alive.load(Ordering::SeqCst) {
        std::thread::sleep(POLL);
        if !alive.load(Ordering::SeqCst) {
            return;
        }

        let capture = match stuck_capture(thread_id) {
            Some(hwnd) => hwnd,
            None => {
                stuck_since = None;
                continue;
            }
        };

        match stuck_since {
            None => stuck_since = Some(Instant::now()),
            Some(since) if since.elapsed() >= timeout => {
                log::warn!(
                    "drag: DoDragDrop held the mouse capture for {:?} with no button down; \
                     cancelling it with VK_ESCAPE",
                    since.elapsed()
                );
                cancel(capture);
                return;
            }
            Some(_) => {}
        }
    }
}

/// `Some(hwnd)` when the dragging thread holds the capture through OLE's drag
/// window and no mouse button is down, which together mean the drag has outlived
/// the gesture that started it.
fn stuck_capture(thread_id: u32) -> Option<HWND> {
    if mouse_button_down() {
        return None;
    }

    let mut info = GUITHREADINFO {
        cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: `info` is a correctly sized, zeroed GUITHREADINFO.
    unsafe { GetGUIThreadInfo(thread_id, &mut info) }.ok()?;

    let capture = info.hwndCapture;
    if capture.0 == 0 {
        return None;
    }

    let mut class = [0u16; 64];
    // SAFETY: `capture` came from GetGUIThreadInfo; the buffer bounds the write.
    let len = unsafe { GetClassNameW(capture, &mut class) };
    if len <= 0 {
        return None;
    }

    if String::from_utf16_lossy(&class[..len as usize]) == OLE_DRAG_CLASS {
        Some(capture)
    } else {
        None
    }
}

fn mouse_button_down() -> bool {
    // SAFETY: GetAsyncKeyState takes a virtual key code and has no preconditions.
    unsafe {
        (GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16 & 0x8000) != 0
            || (GetAsyncKeyState(VK_RBUTTON.0 as i32) as u16 & 0x8000) != 0
    }
}

/// Ends the drag the way the user would, through `QueryContinueDrag`'s own
/// escape branch.
///
/// Deliberately *not* `ReleaseCapture`: taking the capture away from OLE leaves
/// it believing a drag is still in progress, which trades one bad state for a
/// worse one.
fn cancel(capture: HWND) {
    // SAFETY: posting is asynchronous and does not dereference anything; an
    // invalid window simply fails.
    unsafe {
        let _ = PostMessageW(capture, WM_KEYDOWN, WPARAM(VK_ESCAPE.0 as usize), LPARAM(0));
        let _ = PostMessageW(capture, WM_KEYUP, WPARAM(VK_ESCAPE.0 as usize), LPARAM(0));
    }
}
