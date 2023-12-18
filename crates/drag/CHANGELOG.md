# Changelog

## \[0.3.0]

- [`f58ed78`](https://github.com/crabnebula-dev/drag-rs/commit/f58ed7838abe1fe5b23c4e3aa92df28e77564345) Added `DragItem::Drag` variant (supported on macOS) to drag a buffer (e.g. Final Cut Pro XMLs).
- [`1449076`](https://github.com/crabnebula-dev/drag-rs/commit/14490764de8ff50969a3f2299d204e44e091752e) The `start_drag` function now takes a closure for the operation result (either `DragResult::Dropped` or `DragResult::Cancelled`).

## \[0.2.0]

- [`644cfa2`](https://github.com/crabnebula-dev/drag-rs/commit/644cfa28b09bee9c3de396bdcc1dc801a26d65bc) Initial release.
