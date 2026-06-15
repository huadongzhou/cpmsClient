import type { TodoTask } from "@/types/task/todo-task";

export type ClientIframeState = "idle" | "loading" | "loaded" | "invalid-url" | "error";

export interface ClientIframeStatePayload {
  state: ClientIframeState;
  url?: string | null;
  message?: string | null;
  updatedAt: string;
}

export type ClientTodoTaskPayload = TodoTask;

export type ClientSocketStatus =
  | ""
  | "connecting"
  | "connected"
  | "disconnected"
  | "failed";

export interface ClientSocketStatePayload {
  url: string;
  port?: number | null;
  status: ClientSocketStatus;
  message?: string | null;
  updatedAt: string;
}
