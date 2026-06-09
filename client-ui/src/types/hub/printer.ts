export interface PrintState {
  printServerReady: boolean;
  printerId: string;
  printerName: string;
  status: "idle" | "unavailable" | "unknown" | string;
}

export interface AuthDirectDeviceData {
  deviceId: string;
  deviceName: string;
  deviceIp: string;
  deviceAuthenticate?: string;
  authType?: number;
}
