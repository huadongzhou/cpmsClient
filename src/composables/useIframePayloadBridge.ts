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
  payload?: unknown;
}

export interface IframePayloadBridgeResult {
  requestId: string;
  ok: boolean;
  reason: string;
  payload?: unknown;
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
      const result = {
        ok: false,
        reason,
        error: "iframe window unavailable",
      };

      await submitClientIframePayload(requestId, result);
      return {
        requestId,
        ...result,
      };
    }

    try {
      const responsePayload = await queryPayloadFromIframe(iframeWindow, requestId, reason);
      const result = {
        ok: true,
        reason,
        payload: responsePayload,
      };

      await submitClientIframePayload(requestId, result);
      return {
        requestId,
        ...result,
      };
    } catch (error) {
      const result = {
        ok: false,
        reason,
        error: error instanceof Error ? error.message : "query payload timeout",
      };

      await submitClientIframePayload(requestId, result);
      return {
        requestId,
        ...result,
      };
    }
  }

  function queryPayloadFromIframe(iframeWindow: Window, requestId: string, reason: string) {
    return new Promise<unknown>((resolve, reject) => {
      let done = false;
      const timeout = window.setTimeout(() => finish("timeout"), 8_000);

      const listener = (event: MessageEvent<unknown>) => {
        const data = event.data as IframePayloadResponse | undefined;

        if (data?.type !== IFRAME_RESPONSE_EVENT) return;

        if (data.requestId && data.requestId !== requestId) return;

        finish(undefined, data.payload);
      };

      function finish(error?: string, payload?: unknown) {
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

        resolve(payload);
      }

      window.addEventListener("message", listener);
      iframeWindow.postMessage(
        {
          type: IFRAME_QUERY_EVENT,
          requestId,
          reason,
          at: new Date().toISOString(),
        },
        "*",
      );
    });
  }

  return {
    queryIframePayload,
  };
}

function createRequestId(prefix: string) {
  return `${prefix}-${Date.now()}`;
}
