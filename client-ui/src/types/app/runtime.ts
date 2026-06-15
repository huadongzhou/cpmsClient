import type { TodoTask } from "@/types/task/todo-task";

export type ClientIframeState = "idle" | "loading" | "loaded" | "invalid-url" | "error";

export interface ClientIframeStatePayload {
  state: ClientIframeState;
  url?: string | null;
  message?: string | null;
  updatedAt: string;
}

export type ClientTodoTaskPayload = TodoTask;
