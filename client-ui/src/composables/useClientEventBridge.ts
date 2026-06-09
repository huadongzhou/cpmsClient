import type { UnlistenFn } from "@tauri-apps/api/event";
import { listenClientEvent } from "@/api/tauri/events";
import { useAppStore } from "@/stores/app";

/** 监听客户端回推事件，并以轻量通知形式反馈到视图端。 */
export function useClientEventBridge() {
  const appStore = useAppStore();
  let unlisten: UnlistenFn | undefined;

  onMounted(async () => {
    unlisten = await listenClientEvent((payload) => {
      appStore.pushNotification({
        type: "info",
        title: `客户端事件：${payload.name}`,
        message:
          typeof payload.payload === "string" ? payload.payload : JSON.stringify(payload.payload),
        durationMs: 3500,
      });
    });
  });

  onBeforeUnmount(() => {
    unlisten?.();
  });
}
