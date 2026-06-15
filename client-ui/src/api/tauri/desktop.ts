import { unwrapCommand } from "@/api/tauri/client";
import type { ClientHttpRequest, DesktopNotificationEventPayload } from "@/types/app/ipc";
import type {
  ClientIframeStatePayload,
  ClientSocketStatePayload,
  PrintClientInfo,
} from "@/types/app/runtime";

/** 让客户端主动向视图端派发事件。 */
export function emitClientEvent(name: string, payload?: unknown) {
  return unwrapCommand<boolean>("emit_client_event", { name, payload });
}

/** 让客户端向视图端派发通知事件。 */
export function pushClientNotificationEvent(notification: DesktopNotificationEventPayload) {
  return unwrapCommand<boolean>("push_desktop_notification_event", { notification });
}

/** 获取客户端缓存的 iframe 容器状态。 */
export function getClientIframeContainerState() {
  return unwrapCommand<ClientIframeStatePayload>("client_get_iframe_container_state");
}

/** 触发客户端刷新 iframe 容器地址。 */
export function refreshClientIframeContainer() {
  return unwrapCommand<ClientIframeStatePayload>("client_refresh_iframe_container");
}

/** 由客户端发起请求，触发视图端查询 iframe payload。 */
export function requestClientIframePayload(reason?: string) {
  return unwrapCommand<string>("client_request_iframe_payload", { reason });
}

/** 视图端把 iframe payload 查询结果回传给客户端。 */
export function submitClientIframePayload(requestId: string, payload?: unknown) {
  return unwrapCommand<boolean>("client_submit_iframe_payload", { requestId, payload });
}

/** 由客户端代理请求线上服务端。 */
export function clientHttpRequest<T = unknown>(request: ClientHttpRequest) {
  return unwrapCommand<T>("client_http_request", { request });
}

/** 查询开机自启动状态。 */
export function getAutostartEnabled() {
  return unwrapCommand<boolean>("autostart_is_enabled");
}

/** 设置开机自启动开关。 */
export function setAutostartEnabled(enabled: boolean) {
  return unwrapCommand<boolean>("autostart_set_enabled", { enabled });
}

/** 触发客户端立即重连本地 PrintClient socket 服务。 */
export function reconnectSocket() {
  return unwrapCommand<boolean>("reconnect_socket");
}

/** 读取本地 socket 连接状态（完整地址/端口/连接状态）。 */
export function getSocketState() {
  return unwrapCommand<ClientSocketStatePayload>("get_socket_state");
}

/** 读取本地 PrintClient 安装路径、DriverClient.ini 内容与 WebsocketPort。 */
export function getPrintClientInfo() {
  return unwrapCommand<PrintClientInfo>("get_print_client_info");
}
