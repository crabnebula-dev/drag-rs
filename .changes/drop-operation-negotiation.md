---
"drag": major
"tauri-plugin-drag": major
"tauri-plugin-drag-as-window": major
"@crabnebula/tauri-plugin-drag": major
"@crabnebula/tauri-plugin-drag-as-window": major
---

Carry the drag-and-drop negotiation in both directions:

- `DragResult::Dropped` now carries a `DropOperation` — a portable bit mask
  (`COPY | MOVE | LINK`) reporting the operation the drop target actually
  negotiated, normalized from `DoDragDrop`'s out `DROPEFFECT` (Windows),
  the `draggingSession:endedAtPoint:operation:` argument (macOS), and the
  drag context's `selected_action` (GTK). With the `serde` feature the wire
  form of `Dropped` changes from `"Dropped"` to `{"Dropped": <mask>}`; the
  TypeScript `DragResult` types are updated to match (they previously also
  never matched the `Cancel` arm).
- `Options.mode: DragMode` is replaced by `Options.allowed_operations:
  DropOperation`, the mask of operations the source permits the target to
  negotiate, feeding `dwOKEffects` (Windows), the
  `sourceOperationMaskForDraggingContext:` return (macOS) and the source's
  `GdkDragAction` (GTK). `Options::default()` still permits exactly a copy.
  An empty mask now makes the drag undroppable instead of silently offering
  a copy. The plugins' `mode` option accepts `"link"` in addition to
  `"copy"` and `"move"`.
