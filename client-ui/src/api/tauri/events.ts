import { isTauri } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { type EventBusKey, useEventBus } from "@vueuse/core";
import type { ClientEventPayload } from "@/types/app/ipc";
import { createId } from "@/utils/id";

/**
 * 统一通信常量（已去掉 cpms: 前缀）。
 * 这些字符串值同时约束三处，三者必须一致：
 * - client-tauri1 Rust 侧 emit/listen 的事件名；
 * - 本视图端（client-ui）；
 * - hub-web iframe 端。
 */
export const VIEW_TO_CLIENT_EVENT = "view-to-client";
export const CLIENT_TO_VIEW_EVENT = "client-to-view";
export const CLIENT_NOTIFICATION_EVENT = "desktop-notification";
export const CLIENT_IFRAME_EVENT = "client-iframe";
export const CLIENT_TODO_TASK_EVENT = "client-todo-task";
export const CLIENT_LOG_EVENT = "client-log";
export const CLIENT_SOCKET_EVENT = "client-socket";

/**
 * iframe ↔ 客户端父窗口 postMessage 通道的统一消息类型（去 cpms: 前缀）。
 * RPC 不再用 hub-client:* 信封：`type` 直接是真实方法名，有 `id` 即请求/响应、无 `id` 即单向推送。
 */
export const IFRAME_TOKEN_EVENT = "token";
export const IFRAME_SERVER_ADDRESS_EVENT = "serverAddress";
export const IFRAME_DIRECT_DEVICE_EVENT = "deviceId";
export const IFRAME_PLATFORM_EVENT = "platform";
export const IFRAME_REFRESH_EVENT = "refresh";

/**
 * 视图端 ↔ 客户端的业务事件类型（client-to-view / view-to-client 信封内的 `type` 值）。
 * 字符串值与 client-tauri1 Rust 侧发出的值一致，此处仅做语义化命名、值不变。
 */
export const CLIENT_EVENT = {
  // iframe 登录态同步
  IFRAME_PAYLOAD_REQUEST: "client.iframe_payload.request",
  IFRAME_PAYLOAD_REPORTED: "client.iframe_payload.reported",
  IFRAME_REFRESH: "client.iframe.refresh",
  // 窗口控制（视图端 → 客户端）
  WINDOW_PIN: "client.window.pin",
  WINDOW_UNPIN: "client.window.unpin",
  WINDOW_MINIMIZE: "client.window.minimize",
  WINDOW_FULLSCREEN: "client.window.fullscreen",
  WINDOW_EXIT_FULLSCREEN: "client.window.exit-fullscreen",
  WINDOW_CLOSE: "client.window.close",
} as const;

/** 本地 socket 打印任务事件前缀（`client.socket_task.*`）。 */
export const CLIENT_SOCKET_TASK_PREFIX = "client.socket_task.";

/* ────────────────────────────── 统一消息实体 ────────────────────────────── */

/** 消息环境：客户端（含 Tauri 后端）或 iframe 业务视图。 */
export type MessageEnv = "client" | "iframe";
/** 日志类型：请求型 / 普通日志。 */
export type MessageLogType = "request" | "log";
/** 消息状态：成功 / 失败（用于请求-响应）。 */
export type MessageStatus = "ok" | "error";

/**
 * 跨端统一消息信封。所有通道（Tauri 事件、iframe postMessage）共用同一结构。
 * - `payload` 承载消息体；请求/响应的 method/args/result/error 等放入 payload；
 * - `id` 为可选关联字段，仅请求-响应（取 token、RPC 调用）使用；
 * - `log` 存在时由客户端自动落盘，前缀 `[env/logType]`。
 */
export interface BridgeMessage<P = unknown> {
  env: MessageEnv;
  type: string;
  payload?: P;
  time: number;
  id?: string;
  log?: string;
  logType?: MessageLogType;
  status?: MessageStatus;
}

/* ───────────────────────────── 统一事件总线 ───────────────────────────── */

/**
 * 单一事件总线：所有来源（Tauri listen、iframe postMessage）汇入此处，
 * 各业务文件只订阅总线、不再各自 addEventListener / listen，从而：
 * - 保持「一个 onMessage」；
 * - 多文件通过 useEventBus 推送/订阅；
 * - 不再有 removeEventListener / unlisten。
 */
export const BRIDGE_BUS_KEY: EventBusKey<BridgeMessage> = Symbol("cpms-bridge-bus");

export function useBridgeBus() {
  return useEventBus(BRIDGE_BUS_KEY);
}

/** 向总线推送一条统一消息。 */
export function emitBridgeMessage(message: BridgeMessage) {
  useBridgeBus().emit(message);
}

/** 订阅总线上指定 `type` 的消息（统一出口，替代各文件单独监听）。返回退订函数（通常无需调用）。 */
export function onBridgeMessage<P = unknown>(
  type: string,
  handler: (payload: P, message: BridgeMessage<P>) => void,
) {
  return useBridgeBus().on((message) => {
    if (message.type === type) {
      handler(message.payload as P, message as BridgeMessage<P>);
    }
  });
}

/** 生成标准信封的消息 id。 */
export function createMessageId(): string {
  return createId();
}

/** 视图端向客户端发送事件（标准信封 `{ id, type, payload, time }`）。 */
export async function emitViewEvent(type: string, payload?: unknown) {
  if (!isTauri()) {
    return;
  }

  await emit(VIEW_TO_CLIENT_EVENT, {
    env: "client",
    id: createMessageId(),
    type,
    payload,
    time: Date.now(),
  } satisfies ClientEventPayload);
}

/* ──────────────────────── Tauri 事件 → 统一总线（一次性装配） ──────────────────────── */

let tauriBridgeInstalled = false;

/**
 * 把客户端（Rust 侧）经 Tauri 事件回推的全部消息一次性汇入统一总线。
 * 仅在主窗口装配一次，应用生命周期内保持监听、不再取消。
 */
export async function installClientTauriBridge(): Promise<void> {
  if (!isTauri() || tauriBridgeInstalled) {
    return;
  }
  tauriBridgeInstalled = true;

  const bus = useBridgeBus();
  const forward =
    (type: string) =>
    (event: { payload: unknown }) =>
      bus.emit({ env: "client", type, payload: event.payload, time: Date.now() });

  // 装配后不保存 unlisten：保持监听、移除 message-remove 行为。
  await Promise.all([
    listen(CLIENT_TO_VIEW_EVENT, forward(CLIENT_TO_VIEW_EVENT)),
    listen(CLIENT_NOTIFICATION_EVENT, forward(CLIENT_NOTIFICATION_EVENT)),
    listen(CLIENT_IFRAME_EVENT, forward(CLIENT_IFRAME_EVENT)),
    listen(CLIENT_TODO_TASK_EVENT, forward(CLIENT_TODO_TASK_EVENT)),
    listen(CLIENT_LOG_EVENT, forward(CLIENT_LOG_EVENT)),
    listen(CLIENT_SOCKET_EVENT, forward(CLIENT_SOCKET_EVENT)),
  ]);
}
