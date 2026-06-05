import { unwrapCommand } from "./client";
import type { SocketState } from "@/types/hub/socket";

/** 启动本地打印文件 Socket Server。 */
export function startSocketServer() {
  return unwrapCommand<SocketState>("start_socket_server");
}

/** 停止本地打印文件 Socket Server。 */
export function stopSocketServer() {
  return unwrapCommand<SocketState>("stop_socket_server");
}
