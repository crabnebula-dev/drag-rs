import { invoke, transformCallback } from "@tauri-apps/api/core";
import html2canvas from "html2canvas";

type DragResult = "Dropped" | "Cancelled";

/**
 * Logical position of the cursor.
 */
export interface CursorPosition {
  x: Number;
  y: Number;
}

interface RawCallbackPayload {
  result: DragResult;
  cursorPos: CursorPosition;
}

export interface CallbackPayload {
  cursorPos: CursorPosition;
}

/**
 * Listen to a DOM element being dropped in this window. The data that was associated with the drop event is passed as argument to the handler closure.
 *
 * @param handler closure that is called when a DOM element gets dropped in this window
 */
export async function onElementDrop(handler: (data: any) => void) {
  await invoke("plugin:drag-as-window|on_drop", {
    handler: transformCallback(handler),
  });
}

/**
 * Starts a drag operation of the given DOM element.
 *
 * The intention of this drag operation is to drag the DOM element to any region in the desktop to create a new window.
 * The window creation is your responsibility, use the onDrop hook to achieve it.
 *
 * ```typescript
 * import { dragBack } from "@crabnebula/tauri-plugin-drag-as-window";
 * import { WebviewWindow } from "@tauri-apps/api/window";
 *
 * const el = document.querySelector("#my-target")
 * await dragBack(el, (payload) => {
 *   new WebviewWindow('label', {
 *     url: 'new-window',
 *     width: el.clientWidth,
 *     height: el.clientHeight + 20,
 *     x: payload.cursorPos.x,
 *     y: payload.cursorPos.y,
 *   });
 * });
 * ```
 *
 * @param el the element or selector to drag
 * @param onDrop on drop handler, used to create the window
 */
export async function dragAsWindow(
  el: string | HTMLElement,
  onDrop?: (result: CallbackPayload) => void
): Promise<void> {
  const element = typeof el === "string" ? document.querySelector(el) : el;
  if (element === null) {
    throw new Error(`Element with selector "${el}" not found`);
  }
  const canvas = await html2canvas(element as HTMLElement, { logging: false });

  await invoke("plugin:drag-as-window|drag_new_window", {
    imageBase64: canvas.toDataURL("image/png"),
    onEventFn: onDrop
      ? transformCallback((payload: RawCallbackPayload) => {
          onDrop({
            cursorPos: payload.cursorPos,
          });
        })
      : null,
  });
}

/**
 * Starts a drag operation of the given DOM element. You can associate any JSON data with the operation to retrieve later.
 *
 * The intention of this drag operation is to get the DOM element back to an app window.
 *
 * ```typescript
 * import { dragBack } from "@crabnebula/tauri-plugin-drag-as-window";
 *
 * await dragBack('#my-target', { id: 0 });
 * ```
 *
 * @param el the element or selector to drag
 * @param data the data to associate with the drag operation
 * @param onEvent on drop event handler
 */
export async function dragBack(
  el: string | HTMLElement,
  data: any,
  onEvent?: (result: CallbackPayload) => void
): Promise<void> {
  const element = typeof el === "string" ? document.querySelector(el) : el;
  if (element === null) {
    throw new Error(`Element with selector "${el}" not found`);
  }
  const canvas = await html2canvas(element as HTMLElement, { logging: false });

  await invoke("plugin:drag-as-window|drag_back", {
    data,
    imageBase64: canvas.toDataURL("image/png"),
    onEventFn: onEvent
      ? transformCallback((payload: RawCallbackPayload) => {
          onEvent({
            cursorPos: payload.cursorPos,
          });
        })
      : null,
  });
}
