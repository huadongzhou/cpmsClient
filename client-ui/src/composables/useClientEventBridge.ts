import type { UnlistenFn } from "@tauri-apps/api/event";
import { listenClientEvent } from "@/api/tauri/events";
import { useAppStore } from "@/stores/app";
import type { ClientEventPayload } from "@/types/app/ipc";
import type { PushNotificationInput } from "@/types/app/notification";

/** 监听客户端回推事件，并以轻量通知形式反馈到视图端。 */
export function useClientEventBridge() {
  const appStore = useAppStore();
  let unlisten: UnlistenFn | undefined;

  onMounted(async () => {
    unlisten = await listenClientEvent((payload) => {
      const notification = toClientEventNotification(payload);
      if (notification) {
        appStore.pushNotification(notification);
      }
    });
  });

  onBeforeUnmount(() => {
    unlisten?.();
  });
}

function toClientEventNotification(payload: ClientEventPayload): PushNotificationInput | null {
  if (payload.type.startsWith("client.socket_task.")) {
    return null;
  }

  if (isInternalClientEvent(payload)) {
    return null;
  }

  if (payload.type.includes("fail") || payload.type.includes("error")) {
    return {
      type: "error",
      title: eventFailureTitle(payload.type),
      message: "操作未完成，请联系管理员！",
      durationMs: 6000,
    };
  }

  const commandResult = commandResultNotification(payload);
  if (commandResult) {
    return commandResult;
  }

  const message = readablePayloadMessage(payload.payload);
  if (!message) {
    return null;
  }

  return {
    type: "info",
    title: eventInfoTitle(payload.type),
    message,
    durationMs: 3500,
  };
}

function isInternalClientEvent(payload: ClientEventPayload) {
  if (
    payload.type === "client.iframe_payload.request" ||
    payload.type === "client.iframe_payload.reported" ||
    payload.type === "client.iframe.refresh"
  ) {
    return true;
  }

  if (!payload.type.endsWith(".result")) {
    return false;
  }

  const result = payload.payload as { success?: unknown; ok?: unknown } | undefined;
  return result?.success !== false && result?.ok !== false;
}

function commandResultNotification(payload: ClientEventPayload): PushNotificationInput | null {
  if (!payload.type.endsWith(".result")) {
    return null;
  }

  const result = payload.payload as
    | { success?: unknown; ok?: unknown; message?: unknown; code?: unknown }
    | undefined;
  const failed = result?.success === false || result?.ok === false;
  if (!failed) {
    return null;
  }

  return {
    type: "error",
    title: eventFailureTitle(payload.type),
    message: "操作未完成，请联系管理员！",
    durationMs: 6000,
  };
}

function readablePayloadMessage(payload: unknown) {
  if (!payload || typeof payload !== "object") {
    return typeof payload === "string" && payload.trim() ? payload.trim() : undefined;
  }

  const message = (payload as { message?: unknown }).message;
  return typeof message === "string" && message.trim() ? message.trim() : undefined;
}

function eventFailureTitle(type: string) {
  if (type.includes("device") || type.includes("printer")) return "设备操作失败";
  if (type.includes("job")) return "作业处理失败";
  if (type.includes("window")) return "窗口操作失败";
  if (type.includes("socket")) return "连接处理失败";
  if (type.includes("auth") || type.includes("token")) return "登录状态异常";
  return "系统处理失败";
}

function eventInfoTitle(type: string) {
  if (type.includes("device") || type.includes("printer")) return "设备通知";
  if (type.includes("job")) return "作业通知";
  if (type.includes("socket")) return "连接通知";
  return "系统通知";
}
