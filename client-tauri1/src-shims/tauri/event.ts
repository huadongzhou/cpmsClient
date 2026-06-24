import {
  emit,
  listen,
  once,
  type Event,
  type EventCallback,
  type UnlistenFn,
} from "../../node_modules/@tauri-apps/api/event.js";

async function emitTo(label: string, event: string, payload?: unknown) {
  // Tauri 1 has no real emitTo API. A global emit reaches window-scoped
  // listeners, while WebviewWindow.emit uses the label as the source window.
  void label;
  await emit(event, payload);
}

export { emit, emitTo, listen, once };
export type { Event, EventCallback, UnlistenFn };
