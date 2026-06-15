import { listen as tauriListen } from "@tauri-apps/api/event";
import { invokeCommand } from "@/api/tauri/client";

/**
 * 向全局 window 注入 `__HUB_CLIENT__` 桥接对象，供 iframe 内的 hub-platform 调用。
 * 统一经由 api/tauri 的 invokeCommand 入口透传 Rust CommandResult 原始结构，
 * 以及 Tauri 事件监听能力。
 */
export function injectHubClientBridge() {
  const bridge = {
    getStartupState: () => invokeCommand("get_startup_state"),
    savePolicyAgreed: () => invokeCommand("save_policy_agreed"),
    saveAuthState: (state: unknown) => invokeCommand("save_auth_state", { state }),
    clearAuthState: () => invokeCommand("clear_auth_state"),
    saveAuthToken: (token: string) => invokeCommand("save_auth_token", { token }),
    saveServerInfo: (server: unknown) => invokeCommand("save_server_info", { server }),
    saveDirectDevice: (device: unknown) => invokeCommand("save_direct_device", { device }),
    getJobList: (params: Record<string, unknown>) =>
      invokeCommand("get_job_list", {
        pageNumber: params.pageNumber,
        pageSize: params.pageSize,
        jobType: params.type,
        title: params.title ?? "",
        searchTime: params.searchTime ?? "",
      }),
    getAvailableDevices: () => invokeCommand("get_available_devices"),
    selectDirectDevice: (device: unknown) => invokeCommand("select_direct_device", { device }),
    systemInit: () => invokeCommand("system_init"),
    systemDestroy: () => invokeCommand("system_destroy"),
    startBackgroundTasks: () => invokeCommand("start_background_tasks"),
    stopBackgroundTasks: () => invokeCommand("stop_background_tasks"),
    closeWindowWithConfirm: () => invokeCommand("close_window_with_confirm"),
    getAppVersion: () => invokeCommand("get_app_version"),
    openExternal: (url: string) => invokeCommand("open_external", { url }),
    signRequest: (uri: string, params?: string) =>
      invokeCommand("sign_request", { uri, params: params ?? "" }),
    pushLog: (entry: { level?: string; source?: string; message: string; detail?: string }) =>
      invokeCommand("push_client_log", {
        level: entry.level,
        source: entry.source ?? "iframe",
        message: entry.message,
        detail: entry.detail,
      }),

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
