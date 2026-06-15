import { defineStore } from "pinia";
import type { AppendClientLogInput, ClientLogEntry } from "@/types/app/log";

const MAX_LOGS = 500;

export const useLogStore = defineStore("log", () => {
  const logs = ref<ClientLogEntry[]>([]);

  /** 纯文本日志导出，按时间正序，供复制。 */
  const logText = computed(() =>
    [...logs.value]
      .reverse()
      .map((entry) => {
        const head = `[${entry.at}] [${entry.level.toUpperCase()}] [${entry.source}] ${entry.title}`;
        return entry.detail ? `${head}\n${entry.detail}` : head;
      })
      .join("\n"),
  );

  /** 追加一条客户端日志，最多保留最近 500 条。 */
  function appendLog(input: AppendClientLogInput) {
    const entry: ClientLogEntry = {
      ...input,
      id: createLogId(),
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
    logText,
    appendLog,
    clearLogs,
  };
});

function createLogId() {
  if (window.crypto?.randomUUID) {
    return window.crypto.randomUUID();
  }

  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}
