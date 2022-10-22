import { invoke } from '@tauri-apps/api'
import type { event } from '@tauri-apps/api'

export async function popup(msg: string) {
  await invoke('error_popup', { msg })
}
export async function runCmd<T = unknown>(cmd: string, options: { [key: string]: unknown } = {}) {
  return (await invoke(cmd, options).catch((msg) => {
    popup(msg)
    throw msg
  })) as T
}

export function extractUnlistener(futureUnlistener: Promise<event.UnlistenFn>) {
  return async () => {
    const unlisten = await futureUnlistener
    unlisten()
  }
}
