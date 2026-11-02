import { invoke, Channel } from "@tauri-apps/api/core";

export type DragItem =
  | string[]
  | { data: string | Record<string, string>; types: string[] };

/**
 * The result of a drag operation.
 *
 * `Dropped` carries the operation the drop target negotiated, as a bit mask:
 * copy = 1, move = 2, link = 4. A mask of 0 means the target accepted the
 * drop but performed nothing. (This matches the Rust `DragResult` wire
 * format; the previous `"Dropped" | "Cancelled"` type never matched the
 * `Cancel` arm.)
 */
export type DragResult = { Dropped: number } | "Cancel";

/**
 * Logical position of the cursor.
 */
export interface CursorPosition {
  x: Number;
  y: Number;
}

export interface Options {
  item: DragItem;
  icon: string;
  mode?: "copy" | "move" | "link";
}

export interface CallbackPayload {
  result: DragResult;
  cursorPos: CursorPosition;
}

/**
 * Starts a drag operation. Can either send a list of files or data to another app.
 *
 * ```typescript
 * import { startDrag } from "@crabnebula/tauri-plugin-drag";
 *
 * // drag a file:
 * startDrag({
 *  item: ["/path/to/file.png"],
 *  icon: "/path/to/preview.png"
 * });
 *
 * // drag Final Cut Pro data:
 * startDrag({
 *   item: {
 *    // alternatively, you can pass an object mapping each type to its own XML format
 *     data: '<fcpxml version="1.10">...</fcpxml>',
 *     types: [
 *       "com.apple.finalcutpro.xml.v1-10",
 *       "com.apple.finalcutpro.xml.v1-9",
 *       "com.apple.finalcutpro.xml"
 *     ]
 *   }
 * });
 * ```
 *
 * @param options the drag options containing data and preview image
 * @param onEvent on drag event handler
 */
export async function startDrag(
  options: Options,
  onEvent?: (result: CallbackPayload) => void
): Promise<void> {
  const onEventChannel = new Channel<CallbackPayload>();
  if (onEvent) {
    onEventChannel.onmessage = onEvent;
  }
  await invoke("plugin:drag|start_drag", {
    item: options.item,
    image: options.icon,
    options: {
      mode: options.mode,
    },
    onEvent: onEventChannel,
  });
}
