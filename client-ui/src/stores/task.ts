import { defineStore } from "pinia";
import type { TodoTask } from "@/types/task/todo-task";

export const useTaskStore = defineStore("task", () => {
  const todoTasks = ref<TodoTask[]>([]);
  const pendingTodoCount = computed(
    () => todoTasks.value.filter((task) => task.state !== "done").length,
  );

  /** 根据 id 新增或更新 Todo 任务。 */
  function upsertTodoTask(task: Omit<TodoTask, "updatedAt"> & { updatedAt?: string }) {
    const nextTask: TodoTask = {
      ...task,
      updatedAt: task.updatedAt || new Date().toISOString(),
    };
    const index = todoTasks.value.findIndex((item) => item.id === nextTask.id);

    if (index >= 0) {
      todoTasks.value[index] = {
        ...todoTasks.value[index],
        ...nextTask,
        createdAt: todoTasks.value[index].createdAt,
      };
      return;
    }

    todoTasks.value.unshift(nextTask);
    todoTasks.value = todoTasks.value.slice(0, 100);
  }

  return {
    todoTasks,
    pendingTodoCount,
    upsertTodoTask,
  };
});
