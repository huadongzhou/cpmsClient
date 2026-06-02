export type TodoTaskState = "todo" | "running" | "done" | "failed";

export interface TodoTask {
  id: string;
  title: string;
  detail?: string;
  state: TodoTaskState;
  source: "socket";
  createdAt: string;
  updatedAt: string;
}

export interface TodoTaskSocketMessage {
  id?: string;
  taskId?: string;
  title?: string;
  name?: string;
  detail?: string;
  description?: string;
  state?: string;
  status?: string;
  type?: string;
}
