export interface UsbData {
  manufacturerName: string;
  productName: string;
  uuid: string;
}

export interface UsbState {
  usbPrinterExists: boolean;
  usbData?: UsbData | null;
  running: boolean;
}
