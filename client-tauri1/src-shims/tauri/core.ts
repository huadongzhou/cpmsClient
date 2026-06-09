import { convertFileSrc, invoke, transformCallback, type InvokeArgs } from "../../node_modules/@tauri-apps/api/tauri.js";

function isTauri() {
  return typeof window !== "undefined" && "__TAURI__" in window;
}

export { convertFileSrc, invoke, isTauri, transformCallback };
export type { InvokeArgs };
