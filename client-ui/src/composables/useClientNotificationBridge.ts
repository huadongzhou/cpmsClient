import type { UnlistenFn } from "@tauri-apps/api/event";
import { listenClientNotificationEvent } from "@/api/tauri/events";
import { useAppStore } from "@/stores/app";

/** 监听客户端通知事件并写入前端通知队列，再由通知子窗口自动渲染展示。 */
export function useClientNotificationBridge() {
  const appStore = useAppStore();
  let unlisten: UnlistenFn | undefined;

  onMounted(async () => {
    unlisten = await listenClientNotificationEvent((payload) => {
      appStore.pushNotification({
        type: payload.type,
        title: payload.title,
        message: payload.message,
        durationMs: payload.durationMs,
      });
    });
  });

  onBeforeUnmount(() => {
    unlisten?.();
  });
}
