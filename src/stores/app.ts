import { defineStore } from "pinia";
import { useLocalStorage } from "@vueuse/core";
import type { AppError } from "@/types/app/error";
import type { ClientConfig } from "@/types/app/config";
import type { AppNotification, PushNotificationInput } from "@/types/app/notification";

export const useAppStore = defineStore("app", () => {
  const config = useLocalStorage<ClientConfig>("cpmsClient:config", {
    cpmsBaseUrl: "http://localhost:8080",
    localServiceUrl: "http://127.0.0.1:18080",
    logLevel: "info",
  });
  const errors = ref<AppError[]>([]);
  const notifications = ref<AppNotification[]>([]);
  const latestError = computed(() => errors.value[0]);

  /** 写入应用错误，并自动生成一条错误通知。 */
  function pushError(input: Omit<AppError, "id" | "createdAt">) {
    const error: AppError = {
      ...input,
      id: createErrorId(),
      createdAt: new Date().toISOString(),
    };

    errors.value = [error, ...errors.value].slice(0, 20);
    pushNotification({
      type: input.level === "error" ? "error" : "warning",
      title: input.message,
      message: `${input.source} / ${input.code}`,
      durationMs: 6000,
    });
    return error;
  }

  /** 移除指定应用错误。 */
  function clearError(id: string) {
    errors.value = errors.value.filter((error) => error.id !== id);
  }

  /** 清空应用错误队列。 */
  function clearErrors() {
    errors.value = [];
  }

  /** 写入应用通知队列，最多保留最近 5 条通知。 */
  function pushNotification(input: PushNotificationInput) {
    const notification: AppNotification = {
      id: createErrorId(),
      type: input.type ?? "info",
      title: input.title,
      message: input.message,
      createdAt: new Date().toISOString(),
      durationMs: input.durationMs ?? 4500,
    };

    notifications.value = [notification, ...notifications.value].slice(0, 5);
    return notification;
  }

  /** 移除指定应用通知。 */
  function removeNotification(id: string) {
    notifications.value = notifications.value.filter((notification) => notification.id !== id);
  }

  /** 清空应用通知队列。 */
  function clearNotifications() {
    notifications.value = [];
  }

  return {
    config,
    errors,
    notifications,
    latestError,
    pushError,
    clearError,
    clearErrors,
    pushNotification,
    removeNotification,
    clearNotifications,
  };
});

function createErrorId() {
  if (window.crypto?.randomUUID) {
    return window.crypto.randomUUID();
  }

  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}
