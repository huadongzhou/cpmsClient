export type ClientLogLevel = "info" | "warn" | "error";

export interface ClientLogEntry {
  id: string;
  at: string;
  level: ClientLogLevel;
  source: string;
  title: string;
  detail?: string;
}

export type AppendClientLogInput = Omit<ClientLogEntry, "id" | "at">;

/** 客户端（Rust 侧）经 cpms:client-log 事件推送的日志载荷。 */
export interface ClientLogEventPayload {
  at: string;
  level: string;
  source: string;
  title: string;
  detail?: string | null;
}

/** 前端向客户端日志文件推送日志的入参。 */
export interface PushClientLogInput {
  level?: "info" | "warn" | "error" | "debug";
  source?: string;
  message: string;
  detail?: string;
}

/** 客户端日志文件状态。 */
export interface ClientLogFileState {
  path: string;
  sizeBytes: number;
}
