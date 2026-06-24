import { CLIENT_IFRAME_EVENT, CLIENT_TODO_TASK_EVENT, onBridgeMessage } from "@/api/tauri/events";
import { getClientIframeContainerState } from "@/api/tauri/desktop";
import { useRuntimeStore } from "@/stores/runtime";
import { useTaskStore } from "@/stores/task";
import type { ClientIframeStatePayload, ClientTodoTaskPayload } from "@/types/app/runtime";

/** 订阅客户端运行时事件：iframe 状态与本地 socket 任务。 */
export function useClientRuntimeBridge() {
  const runtimeStore = useRuntimeStore();
  const taskStore = useTaskStore();

  onMounted(async () => {
    try {
      const iframeState = await getClientIframeContainerState();
      runtimeStore.setIframeState(iframeState);
    } catch {
      // ignore
    }
  });

  onBridgeMessage<ClientIframeStatePayload>(CLIENT_IFRAME_EVENT, (payload) => {
    runtimeStore.setIframeState(payload);
  });

  onBridgeMessage<ClientTodoTaskPayload>(CLIENT_TODO_TASK_EVENT, (payload) => {
    taskStore.upsertTodoTask(payload);
  });
}
