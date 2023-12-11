import { invoke, transformCallback } from '@tauri-apps/api/tauri'

export type DragItem = string[]

export type DragResult = 'Dropped' | 'Cancelled'

export interface Options {
  item: DragItem
  icon: string
}

export async function startDrag(options: Options, onEvent?: (result: DragResult) => void): Promise<void> {
  await invoke('plugin:drag|start_drag', {
    item: options.item,
    image: options.icon,
    onEventFn: onEvent ? transformCallback(onEvent) : null
  })
}
