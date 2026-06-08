import { invoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";

/**
 * 向全局 window 注入 `__HUB_CLIENT__` 桥接对象，供 iframe 内的 hub-platform 调用。
 * 包含所有 Hub command 的 invoke 封装以及 Tauri 事件监听能力。
 */
export function injectHubClientBridge() {
  const bridge = {
    getStartupState: () => invoke("get_startup_state"),
    savePolicyAgreed: () => invoke("save_policy_agreed"),
    saveAuthState: (state: unknown) => invoke("save_auth_state", { state }),
    clearAuthState: () => invoke("clear_auth_state"),
    saveServerInfo: (server: unknown) => invoke("save_server_info", { server }),
    saveDirectDevice: (device: unknown) => invoke("save_direct_device", { device }),
    systemInit: () => invoke("system_init"),
    systemDestroy: () => invoke("system_destroy"),
    startBackgroundTasks: () => invoke("start_background_tasks"),
    stopBackgroundTasks: () => invoke("stop_background_tasks"),
    addPrinter: () => invoke("add_printer"), // 启动 print worker（不涉及系统级虚拟打印机注册）
    disablePrinter: () => invoke("disable_printer"), // 停止 print worker
    fixPrinter: () => invoke("fix_printer"), // 重新启动 print worker
    initUsbPrinter: () => invoke("init_usb_printer"),
    closeWindowWithConfirm: () => invoke("close_window_with_confirm"),
    getAppVersion: () => invoke("get_app_version"),
    openExternal: (url: string) => invoke("open_external", { url }),
    signRequest: (uri: string, params?: string) =>
      invoke("sign_request", { uri, params: params ?? "" }),
    sm4Encrypt: (text: string) => invoke("sm4_encrypt", { text }),

    /**
     * 订阅 Tauri 事件；由 client 前端统一管理，收到后回调 iframe 传入的 handler。
     * 返回取消订阅函数。
     */
    listen: async (eventName: string, handler: (payload: unknown) => void): Promise<() => void> => {
      const unlisten = await tauriListen(eventName, (event) => {
        handler(event.payload);
      });
      return unlisten;
    },
  };

  (window as unknown as Record<string, unknown>).__HUB_CLIENT__ = bridge;
}
