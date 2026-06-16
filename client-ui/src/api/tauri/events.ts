import { isTauri } from "@tauri-apps/api/core";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ClientEventPayload, DesktopNotificationEventPayload } from "@/types/app/ipc";
import type { ClientLogEventPayload } from "@/types/app/log";
import type {
  ClientIframeStatePayload,
  ClientSocketStatePayload,
  ClientTodoTaskPayload,
} from "@/types/app/runtime";

export const VIEW_TO_CLIENT_EVENT = "cpms:view-to-client";
export const CLIENT_TO_VIEW_EVENT = "cpms:client-to-view";
export const CLIENT_NOTIFICATION_EVENT = "cpms:desktop-notification";
export const CLIENT_IFRAME_EVENT = "cpms:client-iframe";
export const CLIENT_TODO_TASK_EVENT = "cpms:client-todo-task";
export const CLIENT_LOG_EVENT = "cpms:client-log";
export const CLIENT_SOCKET_EVENT = "cpms:client-socket";
export const HUB_SYSTEM_STATE_EVENT = "cpms:hub-system-state";
export const HUB_NETWORK_CHANGED_EVENT = "cpms:hub-network-changed";

/** 生成标准信封的消息 id。 */
export function createMessageId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `msg-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

/** 视图端向客户端发送事件（标准信封 `{ id, type, payload, time }`）。 */
export async function emitViewEvent(type: string, payload?: unknown) {
  if (!isTauri()) {
    return;
  }

  await emit(VIEW_TO_CLIENT_EVENT, {
    id: createMessageId(),
    type,
    payload,
    time: Date.now(),
  } satisfies ClientEventPayload);
}

/** 监听客户端向视图端回推事件。 */
export async function listenClientEvent(
  handler: (payload: ClientEventPayload) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) {
    return () => undefined;
  }

  return listen<ClientEventPayload>(CLIENT_TO_VIEW_EVENT, (event) => {
    handler(event.payload);
  });
}

/** 监听客户端通知事件，收到后由前端渲染并驱动通知子窗口展示。 */
export async function listenClientNotificationEvent(
  handler: (payload: DesktopNotificationEventPayload) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) {
    return () => undefined;
  }

  return listen<DesktopNotificationEventPayload>(CLIENT_NOTIFICATION_EVENT, (event) => {
    handler(event.payload);
  });
}

/** 监听客户端 iframe 容器状态事件。 */
export async function listenClientIframeEvent(
  handler: (payload: ClientIframeStatePayload) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) {
    return () => undefined;
  }

  return listen<ClientIframeStatePayload>(CLIENT_IFRAME_EVENT, (event) => {
    handler(event.payload);
  });
}

/** 监听客户端 Todo 任务事件。 */
export async function listenClientTodoTaskEvent(
  handler: (payload: ClientTodoTaskPayload) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) {
    return () => undefined;
  }

  return listen<ClientTodoTaskPayload>(CLIENT_TODO_TASK_EVENT, (event) => {
    handler(event.payload);
  });
}

/** 监听客户端本地 socket 连接状态事件（地址/端口/连接状态）。 */
export async function listenClientSocketEvent(
  handler: (payload: ClientSocketStatePayload) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) {
    return () => undefined;
  }

  return listen<ClientSocketStatePayload>(CLIENT_SOCKET_EVENT, (event) => {
    handler(event.payload);
  });
}

/** 监听客户端（Rust 侧）日志事件，供调试抽屉日志面板展示。 */
export async function listenClientLogEvent(
  handler: (payload: ClientLogEventPayload) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) {
    return () => undefined;
  }

  return listen<ClientLogEventPayload>(CLIENT_LOG_EVENT, (event) => {
    handler(event.payload);
  });
}

/** 监听 Hub 客户端领域事件。 */
export async function listenHubEvent<T>(
  eventName: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) {
    return () => undefined;
  }

  return listen<T>(eventName, (event) => {
    handler(event.payload);
  });
}
