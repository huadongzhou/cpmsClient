import { unwrapCommand } from "@/api/tauri/client";
import type { ClientHttpRequest, DesktopNotificationEventPayload } from "@/types/app/ipc";
import type {
  ClientIframeStatePayload,
  ClientSocketStatePayload,
  PrintClientInfo,
} from "@/types/app/runtime";

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

/** 由视图端设置 iframe 容器地址（入口页手动输入）。 */
export function setClientIframeUrl(url: string) {
  return unwrapCommand<ClientIframeStatePayload>("client_set_iframe_container_url", { url });
}

/** 视图端把 iframe token 查询结果回传给客户端（标准信封：payload 为 token 字符串）。 */
export function submitClientIframePayload(report: {
  id: string;
  payload?: string;
  ok: boolean;
  reason?: string;
  error?: string;
}) {
  return unwrapCommand<boolean>("client_submit_iframe_payload", {
    id: report.id,
    payload: report.payload ?? null,
    ok: report.ok,
    reason: report.reason ?? null,
    // Tauri v1 保留 invoke 字段 error(usize 回调 id)，命令实参改用 failure 避免撞名覆盖。
    failure: report.error ?? null,
  });
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

/** 设置会话级服务端地址（iframe 推送）。 */
export function setClientSessionServerAddress(addr: string) {
  return unwrapCommand<boolean>("set_session_server_address", { addr });
}

/** 清空会话级服务端地址。 */
export function clearClientSessionServerAddress() {
  return unwrapCommand<boolean>("clear_session_server_address");
}

/** 设置会话级直连设备 ID（iframe 推送）。 */
export function setClientSessionDirectDeviceId(deviceId: string) {
  return unwrapCommand<boolean>("set_session_direct_device_id", { deviceId });
}

/** 清空会话级直连设备 ID。 */
export function clearClientSessionDirectDeviceId() {
  return unwrapCommand<boolean>("clear_session_direct_device_id");
}

/** 设置会话级平台标识（iframe 推送）。 */
export function setClientSessionPlatform(platform: string) {
  return unwrapCommand<boolean>("set_session_platform", { platform });
}

/** 清空会话级平台标识。 */
export function clearClientSessionPlatform() {
  return unwrapCommand<boolean>("clear_session_platform");
}
