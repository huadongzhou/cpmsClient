import type { UnlistenFn } from "@tauri-apps/api/event";
import { listenClientIframeEvent, listenClientTodoTaskEvent } from "@/api/tauri/events";
import { getClientIframeContainerState } from "@/api/tauri/desktop";
import { useRuntimeStore } from "@/stores/runtime";
import { useTaskStore } from "@/stores/task";

/** 监听客户端运行时事件：iframe 状态与本地 socket 任务。 */
export function useClientRuntimeBridge() {
  const runtimeStore = useRuntimeStore();
  const taskStore = useTaskStore();
  let unlistenIframe: UnlistenFn | undefined;
  let unlistenTodo: UnlistenFn | undefined;

  onMounted(async () => {
    try {
      const iframeState = await getClientIframeContainerState();
      runtimeStore.setIframeState(iframeState);
    } catch {
      // ignore
    }

    unlistenIframe = await listenClientIframeEvent((payload) => {
      runtimeStore.setIframeState(payload);
    });

    unlistenTodo = await listenClientTodoTaskEvent((payload) => {
      taskStore.upsertTodoTask(payload);
    });
  });

  onBeforeUnmount(() => {
    unlistenIframe?.();
    unlistenTodo?.();
  });
}
