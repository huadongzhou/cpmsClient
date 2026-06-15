import { listenClientEvent } from "@/api/tauri/events";
import { submitClientIframePayload } from "@/api/tauri/desktop";
import type { UnlistenFn } from "@tauri-apps/api/event";

interface ClientIframePayloadRequest {
  requestId?: string;
  reason?: string;
}

interface IframePayloadResponse {
  type: "cpms:payload-response";
  requestId?: string;
  token?: string;
}

export interface IframePayloadBridgeResult {
  requestId: string;
  ok: boolean;
  reason: string;
  token?: string;
  error?: string;
}

const IFRAME_QUERY_EVENT = "cpms:query-payload";
const IFRAME_RESPONSE_EVENT = "cpms:payload-response";

/** 客户端发起 iframe payload 请求时，视图端查询并回传；并在路由加载 10 秒后自动查询一次。 */
export function useIframePayloadBridge(iframeRef: Ref<HTMLIFrameElement | undefined>) {
  let unlistenClientEvent: UnlistenFn | undefined;
  let autoQueryTimer: number | undefined;

  onMounted(async () => {
    unlistenClientEvent = await listenClientEvent((event) => {
      if (event.name !== "client.iframe_payload.request") {
        return;
      }

      const payload = (event.payload || {}) as ClientIframePayloadRequest;
      void queryAndSubmit(
        payload.requestId || createRequestId("client"),
        payload.reason || "client-request",
      );
    });

    autoQueryTimer = window.setTimeout(() => {
      void queryAndSubmit(createRequestId("auto"), "route-loaded-10s");
    }, 10_000);
  });

  onBeforeUnmount(() => {
    unlistenClientEvent?.();

    if (autoQueryTimer) {
      window.clearTimeout(autoQueryTimer);
      autoQueryTimer = undefined;
    }
  });

  async function queryIframePayload(reason = "manual-detect") {
    return queryAndSubmit(createRequestId("manual"), reason);
  }

  async function queryAndSubmit(
    requestId: string,
    reason: string,
  ): Promise<IframePayloadBridgeResult> {
    const iframeWindow = iframeRef.value?.contentWindow;

    if (!iframeWindow) {
      const result: IframePayloadBridgeResult = {
        requestId,
        ok: false,
        reason,
        error: "iframe window unavailable",
      };

      await submitClientIframePayload(result);
      return result;
    }

    try {
      const token = await queryTokenFromIframe(iframeWindow, requestId, reason);
      const result: IframePayloadBridgeResult = { requestId, ok: true, reason, token };

      await submitClientIframePayload(result);
      return result;
    } catch (error) {
      const result: IframePayloadBridgeResult = {
        requestId,
        ok: false,
        reason,
        error: error instanceof Error ? error.message : "query token timeout",
      };

      await submitClientIframePayload(result);
      return result;
    }
  }

  /** 向 iframe 发送一层结构的查询 `{ type, requestId, reason }`，等待一层结构的回包 `{ type, requestId, token }`。 */
  function queryTokenFromIframe(iframeWindow: Window, requestId: string, reason: string) {
    return new Promise<string | undefined>((resolve, reject) => {
      let done = false;
      const timeout = window.setTimeout(() => finish("timeout"), 8_000);

      const listener = (event: MessageEvent<unknown>) => {
        const data = event.data as IframePayloadResponse | undefined;

        if (data?.type !== IFRAME_RESPONSE_EVENT) return;

        if (data.requestId && data.requestId !== requestId) return;

        finish(undefined, data.token);
      };

      function finish(error?: string, token?: string) {
        if (done) {
          return;
        }

        done = true;
        window.clearTimeout(timeout);
        window.removeEventListener("message", listener);

        if (error) {
          reject(new Error(error));
          return;
        }

        resolve(token);
      }

      window.addEventListener("message", listener);
      iframeWindow.postMessage({ type: IFRAME_QUERY_EVENT, requestId, reason }, "*");
    });
  }

  return {
    queryIframePayload,
  };
}

function createRequestId(prefix: string) {
  return `${prefix}-${Date.now()}`;
}
