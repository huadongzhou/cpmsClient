import {
  emit,
  listen,
  once,
  type Event,
  type EventCallback,
  type UnlistenFn,
} from "../../node_modules/@tauri-apps/api/event.js";
import { WebviewWindow } from "../../node_modules/@tauri-apps/api/window.js";

async function emitTo(label: string, event: string, payload?: unknown) {
  const target = WebviewWindow.getByLabel(label);

  if (target) {
    await target.emit(event, payload);
    return;
  }

  await emit(event, payload);
}

export { emit, emitTo, listen, once };
export type { Event, EventCallback, UnlistenFn };
