import { unwrapCommand } from "./client";
import type { PrintState } from "@/types/hub/printer";

/** 注册或修复 CPMS 虚拟打印机。 */
export function addPrinter() {
  return unwrapCommand<PrintState>("add_printer");
}

/** 禁用或移除 CPMS 虚拟打印机。 */
export function disablePrinter() {
  return unwrapCommand<PrintState>("disable_printer");
}

/** 修复 CPMS 虚拟打印机状态。 */
export function fixPrinter() {
  return unwrapCommand<PrintState>("fix_printer");
}
