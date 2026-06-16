import type { AppNotificationType } from "@/types/app/notification";

/** 视图端 ↔ 客户端通信的标准消息信封（一层结构），额外字段（reason/ok/error 等）补在同层。 */
export interface ClientEventPayload {
  id: string;
  type: string;
  payload?: unknown;
  time: number;
  reason?: string;
  ok?: boolean;
  error?: string;
}

export interface DesktopNotificationEventPayload {
  type: AppNotificationType;
  title: string;
  message?: string;
  durationMs?: number;
}

export interface ClientHttpRequest {
  method?: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
  url: string;
  headers?: Record<string, string>;
  query?: Record<string, string | number | boolean | null | undefined>;
  body?: unknown;
  timeoutMs?: number;
}
