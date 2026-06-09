export type AppErrorSource = "http" | "tauri" | "service" | "task" | "ui";
export type AppErrorLevel = "info" | "warning" | "error";

export interface AppError {
  id: string;
  source: AppErrorSource;
  level: AppErrorLevel;
  code: string;
  message: string;
  createdAt: string;
}
