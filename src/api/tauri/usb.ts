import { unwrapCommand } from "./client";
import type { UsbData, UsbState } from "@/types/hub/usb";

/** 读取当前 USB 打印机状态。 */
export function getUsbState() {
  return unwrapCommand<UsbState>("get_usb_state");
}

/** 初始化 USB 打印机并返回持久化的 USB 数据。 */
export function initUsbPrinter() {
  return unwrapCommand<UsbData | null>("init_usb_printer");
}
