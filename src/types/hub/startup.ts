import type { UserData } from "./auth";
import type { PrintState, AuthDirectDeviceData } from "./printer";
import type { NetworkState, ServerData } from "./server";
import type { SocketState } from "./socket";
import type { UsbState } from "./usb";

export interface AppVersion {
  version: string;
  buildNumber: string;
}

export interface StartupState {
  policyAgreed: boolean;
  user?: UserData | null;
  server?: ServerData | null;
  productType: number;
  systemInitData?: unknown;
  usbState: UsbState;
  printState: PrintState;
  socketState: SocketState;
  networkState: NetworkState;
  authDirectDevice?: AuthDirectDeviceData | null;
}
