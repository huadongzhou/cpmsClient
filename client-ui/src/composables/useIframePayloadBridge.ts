import { createMessageId, listenClientEvent } from "@/api/tauri/events";
import {
  clearClientSessionDirectDeviceId,
  clearClientSessionPlatform,
  clearClientSessionServerAddress,
  setClientSessionDirectDeviceId,
  setClientSessionPlatform,
  setClientSessionServerAddress,
  submitClientIframePayload,
} from "@/api/tauri/desktop";
import type { UnlistenFn } from "@tauri-apps/api/event";

/** 与 iframe 之间统一使用的消息类型（取 token / 推送 token 同名）。 */
const IFRAME_TOKEN_EVENT = "cpms:token";
/** iframe 向客户端推送当前登录使用的服务端地址。 */
const IFRAME_SERVER_ADDRESS_EVENT = "cpms:serverAddress";
/** iframe 向客户端推送当前直连设备 ID。 */
const IFRAME_DIRECT_DEVICE_EVENT = "cpms:deviceId";
/** iframe 向客户端推送当前平台标识。 */
const IFRAME_PLATFORM_EVENT = "cpms:platform";
/** 认证过期时通知 iframe 重新登录/刷新会话。 */
const IFRAME_REFRESH_EVENT = "cpms:refresh";

/** iframe 消息（标准信封）：payload 即 token 字符串。 */
interface TokenMessage {
  type: "cpms:token";
  id?: string;
  payload?: string;
}

/** iframe 推送的服务端地址消息。 */
interface ServerAddressMessage {
  type: "cpms:serverAddress";
  payload?: string;
}

/** iframe 推送的直连设备 ID 消息。 */
interface DirectDeviceMessage {
  type: "cpms:deviceId";
  payload?: string;
}

/** iframe 推送的平台标识消息。 */
interface PlatformMessage {
  type: "cpms:platform";
  payload?: string;
}

type IframeMessage = TokenMessage | ServerAddressMessage | DirectDeviceMessage | PlatformMessage;

export interface IframePayloadBridgeResult {
  id: string;
  ok: boolean;
  reason: string;
  token?: string;
  error?: string;
}

interface PendingQuery {
  reason: string;
  resolve: (result: IframePayloadBridgeResult) => void;
  timer: number;
}

/**
 * iframe token 桥：
 * - 统一出口：对 iframe 的 `message` 监听长期保持，单一类型 `cpms:token`，同时处理
 *   ①手动取 token 的响应、②iframe 主动推送的 token。
 * - 常规登录/水合由 iframe 主动推送 token；客户端仅在鉴权失败等明确场景下手动请求。
 */
export function useIframePayloadBridge(iframeRef: Ref<HTMLIFrameElement | undefined>) {
  let unlistenClientEvent: UnlistenFn | undefined;
  // 待回的手动查询：id → 解析器（统一出口收到对应 id 的响应时解析）。
  const pending = new Map<string, PendingQuery>();

  /** 统一出口：处理所有来自 iframe 的消息。 */
  function handleIframeMessage(event: MessageEvent<unknown>) {
    const data = event.data as IframeMessage | undefined;
    if (!data) {
      return;
    }

    if (data.type === IFRAME_SERVER_ADDRESS_EVENT) {
      const addr = typeof data.payload === "string" ? data.payload.trim() : "";
      if (addr) {
        void setClientSessionServerAddress(addr).catch((error) => {
          console.warn("[iframe bridge] 设置会话服务端地址失败", error);
        });
      } else {
        void clearClientSessionServerAddress().catch((error) => {
          console.warn("[iframe bridge] 清空会话服务端地址失败", error);
        });
      }
      return;
    }

    if (data.type === IFRAME_DIRECT_DEVICE_EVENT) {
      const deviceId = typeof data.payload === "string" ? data.payload.trim() : "";
      if (deviceId) {
        void setClientSessionDirectDeviceId(deviceId).catch((error) => {
          console.warn("[iframe bridge] 设置会话直连设备 ID 失败", error);
        });
      } else {
        void clearClientSessionDirectDeviceId().catch((error) => {
          console.warn("[iframe bridge] 清空会话直连设备 ID 失败", error);
        });
      }
      return;
    }

    if (data.type === IFRAME_PLATFORM_EVENT) {
      const platform = typeof data.payload === "string" ? data.payload.trim() : "";
      if (platform) {
        void setClientSessionPlatform(platform).catch((error) => {
          console.warn("[iframe bridge] 设置会话平台标识失败", error);
        });
      } else {
        void clearClientSessionPlatform().catch((error) => {
          console.warn("[iframe bridge] 清空会话平台标识失败", error);
        });
      }
      return;
    }

    if (data.type !== IFRAME_TOKEN_EVENT) {
      return;
    }

    const token = typeof data.payload === "string" ? data.payload.trim() : "";
    const matched = data.id ? pending.get(data.id) : undefined;

    // 机制1：某个手动查询的响应。
    if (matched && data.id) {
      pending.delete(data.id);
      window.clearTimeout(matched.timer);
      const error = token ? undefined : "iframe returned empty token";
      void submitClientIframePayload({
        id: data.id,
        payload: token || undefined,
        ok: Boolean(token),
        reason: matched.reason,
        error,
      });
      matched.resolve({
        id: data.id,
        ok: Boolean(token),
        reason: matched.reason,
        token: token || undefined,
        error,
      });
      return;
    }

    // 机制2：iframe 主动推送，有 token 才更新会话态。
    if (token) {
      void submitClientIframePayload({
        id: data.id || createMessageId(),
        payload: token,
        ok: true,
        reason: "iframe-auto-push",
      });
    }
  }

  onMounted(async () => {
    // 长期保持的统一出口监听。
    window.addEventListener("message", handleIframeMessage);

    // 客户端（Rust）发起的指令：手动取 token / 认证过期请求 iframe 刷新。
    unlistenClientEvent = await listenClientEvent((event) => {
      if (event.type === "client.iframe_payload.request") {
        void queryIframeToken(event.id || createMessageId(), event.reason || "client-request");
        return;
      }
      if (event.type === "client.iframe.refresh") {
        postRefreshToIframe();
      }
    });

  });

  onBeforeUnmount(() => {
    window.removeEventListener("message", handleIframeMessage);
    unlistenClientEvent?.();

    for (const [id, entry] of pending) {
      window.clearTimeout(entry.timer);
      entry.resolve({ id, ok: false, reason: entry.reason, error: "bridge unmounted" });
    }
    pending.clear();
  });

  /** 机制1：向 iframe 发送 `cpms:token` 请求，等待统一出口里匹配 id 的响应（超时回失败）。
   * 若 iframe 尚未加载完成，会轮询等待 contentWindow 最多 5 秒，避免启动初期立即失败。 */
  function queryIframeToken(id: string, reason: string): Promise<IframePayloadBridgeResult> {
    return new Promise((resolve) => {
      const startAt = Date.now();
      const timeoutMs = 8_000;
      const iframeWaitMs = 5_000;

      function trySend() {
        const iframeWindow = iframeRef.value?.contentWindow;
        if (!iframeWindow) {
          if (Date.now() - startAt < iframeWaitMs) {
            window.setTimeout(trySend, 200);
            return;
          }
          const error = "iframe window unavailable";
          void submitClientIframePayload({ id, ok: false, reason, error });
          resolve({ id, ok: false, reason, error });
          return;
        }

        const timer = window.setTimeout(() => {
          if (!pending.has(id)) {
            return;
          }
          pending.delete(id);
          const error = "query token timeout";
          void submitClientIframePayload({ id, ok: false, reason, error });
          resolve({ id, ok: false, reason, error });
        }, timeoutMs - (Date.now() - startAt));

        pending.set(id, { reason, resolve, timer });
        iframeWindow.postMessage(
          { id, type: IFRAME_TOKEN_EVENT, payload: null, time: Date.now(), reason },
          "*",
        );
      }

      trySend();
    });
  }

  /** 认证过期：向 iframe 发送 `cpms:refresh`，请其重新登录/刷新会话。 */
  function postRefreshToIframe() {
    const iframeWindow = iframeRef.value?.contentWindow;
    if (!iframeWindow) {
      return;
    }
    iframeWindow.postMessage(
      { id: createMessageId(), type: IFRAME_REFRESH_EVENT, payload: null, time: Date.now() },
      "*",
    );
  }

  return {
    queryIframePayload: (reason = "manual-detect") => queryIframeToken(createMessageId(), reason),
  };
}
