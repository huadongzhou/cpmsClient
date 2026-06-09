import {
  WebviewWindow as Tauri1WebviewWindow,
  type WindowOptions as Tauri1WindowOptions,
} from "../../node_modules/@tauri-apps/api/window.js";

type LegacyWindowOptions = Tauri1WindowOptions & {
  parent?: string;
  shadow?: boolean;
};

export class WebviewWindow extends Tauri1WebviewWindow {
  constructor(label: string, options?: LegacyWindowOptions) {
    const { parent: _parent, shadow: _shadow, ...tauri1Options } = options ?? {};
    super(label, tauri1Options);
  }

  static getByLabel(label: string): WebviewWindow | null {
    return Tauri1WebviewWindow.getByLabel(label) as WebviewWindow | null;
  }
}

export type WindowOptions = LegacyWindowOptions;
