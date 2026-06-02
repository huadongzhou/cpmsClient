import { watch } from "vue";
import { storeToRefs } from "pinia";
import { showDesktopNotification } from "@/api/desktop/notification";
import { useAppStore } from "@/stores/app";

/** 监听主窗口通知队列，并把新通知转发到 Tauri 桌面子窗口。 */
export function useDesktopNotificationBridge() {
  const appStore = useAppStore();
  const { notifications } = storeToRefs(appStore);
  const deliveredIds = new Set<string>();

  watch(
    notifications,
    (items) => {
      for (const item of items) {
        if (deliveredIds.has(item.id)) {
          continue;
        }

        deliveredIds.add(item.id);
        void showDesktopNotification(item).catch((error) => {
          console.warn("Failed to show desktop notification window", error);
        });
      }
    },
    { deep: true },
  );
}
