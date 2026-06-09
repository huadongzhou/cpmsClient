import type { AppNotificationType } from "@/types/app/notification";

export interface ClientEventPayload {
  name: string;
  payload?: unknown;
  at: string;
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
