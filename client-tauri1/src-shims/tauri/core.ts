import { convertFileSrc, invoke, transformCallback, type InvokeArgs } from "../../node_modules/@tauri-apps/api/tauri.js";

function isTauri() {
  if (typeof window === "undefined") {
    return false;
  }

  const tauriWindow = window as Window & {
    __TAURI__?: unknown;
    __TAURI_IPC__?: unknown;
  };

  return "__TAURI__" in tauriWindow || "__TAURI_IPC__" in tauriWindow;
}

export { convertFileSrc, invoke, isTauri, transformCallback };
export type { InvokeArgs };
