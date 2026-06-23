import type { Ref } from "vue";
import { listen as tauriListen } from "@tauri-apps/api/event";
import { unwrapCommand } from "@/api/tauri/client";

/**
 * 调用方法名与参数列表。
 */
type HubClientMethod = keyof HubClientBridge;

interface HubClientBridge {
  "cpms:ping": (server: ServerLike) => Promise<ClientPingResult>;
  getStartupState: () => Promise<unknown>;
  savePolicyAgreed: () => Promise<unknown>;
  saveAuthState: (state: unknown) => Promise<unknown>;
  clearAuthState: () => Promise<unknown>;
  saveAuthToken: (token: string) => Promise<unknown>;
  saveServerInfo: (server: unknown) => Promise<unknown>;
  saveDirectDevice: (device: unknown) => Promise<unknown>;
  getJobList: (params: Record<string, unknown>) => Promise<unknown>;
  getAvailableDevices: () => Promise<unknown>;
  selectDirectDevice: (device: unknown) => Promise<unknown>;
  systemInit: () => Promise<unknown>;
  systemDestroy: () => Promise<unknown>;
  startBackgroundTasks: () => Promise<unknown>;
  stopBackgroundTasks: () => Promise<unknown>;
  closeWindowWithConfirm: () => Promise<unknown>;
  getAppVersion: () => Promise<unknown>;
  openExternal: (url: string) => Promise<unknown>;
  signRequest: (uri: string, params?: string) => Promise<unknown>;
  pushLog: (entry: {
    level?: string;
    source?: string;
    message: string;
    detail?: string;
  }) => Promise<unknown>;
  listen: (eventName: string, handler: (payload: unknown) => void) => Promise<() => void>;
}

interface ServerLike {
  server?: string;
  https?: boolean;
}

interface ClientPingResult {
  ok: boolean;
  status?: number;
  elapsed?: number;
  message?: string;
}

const HUB_CLIENT_REQUEST = "cpms:hub-client:request";
const HUB_CLIENT_RESPONSE = "cpms:hub-client:response";
const HUB_CLIENT_LISTEN = "cpms:hub-client:listen";
const HUB_CLIENT_EVENT = "cpms:hub-client:event";
const HUB_CLIENT_UNSUBSCRIBE = "cpms:hub-client:unsubscribe";

/**
 * 创建可与 Tauri 后端交互的桥接对象。
 * 该对象不再直接暴露给跨源 iframe；改由 message 桥统一转发。
 */
export function createHubClientBridge(): HubClientBridge {
  return {
    "cpms:ping": pingCpmsServer,
    getStartupState: () => unwrapCommand("get_startup_state"),
    savePolicyAgreed: () => unwrapCommand("save_policy_agreed"),
    saveAuthState: (state: unknown) => unwrapCommand("save_auth_state", { state }),
    clearAuthState: () => unwrapCommand("clear_auth_state"),
    saveAuthToken: (token: string) => unwrapCommand("save_auth_token", { token }),
    saveServerInfo: (server: unknown) => unwrapCommand("save_server_info", { server }),
    saveDirectDevice: (device: unknown) => unwrapCommand("save_direct_device", { device }),
    getJobList: (params: Record<string, unknown>) =>
      unwrapCommand("get_job_list", {
        pageNumber: params.pageNumber,
        pageSize: params.pageSize,
        jobType: params.type,
        title: params.title ?? "",
        searchTime: params.searchTime ?? "",
      }),
    getAvailableDevices: () => unwrapCommand("get_available_devices"),
    selectDirectDevice: (device: unknown) => unwrapCommand("select_direct_device", { device }),
    systemInit: () => unwrapCommand("system_init"),
    systemDestroy: () => unwrapCommand("system_destroy"),
    startBackgroundTasks: () => unwrapCommand("start_background_tasks"),
    stopBackgroundTasks: () => unwrapCommand("stop_background_tasks"),
    closeWindowWithConfirm: () => unwrapCommand("close_window_with_confirm"),
    getAppVersion: () => unwrapCommand("get_app_version"),
    openExternal: (url: string) => unwrapCommand("open_external", { url }),
    signRequest: (uri: string, params?: string) =>
      unwrapCommand("sign_request", { uri, params: params ?? "" }),
    pushLog: (entry) =>
      unwrapCommand("push_client_log", {
        level: entry.level,
        source: entry.source ?? "iframe",
        message: entry.message,
        detail: entry.detail,
      }),
    listen: async (eventName, handler) => {
      const unlisten = await tauriListen(eventName, (event) => {
        handler(event.payload);
      });
      return unlisten;
    },
  };
}

async function pingCpmsServer(server: ServerLike): Promise<ClientPingResult> {
  const host = normalizePingHost(server.server);
  if (!host) {
    throw new Error("服务地址不能为空");
  }

  const begin = Date.now();
  const result = await unwrapCommand<ClientPingResult>("client_ping_address", { host });

  return {
    ok: true,
    status: result.status,
    elapsed: result.elapsed ?? Date.now() - begin,
    message: result.message,
  };
}

function normalizePingHost(server?: string) {
  const value = server?.trim();
  if (!value) {
    return "";
  }

  const withoutProtocol = value.replace(/^https?:\/\//i, "");
  return withoutProtocol.split("/")[0]?.split(":")[0]?.trim() ?? "";
}

/**
 * 保留旧注入方式：仅在非 iframe 场景或同源场景下供 hub-platform 直接读取。
 * 实际跨源 iframe 统一走 postMessage 桥。
 */
export function injectHubClientBridge() {
  (window as unknown as Record<string, unknown>).__HUB_CLIENT__ = createHubClientBridge();
}

interface PendingRequest {
  resolve: (value: unknown) => void;
  reject: (reason: Error) => void;
}

/**
 * 为当前主窗口的业务 iframe 启动跨源 postMessage 桥。
 * 监听 iframe 发来的方法调用/事件订阅请求，调用本地 Tauri 能力后通过 postMessage 返回结果。
 */
export function startHubClientMessageBridge(iframeRef: Ref<HTMLIFrameElement | undefined>) {
  const bridge = createHubClientBridge();
  const pendingRequests = new Map<string, PendingRequest>();
  const subscriptions = new Map<string, () => void>();
  let subscriptionCounter = 0;

  function isFromBusinessIframe(event: MessageEvent<unknown>): boolean {
    const iframe = iframeRef.value;
    if (!iframe) return false;
    return event.source === iframe.contentWindow;
  }

  function postTo(source: MessageEventSource | null, message: unknown) {
    if (source && "postMessage" in source) {
      (source as Window).postMessage(message, "*");
    }
  }

  async function handleRequest(
    event: MessageEvent<unknown>,
    data: {
      id: string;
      method: string;
      args?: unknown[];
    },
  ) {
    const { id, method, args = [] } = data;
    const target = bridge[method as HubClientMethod];

    if (typeof target !== "function") {
      postTo(event.source, {
        type: HUB_CLIENT_RESPONSE,
        id,
        ok: false,
        error: { code: "UNKNOWN_METHOD", message: `未知方法: ${method}` },
      });
      return;
    }

    try {
      const result = await (target as (...arguments_: unknown[]) => Promise<unknown>)(...args);
      postTo(event.source, { type: HUB_CLIENT_RESPONSE, id, ok: true, result });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      const code = error instanceof Error && "code" in error ? String(error.code) : "BRIDGE_ERROR";
      postTo(event.source, { type: HUB_CLIENT_RESPONSE, id, ok: false, error: { code, message } });
    }
  }

  async function handleListen(
    event: MessageEvent<unknown>,
    data: {
      id: string;
      eventName: string;
    },
  ) {
    const { id, eventName } = data;
    subscriptionCounter += 1;
    const subscriptionId = `sub-${subscriptionCounter}`;

    try {
      const unlisten = await bridge.listen(eventName, (payload) => {
        postTo(event.source, { type: HUB_CLIENT_EVENT, subscriptionId, eventName, payload });
      });
      subscriptions.set(subscriptionId, unlisten);
      postTo(event.source, { type: HUB_CLIENT_RESPONSE, id, ok: true, result: { subscriptionId } });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      postTo(event.source, {
        type: HUB_CLIENT_RESPONSE,
        id,
        ok: false,
        error: { code: "LISTEN_ERROR", message },
      });
    }
  }

  function handleUnsubscribe(data: { subscriptionId: string }) {
    const unlisten = subscriptions.get(data.subscriptionId);
    if (unlisten) {
      unlisten();
      subscriptions.delete(data.subscriptionId);
    }
  }

  function onMessage(event: MessageEvent<unknown>) {
    if (!isFromBusinessIframe(event)) return;

    const data = event.data as Record<string, unknown> | undefined;
    if (!data || typeof data !== "object") return;

    switch (data.type) {
      case HUB_CLIENT_REQUEST:
        void handleRequest(event, data as { id: string; method: string; args?: unknown[] });
        break;
      case HUB_CLIENT_LISTEN:
        void handleListen(event, data as { id: string; eventName: string });
        break;
      case HUB_CLIENT_UNSUBSCRIBE:
        handleUnsubscribe(data as { subscriptionId: string });
        break;
      default:
        break;
    }
  }

  window.addEventListener("message", onMessage);

  return () => {
    window.removeEventListener("message", onMessage);
    for (const unlisten of subscriptions.values()) {
      unlisten();
    }
    subscriptions.clear();
    pendingRequests.clear();
  };
}
