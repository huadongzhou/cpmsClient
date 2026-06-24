import { defineStore } from "pinia";
import { useLocalStorage } from "@vueuse/core";
import { createId } from "@/utils/id";
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
      id: createId(),
      createdAt: new Date().toISOString(),
    };

    errors.value = [error, ...errors.value].slice(0, 20);
    pushNotification({
      type: input.level === "error" ? "error" : "warning",
      title: input.level === "error" ? "系统运行异常" : "系统提醒",
      message: userFacingErrorMessage(input.message, input.level),
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
    const normalizedInput = normalizeNotificationInput(input);
    const notification: AppNotification = {
      id: createId(),
      type: normalizedInput.type ?? "info",
      title: normalizedInput.title,
      message: normalizedInput.message,
      createdAt: new Date().toISOString(),
      durationMs: normalizedInput.durationMs ?? 4500,
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

const NOTIFICATION_TEXT_MAP: Record<string, string> = {
  UNKNOWN_METHOD: "系统暂不支持该操作，请联系管理员！",
  BRIDGE_ERROR: "系统连接异常，请联系管理员！",
  COMMAND_ERROR: "操作未完成，请联系管理员！",
  EMIT_NOTIFICATION_ERROR: "通知发送失败，请联系管理员！",
};

function normalizeNotificationInput(input: PushNotificationInput): PushNotificationInput {
  const type = input.type ?? "info";
  const title = normalizeNotificationText(input.title, type, "title");
  const message = input.message
    ? normalizeNotificationText(input.message, type, "message")
    : undefined;

  return {
    ...input,
    title,
    message,
  };
}

function normalizeNotificationText(
  value: string,
  type: NonNullable<PushNotificationInput["type"]>,
  field: "title" | "message",
) {
  const text = value.trim();
  const mapped = NOTIFICATION_TEXT_MAP[text];
  if (mapped) {
    return mapped;
  }

  if (!text || isDeveloperFacingText(text)) {
    return field === "title" ? defaultNotificationTitle(type) : defaultNotificationMessage(type);
  }

  return text;
}

function userFacingErrorMessage(message: string, level: AppError["level"]) {
  const text = message.trim();
  if (!text || isDeveloperFacingText(text)) {
    return level === "error"
      ? "操作未完成，请联系管理员！"
      : "操作需要确认，请稍后重试或联系管理员。";
  }
  return text;
}

function isDeveloperFacingText(value: string) {
  return (
    /^client\./i.test(value) ||
    /\b[A-Z][A-Z0-9_]{2,}\b|UNKNOWN_METHOD|BRIDGE_ERROR|COMMAND_ERROR|HTTP status|status=\d{3}|reqwest|serde|JSON\.?stringify|payload|iframe|token|socket|undefined|null|^\s*[{[]/.test(
      value,
    )
  );
}

function defaultNotificationTitle(type: PushNotificationInput["type"]) {
  if (type === "success") return "操作成功";
  if (type === "error") return "操作未完成";
  if (type === "warning") return "操作提醒";
  return "系统通知";
}

function defaultNotificationMessage(type: PushNotificationInput["type"]) {
  if (type === "success") return "操作已完成。";
  if (type === "error") return "操作未完成，请联系管理员！";
  if (type === "warning") return "操作需要确认，请稍后重试或联系管理员。";
  return "系统已完成一项后台处理。";
}
