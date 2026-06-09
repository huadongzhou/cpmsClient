import { defineStore } from "pinia";
import type { PrintState, AuthDirectDeviceData } from "@/types/hub/printer";
import type { SocketState } from "@/types/hub/socket";
import type { UsbState } from "@/types/hub/usb";

const DEFAULT_PRINT_STATE: PrintState = {
  printServerReady: false,
  printerId: "cpmsHubInsoluId1103",
  printerName: "迅维打印(枢纽站)",
  status: "unknown",
};

const DEFAULT_USB_STATE: UsbState = {
  usbPrinterExists: false,
  usbData: null,
  running: false,
};

const DEFAULT_SOCKET_STATE: SocketState = {
  listening: false,
  host: "127.0.0.1",
  port: null,
};

export const useHubPrintStore = defineStore("hubPrint", () => {
  const printState = ref<PrintState>({ ...DEFAULT_PRINT_STATE });
  const usbState = ref<UsbState>({ ...DEFAULT_USB_STATE });
  const socketState = ref<SocketState>({ ...DEFAULT_SOCKET_STATE });
  const authDirectDevice = ref<AuthDirectDeviceData | null>(null);

  /** 写入虚拟打印机状态。 */
  function setPrintState(nextState: PrintState) {
    printState.value = nextState;
  }

  /** 写入 USB 打印机状态。 */
  function setUsbState(nextState: UsbState) {
    usbState.value = nextState;
  }

  /** 写入本地 Socket Server 状态。 */
  function setSocketState(nextState: SocketState) {
    socketState.value = nextState;
  }

  /** 写入用户指定的直连打印机。 */
  function setAuthDirectDevice(device: AuthDirectDeviceData | null) {
    authDirectDevice.value = device;
  }

  return {
    printState,
    usbState,
    socketState,
    authDirectDevice,
    setPrintState,
    setUsbState,
    setSocketState,
    setAuthDirectDevice,
  };
});
