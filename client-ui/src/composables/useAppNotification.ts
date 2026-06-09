import { storeToRefs } from "pinia";
import { useAppStore } from "@/stores/app";
import type { PushNotificationInput } from "@/types/app/notification";

/** 提供应用通知队列的读写方法，主窗口会桥接到桌面级通知子窗口。 */
export function useAppNotification() {
  const appStore = useAppStore();
  const { notifications } = storeToRefs(appStore);

  /** 写入一条应用通知，并返回完整通知对象。 */
  function notify(input: PushNotificationInput) {
    return appStore.pushNotification(input);
  }

  return {
    notifications,
    notify,
    removeNotification: appStore.removeNotification,
    clearNotifications: appStore.clearNotifications,
  };
}
