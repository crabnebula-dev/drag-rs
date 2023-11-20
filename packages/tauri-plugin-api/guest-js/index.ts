import { invoke } from '@tauri-apps/api/tauri'

export type DragItem = string[]

export interface Options {
  item: DragItem
  icon: string
}

export async function startDrag(options: Options): Promise<void> {
  await invoke('plugin:drag|start_drag', { item: options.item, image: options.icon })
}
