import type { Ref } from "vue";
import { unwrapCommand } from "@/api/tauri/client";
import { pushClientLog } from "@/api/tauri/log";
import { type BridgeMessage, useBridgeBus } from "@/api/tauri/events";

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

/** 守卫：判断 message 是否为统一消息实体。 */
function isBridgeMessage(value: unknown): value is BridgeMessage {
  return typeof value === "object" && value !== null && typeof (value as BridgeMessage).type === "string";
}

let messageBridgeInstalled = false;

/**
 * 父窗口侧唯一的 `message` 监听：统一处理来自业务 iframe 的全部消息。
 *
 * 协议（无 hub-client:* 信封，`type` 即真实名字）：
 * - 有 `id` 且无 `status` → iframe 发起的 RPC 请求：以 `type` 为方法名调用本地能力，
 *   回推同 `type` + 同 `id` + `status:'ok'|'error'` 的响应；
 * - 带 `status` 的响应（如取 token 的回包）/ 无 `id` 的单向推送（token/serverAddress/...）
 *   → 汇入统一总线，多文件订阅；
 * - 带 `log` 键的消息自动落盘到客户端日志，前缀 `[env/logType]`。
 *
 * 安装后保持监听、不再 removeEventListener。
 */
export function startHubClientMessageBridge(iframeRef: Ref<HTMLIFrameElement | undefined>) {
  if (messageBridgeInstalled) {
    return () => {};
  }
  messageBridgeInstalled = true;

  const bridge = createHubClientBridge();
  const bus = useBridgeBus();

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

  /** 处理 iframe 发起的 RPC 请求：`type` 即方法名，`payload` 即参数数组；回推同 type+id 的响应。 */
  async function handleRequest(event: MessageEvent<unknown>, message: BridgeMessage) {
    const { id } = message;
    const method = message.type;
    const args = Array.isArray(message.payload) ? message.payload : [];
    const target = bridge[method as HubClientMethod];

    if (typeof target !== "function") {
      postTo(event.source, {
        env: "client",
        type: method,
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
        type: method,
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
        type: method,
        id,
        time: Date.now(),
        status: "error",
        payload: { code, message: errorMessage },
      });
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

    // 有 id 且无 status → iframe 发起的 RPC 请求，就地执行并回推响应。
    if (message.id != null && message.status == null) {
      void handleRequest(event, message);
      return;
    }

    // 其余（带 status 的响应 / 无 id 的单向推送）统一进总线。
    bus.emit(message);
  }

  window.addEventListener("message", onMessage);

  // 保持监听、移除 message-remove 行为：返回空清理函数以兼容旧调用点。
  return () => {};
}
