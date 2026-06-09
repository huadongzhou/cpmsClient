export type AppNotificationType = "info" | "success" | "warning" | "error";

export interface AppNotification {
  id: string;
  type: AppNotificationType;
  title: string;
  message?: string;
  createdAt: string;
  durationMs: number;
}

export interface PushNotificationInput {
  type?: AppNotificationType;
  title: string;
  message?: string;
  durationMs?: number;
}
