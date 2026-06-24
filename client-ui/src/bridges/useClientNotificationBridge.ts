import { CLIENT_NOTIFICATION_EVENT, onBridgeMessage } from "@/api/tauri/events";
import { useAppStore } from "@/stores/app";
import type { DesktopNotificationEventPayload } from "@/types/app/ipc";

/** 订阅客户端通知事件并写入前端通知队列，再由通知子窗口自动渲染展示。 */
export function useClientNotificationBridge() {
  const appStore = useAppStore();

  onBridgeMessage<DesktopNotificationEventPayload>(CLIENT_NOTIFICATION_EVENT, (payload) => {
    appStore.pushNotification({
      type: payload.type,
      title: payload.title,
      message: payload.message,
      durationMs: payload.durationMs,
    });
  });
}
