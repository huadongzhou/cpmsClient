import { unwrapCommand } from "./client";
import type { AppVersion } from "@/types/hub/startup";

/** 获取客户端应用版本号。 */
export function getAppVersion() {
  return unwrapCommand<AppVersion>("get_app_version");
}
