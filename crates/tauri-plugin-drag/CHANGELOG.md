# Changelog

## \[0.3.0]

- [`dd3f087`](https://github.com/crabnebula-dev/drag-rs/commit/dd3f0873ae2406968d412d9dfdc1c79a5ed5533e)([#25](https://github.com/crabnebula-dev/drag-rs/pull/25)) Changed the onEvent callback payload from `DragResult` to `{ result: DragResult, cursorPos: CursorPosition }`.

### Dependencies

- Upgraded to `drag@0.4.0`

## \[0.2.0]

- [`1449076`](https://github.com/crabnebula-dev/drag-rs/commit/14490764de8ff50969a3f2299d204e44e091752e) The `startDrag` function now takes an argument for a callback function for the operation result (either `Dragged` or `Cancelled`).
- [`f58ed78`](https://github.com/crabnebula-dev/drag-rs/commit/f58ed7838abe1fe5b23c4e3aa92df28e77564345) The `startDrag` function can now be used to drag arbitrary data strings on macOS (e.g. Final Cut Pro XMLs).

### Dependencies

- Upgraded to `drag@0.3.0`

## \[0.1.0]

- [`644cfa2`](https://github.com/crabnebula-dev/drag-rs/commit/644cfa28b09bee9c3de396bdcc1dc801a26d65bc) Initial release.
