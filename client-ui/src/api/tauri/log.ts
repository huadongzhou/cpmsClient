import { isTauri } from "@tauri-apps/api/core";
import { unwrapCommand } from "@/api/tauri/client";
import type { ClientLogFileState, PushClientLogInput } from "@/types/app/log";

/** 把前端日志推送到客户端日志文件（非 Tauri 环境下直接忽略）。 */
export async function pushClientLog(input: PushClientLogInput) {
  if (!isTauri()) {
    return false;
  }

  return unwrapCommand<boolean>("push_client_log", {
    level: input.level,
    source: input.source,
    message: input.message,
    detail: input.detail,
  });
}

/** 查询客户端日志文件路径与大小。 */
export function getClientLogState() {
  return unwrapCommand<ClientLogFileState>("get_client_log_state");
}
