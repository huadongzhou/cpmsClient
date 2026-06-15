import type { UnlistenFn } from "@tauri-apps/api/event";
import { listenClientEvent, listenClientLogEvent } from "@/api/tauri/events";
import { pushClientLog } from "@/api/tauri/log";
import { useAppStore } from "@/stores/app";
import { useLogStore } from "@/stores/log";
import type { ClientLogLevel } from "@/types/app/log";

/**
 * 统一日志汇集桥：
 * - 客户端（Rust 侧）日志事件 → 日志面板（文件已由客户端落盘，不回推）；
 * - 客户端事件流（含 socket 转发结果）→ 日志面板（关键节点客户端已自行落盘，不回推）；
 * - 应用错误队列 → 日志面板 + 推送到客户端日志文件。
 */
export function useClientLogBridge() {
  const appStore = useAppStore();
  const logStore = useLogStore();
  const seenErrorIds = new Set<string>();
  let unlistenClientEvents: UnlistenFn | undefined;
  let unlistenClientLogs: UnlistenFn | undefined;

  onMounted(async () => {
    unlistenClientEvents = await listenClientEvent((payload) => {
      logStore.appendLog({
        level: /fail|error/i.test(payload.name) ? "error" : "info",
        source: "client-event",
        title: payload.name,
        detail:
          payload.payload === undefined || payload.payload === null
            ? undefined
            : JSON.stringify(payload.payload, null, 2),
      });
    });

    unlistenClientLogs = await listenClientLogEvent((payload) => {
      logStore.appendLog({
        level: normalizeLogLevel(payload.level),
        source: `client/${payload.source}`,
        title: payload.title,
        detail: payload.detail ?? undefined,
      });
    });
  });

  watch(
    () => appStore.errors,
    (errors) => {
      for (const error of errors) {
        if (seenErrorIds.has(error.id)) {
          continue;
        }

        seenErrorIds.add(error.id);
        const level = error.level === "error" ? "error" : "warn";
        logStore.appendLog({
          level,
          source: `error/${error.source}`,
          title: `${error.code}: ${error.message}`,
        });
        void pushClientLog({
          level,
          source: `ui/${error.source}`,
          message: `${error.code}: ${error.message}`,
        }).catch(() => undefined);
      }
    },
    { deep: true },
  );

  onBeforeUnmount(() => {
    unlistenClientEvents?.();
    unlistenClientLogs?.();
  });
}

function normalizeLogLevel(level: string): ClientLogLevel {
  switch (level.trim().toLowerCase()) {
    case "warn":
    case "warning":
      return "warn";
    case "error":
      return "error";
    default:
      return "info";
  }
}
