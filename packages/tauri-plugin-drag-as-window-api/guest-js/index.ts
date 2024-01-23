import { invoke, transformCallback } from "@tauri-apps/api/tauri";
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
  result: "Dropped" | "CreateWindow";
  cursorPos: CursorPosition;
}

/**
 * Listen to a DOM element being dropped in this window. The data that was associated with the drop event is passed as argument to the handler closure.
 * 
 * @param handler closure that is called when a DOM element gets dropped in this window
 */
export async function onElementDrop(handler: (data: any) => void) {
  await invoke("plugin:drag-as-window|on_drop", {
    handler: transformCallback(handler)
  });
}

/**
 * Starts a drag operation of the given DOM element. You can associate any JSON data with the operation to retrieve later.
 *
 * ```typescript
 * import { startDrag } from "@crabnebula/tauri-plugin-drag-as-window";
 *
 * await startDrag('#my-target', { id: 0 });
 * ```
 *
 * @param options the drag options containing data and preview image
 * @param onEvent on drag event handler
 */
export async function startDrag(
  el: string | HTMLElement,
  data: any,
  onEvent?: (result: CallbackPayload) => void
): Promise<void> {
  const element = typeof el === 'string' ? document.querySelector(el) : el
  if (element === null) {
    throw new Error(`Element with selector "${el}" not found`)
  }
  const canvas = await html2canvas(element as HTMLElement, { logging: false });

  await invoke("plugin:drag-as-window|start_drag", {
    data,
    imageBase64: canvas.toDataURL('image/png'),
    onEventFn: onEvent ? transformCallback((payload: RawCallbackPayload) => {
      onEvent({
        result: payload.result === 'Dropped' ? 'Dropped' : 'CreateWindow',
        cursorPos: payload.cursorPos,
      })
    }) : null,
  });
}
