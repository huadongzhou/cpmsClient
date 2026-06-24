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
