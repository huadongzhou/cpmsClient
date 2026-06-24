import {
  type BridgeMessage,
  CLIENT_EVENT,
  CLIENT_TO_VIEW_EVENT,
  createMessageId,
  IFRAME_DIRECT_DEVICE_EVENT,
  IFRAME_PLATFORM_EVENT,
  IFRAME_REFRESH_EVENT,
  IFRAME_SERVER_ADDRESS_EVENT,
  IFRAME_TOKEN_EVENT,
  useBridgeBus,
} from "@/api/tauri/events";
import {
  clearClientSessionDirectDeviceId,
  clearClientSessionPlatform,
  clearClientSessionServerAddress,
  setClientSessionDirectDeviceId,
  setClientSessionPlatform,
  setClientSessionServerAddress,
  submitClientIframePayload,
} from "@/api/tauri/desktop";
import type { ClientEventPayload } from "@/types/app/ipc";

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
 * - 统一出口：订阅统一事件总线，不再自行 addEventListener；总线消息已由父窗口唯一的
 *   `message` 监听汇入。处理 ①手动取 token 的响应、②iframe 主动推送的 token / 会话态。
 * - 常规登录/水合由 iframe 主动推送；客户端仅在鉴权失败等明确场景下手动请求。
 */
export function useIframePayloadBridge(iframeRef: Ref<HTMLIFrameElement | undefined>) {
  // 待回的手动查询：id → 解析器（统一出口收到对应 id 的响应时解析）。
  const pending = new Map<string, PendingQuery>();
  const bus = useBridgeBus();

  /** 统一出口：处理总线上与 iframe token / 会话态相关的消息。 */
  function handleBusMessage(message: BridgeMessage) {
    // 客户端（Rust）经 Tauri 回推的指令：手动取 token / 认证过期请求刷新。
    if (message.env === "client" && message.type === CLIENT_TO_VIEW_EVENT) {
      const event = message.payload as ClientEventPayload | undefined;
      if (event?.type === CLIENT_EVENT.IFRAME_PAYLOAD_REQUEST) {
        void queryIframeToken(event.id || createMessageId(), event.reason || "client-request");
      } else if (event?.type === CLIENT_EVENT.IFRAME_REFRESH) {
        postRefreshToIframe();
      }
      return;
    }

    if (message.env !== "iframe") {
      return;
    }

    if (message.type === IFRAME_SERVER_ADDRESS_EVENT) {
      const addr = typeof message.payload === "string" ? message.payload.trim() : "";
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

    if (message.type === IFRAME_DIRECT_DEVICE_EVENT) {
      const deviceId = typeof message.payload === "string" ? message.payload.trim() : "";
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

    if (message.type === IFRAME_PLATFORM_EVENT) {
      const platform = typeof message.payload === "string" ? message.payload.trim() : "";
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

    if (message.type === IFRAME_TOKEN_EVENT) {
      handleTokenMessage(message);
    }
  }

  function handleTokenMessage(message: BridgeMessage) {
    const token = typeof message.payload === "string" ? message.payload.trim() : "";
    const matched = message.id ? pending.get(message.id) : undefined;

    // 机制1：某个手动查询的响应。
    if (matched && message.id) {
      pending.delete(message.id);
      window.clearTimeout(matched.timer);
      const error = token ? undefined : "iframe returned empty token";
      void submitClientIframePayload({
        id: message.id,
        payload: token || undefined,
        ok: Boolean(token),
        reason: matched.reason,
        error,
      });
      matched.resolve({
        id: message.id,
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
        id: message.id || createMessageId(),
        payload: token,
        ok: true,
        reason: "iframe-auto-push",
      });
    }
  }

  // 统一出口：长期订阅总线（保持监听，不退订）。
  bus.on(handleBusMessage);

  onBeforeUnmount(() => {
    for (const [id, entry] of pending) {
      window.clearTimeout(entry.timer);
      entry.resolve({ id, ok: false, reason: entry.reason, error: "bridge unmounted" });
    }
    pending.clear();
  });

  /** 机制1：向 iframe 发送 `token` 请求，等待统一出口里匹配 id 的响应（超时回失败）。
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
          {
            env: "client",
            type: IFRAME_TOKEN_EVENT,
            id,
            payload: null,
            time: Date.now(),
          } satisfies BridgeMessage,
          "*",
        );
      }

      trySend();
    });
  }

  /** 认证过期：向 iframe 发送 `refresh`，请其重新登录/刷新会话。 */
  function postRefreshToIframe() {
    const iframeWindow = iframeRef.value?.contentWindow;
    if (!iframeWindow) {
      return;
    }
    iframeWindow.postMessage(
      {
        env: "client",
        type: IFRAME_REFRESH_EVENT,
        payload: null,
        time: Date.now(),
      } satisfies BridgeMessage,
      "*",
    );
  }

  return {
    queryIframePayload: (reason = "manual-detect") => queryIframeToken(createMessageId(), reason),
  };
}
