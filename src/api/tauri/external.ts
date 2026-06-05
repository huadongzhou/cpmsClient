import { unwrapCommand } from "./client";

/** 使用系统默认浏览器打开外部链接。 */
export function openExternal(url: string) {
  return unwrapCommand<boolean>("open_external", { url });
}
