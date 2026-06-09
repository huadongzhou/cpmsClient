import { unwrapCommand } from "./client";
import type { PrintState } from "@/types/hub/printer";

/** 启动 print worker（cache 扫描 + 上传）。不涉及系统级虚拟打印机注册。 */
export function addPrinter() {
  return unwrapCommand<PrintState>("add_printer");
}

/** 停止 print worker。不涉及系统级虚拟打印机移除。 */
export function disablePrinter() {
  return unwrapCommand<PrintState>("disable_printer");
}

/** 重新启动 print worker。不涉及系统级虚拟打印机修复。 */
export function fixPrinter() {
  return unwrapCommand<PrintState>("fix_printer");
}
