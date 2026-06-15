import type { TodoTask } from "@/types/task/todo-task";

export type ClientIframeState = "idle" | "loading" | "loaded" | "invalid-url" | "error";

export interface ClientIframeStatePayload {
  state: ClientIframeState;
  url?: string | null;
  message?: string | null;
  updatedAt: string;
}

export type ClientTodoTaskPayload = TodoTask;

export type ClientSocketStatus = "" | "binding" | "listening" | "failed";

export interface ClientSocketStatePayload {
  url: string;
  port?: number | null;
  status: ClientSocketStatus;
  message?: string | null;
  updatedAt: string;
}

export interface PrintClientInfo {
  installed: boolean;
  processDir?: string | null;
  dir?: string | null;
  configPath?: string | null;
  websocketPort?: number | null;
  serverAddr?: string | null;
  centerServerAddr?: string | null;
  socketUrl: string;
  iniContent?: string | null;
}
