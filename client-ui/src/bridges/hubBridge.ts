import type { Ref } from "vue";
import { listen as tauriListen } from "@tauri-apps/api/event";
import { unwrapCommand } from "@/api/tauri/client";
import { pushClientLog } from "@/api/tauri/log";
import {
  type BridgeMessage,
  HUB_CLIENT_EVENT,
  HUB_CLIENT_LISTEN,
  HUB_CLIENT_REQUEST,
  HUB_CLIENT_RESPONSE,
  HUB_CLIENT_UNSUBSCRIBE,
  useBridgeBus,
} from "@/api/tauri/events";

/**
 * 调用方法名与参数列表。
 */
type HubClientMethod = keyof HubClientBridge;

interface HubClientBridge {
  ping: (server: ServerLike) => Promise<ClientPingResult>;
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

/**
 * 创建可与 Tauri 后端交互的桥接对象。
 * 该对象不再直接暴露给跨源 iframe；改由 message 桥统一转发。
 */
export function createHubClientBridge(): HubClientBridge {
  return {
    ping: pingCpmsServer,
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

/** 守卫：判断 message 是否为统一消息实体。 */
function isBridgeMessage(value: unknown): value is BridgeMessage {
  return typeof value === "object" && value !== null && typeof (value as BridgeMessage).type === "string";
}

let messageBridgeInstalled = false;

/**
 * 父窗口侧唯一的 `message` 监听：统一处理来自业务 iframe 的全部消息。
 * - 带 `log` 键的消息自动落盘到客户端日志，前缀 `[env/logType]`；
 * - RPC 调用/事件订阅（hub-client:*）就地处理并回推统一实体；
 * - 其余业务态推送（token/serverAddress/...）汇入统一总线，多文件订阅。
 * 安装后保持监听、不再 removeEventListener。
 */
export function startHubClientMessageBridge(iframeRef: Ref<HTMLIFrameElement | undefined>) {
  if (messageBridgeInstalled) {
    return () => {};
  }
  messageBridgeInstalled = true;

  const bridge = createHubClientBridge();
  const bus = useBridgeBus();
  const subscriptions = new Map<string, () => void>();
  let subscriptionCounter = 0;

  function isFromBusinessIframe(event: MessageEvent<unknown>): boolean {
    const iframe = iframeRef.value;
    if (!iframe) return false;
    return event.source === iframe.contentWindow;
  }

  function postTo(source: MessageEventSource | null, message: BridgeMessage) {
    if (source && "postMessage" in source) {
      (source as Window).postMessage(message, "*");
    }
  }

  async function handleRequest(
    event: MessageEvent<unknown>,
    message: BridgeMessage<{ method: string; args?: unknown[] }>,
  ) {
    const { id } = message;
    const method = message.payload?.method ?? "";
    const args = message.payload?.args ?? [];
    const target = bridge[method as HubClientMethod];

    if (typeof target !== "function") {
      postTo(event.source, {
        env: "client",
        type: HUB_CLIENT_RESPONSE,
        id,
        time: Date.now(),
        status: "error",
        payload: { code: "UNKNOWN_METHOD", message: `未知方法: ${method}` },
      });
      return;
    }

    try {
      const result = await (target as (...arguments_: unknown[]) => Promise<unknown>)(...args);
      postTo(event.source, {
        env: "client",
        type: HUB_CLIENT_RESPONSE,
        id,
        time: Date.now(),
        status: "ok",
        payload: result,
      });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      const code = error instanceof Error && "code" in error ? String(error.code) : "BRIDGE_ERROR";
      postTo(event.source, {
        env: "client",
        type: HUB_CLIENT_RESPONSE,
        id,
        time: Date.now(),
        status: "error",
        payload: { code, message: errorMessage },
      });
    }
  }

  async function handleListen(
    event: MessageEvent<unknown>,
    message: BridgeMessage<{ eventName: string }>,
  ) {
    const { id } = message;
    const eventName = message.payload?.eventName ?? "";
    subscriptionCounter += 1;
    const subscriptionId = `sub-${subscriptionCounter}`;

    try {
      const unlisten = await bridge.listen(eventName, (payload) => {
        postTo(event.source, {
          env: "client",
          type: HUB_CLIENT_EVENT,
          time: Date.now(),
          payload: { subscriptionId, eventName, data: payload },
        });
      });
      subscriptions.set(subscriptionId, unlisten);
      postTo(event.source, {
        env: "client",
        type: HUB_CLIENT_RESPONSE,
        id,
        time: Date.now(),
        status: "ok",
        payload: { subscriptionId },
      });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      postTo(event.source, {
        env: "client",
        type: HUB_CLIENT_RESPONSE,
        id,
        time: Date.now(),
        status: "error",
        payload: { code: "LISTEN_ERROR", message: errorMessage },
      });
    }
  }

  function handleUnsubscribe(message: BridgeMessage<{ subscriptionId: string }>) {
    const subscriptionId = message.payload?.subscriptionId ?? "";
    const unlisten = subscriptions.get(subscriptionId);
    if (unlisten) {
      unlisten();
      subscriptions.delete(subscriptionId);
    }
  }

  /** 带 log 键的消息自动落盘：前缀 `[env/logType]`。 */
  function autoRecordLog(message: BridgeMessage) {
    if (!message.log) {
      return;
    }
    const logType = message.logType ?? "log";
    void pushClientLog({
      level: "info",
      source: `${message.env}/${logType}`,
      message: `[${message.env}/${logType}] ${message.log}`,
    }).catch(() => undefined);
  }

  function onMessage(event: MessageEvent<unknown>) {
    if (!isFromBusinessIframe(event)) return;

    const message = event.data;
    if (!isBridgeMessage(message)) return;

    autoRecordLog(message);

    switch (message.type) {
      case HUB_CLIENT_REQUEST:
        void handleRequest(event, message as BridgeMessage<{ method: string; args?: unknown[] }>);
        break;
      case HUB_CLIENT_LISTEN:
        void handleListen(event, message as BridgeMessage<{ eventName: string }>);
        break;
      case HUB_CLIENT_UNSUBSCRIBE:
        handleUnsubscribe(message as BridgeMessage<{ subscriptionId: string }>);
        break;
      default:
        // 业务态推送（token/serverAddress/deviceId/platform...）统一进总线。
        bus.emit(message);
        break;
    }
  }

  window.addEventListener("message", onMessage);

  // 保持监听、移除 message-remove 行为：返回空清理函数以兼容旧调用点。
  return () => {};
}
