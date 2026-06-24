import { defineStore } from "pinia";
import type { AppendClientLogInput, ClientLogEntry } from "@/types/app/log";
import { createId } from "@/utils/id";

const MAX_LOGS = 500;

export const useLogStore = defineStore("log", () => {
  const logs = ref<ClientLogEntry[]>([]);

  /** 追加一条客户端日志，最多保留最近 500 条。 */
  function appendLog(input: AppendClientLogInput) {
    const entry: ClientLogEntry = {
      ...input,
      id: createId(),
      at: new Date().toISOString(),
    };

    logs.value = [entry, ...logs.value].slice(0, MAX_LOGS);
    return entry;
  }

  /** 清空日志缓冲。 */
  function clearLogs() {
    logs.value = [];
  }

  return {
    logs,
    appendLog,
    clearLogs,
  };
});
