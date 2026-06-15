<script setup lang="ts" name="ExampleView">
import { storeToRefs } from "pinia";
import { emitViewEvent, listenClientEvent, listenClientSocketEvent } from "@/api/tauri/events";
import {
  clientHttpRequest,
  getAutostartEnabled,
  getPrintClientInfo,
  getSocketState,
  pushClientNotificationEvent,
  reconnectSocket,
  setAutostartEnabled,
} from "@/api/tauri/desktop";
import type { ClientSocketStatePayload, PrintClientInfo } from "@/types/app/runtime";
import ErrorNotice from "@/components/common/ErrorNotice.vue";
import { useIframeContainer } from "@/composables/useIframeContainer";
import { useAppNotification } from "@/composables/useAppNotification";
import { getAccessToken } from "@/api/config";
import { useAppStore } from "@/stores/app";
import { useNetworkStore } from "@/stores/network";
import { useRuntimeStore } from "@/stores/runtime";
import { useTaskStore } from "@/stores/task";
import { useUserStore } from "@/stores/user";
import type { IframePayloadBridgeResult } from "@/composables/useIframePayloadBridge";
import type { UnlistenFn } from "@tauri-apps/api/event";

const props = defineProps<{
  queryIframePayload?: (reason?: string) => Promise<IframePayloadBridgeResult>;
}>();

const appStore = useAppStore();
const networkStore = useNetworkStore();
const runtimeStore = useRuntimeStore();
const taskStore = useTaskStore();
const userStore = useUserStore();
const { notify } = useAppNotification();
const { isOnline } = storeToRefs(networkStore);
const { iframe, loadIframeContainer, loading } = useIframeContainer();
const autostartEnabled = ref(false);
const autostartLoading = ref(false);
const notifyResult = ref("");
const communicationInput = ref("view -> client: hello cpms");
const communicationSendText = ref("");
const communicationReceiveText = ref("");
const tokenResult = ref("");
const tokenLoading = ref(false);
const httpResult = ref("");
const httpLoading = ref(false);
const socketResult = ref("");
const socketReconnectLoading = ref(false);
const socketLink = ref<ClientSocketStatePayload>({ url: "", port: null, status: "", updatedAt: "" });
const printClient = ref<PrintClientInfo>();
const printClientLoading = ref(false);
let unlistenClientEvent: UnlistenFn | undefined;
let unlistenClientSocket: UnlistenFn | undefined;

const SOCKET_STATUS_TEXT: Record<string, string> = {
  "": "未初始化",
  connecting: "连接中",
  connected: "已连接",
  disconnected: "已断开",
  failed: "连接失败",
};
const socketStatusText = computed(() => SOCKET_STATUS_TEXT[socketLink.value.status] ?? socketLink.value.status);
const socketLinkUrl = computed(() => socketLink.value.url || socketEndpoint.value);
const socketLinkPort = computed(() => socketLink.value.port ?? "未知");

const pageAddress = computed(() => window.location.href);
const iframeAddress = computed(() => iframe.value.url || "about:blank");
const localTokenStatus = computed(() => (getAccessToken() ? "存在" : "不存在"));
const iframeTokenStatus = computed(() => (runtimeStore.iframeToken ? "存在" : "不存在"));
const socketEndpoint = computed(() => toSocketEndpoint(appStore.config.localServiceUrl));
const latestSocketTask = computed(() => taskStore.todoTasks[0]);

onMounted(async () => {
  try {
    autostartEnabled.value = await getAutostartEnabled();
  } catch {
    autostartEnabled.value = false;
  }

  unlistenClientEvent = await listenClientEvent((payload) => {
    communicationReceiveText.value = ["[Client -> View]", JSON.stringify(payload, null, 2)].join(
      "\n",
    );
  });

  try {
    socketLink.value = await getSocketState();
  } catch {
    // 非 Tauri 环境或尚未初始化时忽略。
  }
  unlistenClientSocket = await listenClientSocketEvent((payload) => {
    socketLink.value = payload;
  });

  await refreshPrintClientInfo();
});

async function refreshPrintClientInfo() {
  printClientLoading.value = true;
  try {
    printClient.value = await getPrintClientInfo();
  } catch {
    printClient.value = undefined;
  } finally {
    printClientLoading.value = false;
  }
}

onBeforeUnmount(() => {
  unlistenClientEvent?.();
  unlistenClientSocket?.();
});

async function runNotificationDetect() {
  const payload = {
    type: "info",
    title: "通知检测",
    message: "desktop notification body message",
    durationMs: 5000,
  } as const;

  notify(payload);
  await pushClientNotificationEvent(payload);
  notifyResult.value = ["[Notify Payload]", JSON.stringify(payload, null, 2)].join("\n");
}

async function runCommunicationDetect() {
  const payload = {
    text: communicationInput.value,
    channel: "cpms:view-to-client",
    at: new Date().toISOString(),
  };

  await emitViewEvent("example.communication.detect", payload);
  communicationSendText.value = [
    "[View -> Client]",
    JSON.stringify(
      {
        event: "example.communication.detect",
        payload,
      },
      null,
      2,
    ),
  ].join("\n");
}

async function runTokenDetect() {
  tokenLoading.value = true;
  tokenResult.value = "";

  const localToken = getAccessToken();

  try {
    const bridgeResult = await props.queryIframePayload?.("token-detect");
    const iframePayloadToken = extractTokenFromIframePayload(bridgeResult?.payload);

    runtimeStore.setIframeToken(iframePayloadToken || "");

    tokenResult.value = [
      "[Token Detect]",
      JSON.stringify(
        {
          localStorageToken: {
            exists: Boolean(localToken),
            value: maskToken(localToken),
          },
          userStoreToken: {
            exists: Boolean(userStore.token),
            value: maskToken(userStore.token || undefined),
          },
          iframeToken: {
            exists: Boolean(iframePayloadToken),
            value: maskToken(iframePayloadToken),
          },
          iframePayloadBridge: normalizeBridgeResult(bridgeResult),
          iframeUrl: iframeAddress.value,
          checkedAt: new Date().toISOString(),
        },
        null,
        2,
      ),
    ].join("\n");
  } finally {
    tokenLoading.value = false;
  }
}

async function toggleAutostart() {
  autostartLoading.value = true;
  try {
    autostartEnabled.value = await setAutostartEnabled(!autostartEnabled.value);
  } finally {
    autostartLoading.value = false;
  }
}

async function runHttpDetect() {
  httpLoading.value = true;
  httpResult.value = "";
  try {
    const result = await clientHttpRequest({
      method: "GET",
      url: appStore.config.cpmsBaseUrl,
      timeoutMs: 4000,
    });
    httpResult.value = [
      `[HTTP Request] GET ${appStore.config.cpmsBaseUrl}`,
      "[HTTP Response]",
      JSON.stringify(result, null, 2),
    ].join("\n");
  } catch (error) {
    httpResult.value = [
      `[HTTP Request] GET ${appStore.config.cpmsBaseUrl}`,
      "[HTTP Error]",
      error instanceof Error ? error.message : "代理请求失败",
    ].join("\n");
  } finally {
    httpLoading.value = false;
  }
}

function runSocketDetect() {
  if (latestSocketTask.value) {
    socketResult.value = [
      `[Socket Connected] ${socketEndpoint.value}`,
      "[Socket -> Client -> View Payload]",
      JSON.stringify(latestSocketTask.value, null, 2),
    ].join("\n");
    return;
  }

  socketResult.value = [
    `[Socket Connected] ${socketEndpoint.value}`,
    "[Socket Simulated Payload]",
    JSON.stringify(
      {
        taskId: "mock-task-001",
        title: "mock socket task",
        status: "running",
        at: new Date().toISOString(),
      },
      null,
      2,
    ),
  ].join("\n");
}

async function runSocketReconnect() {
  socketReconnectLoading.value = true;
  try {
    await reconnectSocket();
    socketResult.value = [
      `[Socket Reconnect] ${socketEndpoint.value}`,
      "已请求客户端立即重连本地 socket 服务，可在「客户端日志」的「任务 / Socket」类别查看重连结果。",
    ].join("\n");
  } catch (error) {
    socketResult.value = [
      "[Socket Reconnect Error]",
      error instanceof Error ? error.message : "重连请求失败",
    ].join("\n");
  } finally {
    socketReconnectLoading.value = false;
  }
}

function toSocketEndpoint(baseUrl: string) {
  const normalized = baseUrl.replace(/\/$/, "");

  if (normalized.endsWith("/ws/task")) {
    return normalized;
  }

  if (normalized.startsWith("ws://") || normalized.startsWith("wss://")) {
    return `${normalized}/ws/task`;
  }

  if (normalized.startsWith("https://")) {
    return `${normalized.replace("https://", "wss://")}/ws/task`;
  }

  return `${normalized.replace("http://", "ws://")}/ws/task`;
}

function maskToken(token?: string) {
  if (!token) {
    return "";
  }

  if (token.length <= 12) {
    return `${token.slice(0, 2)}****${token.slice(-2)}`;
  }

  return `${token.slice(0, 6)}****${token.slice(-6)}`;
}

function extractTokenFromIframePayload(payload: unknown): string | undefined {
  if (!isRecord(payload)) {
    return undefined;
  }

  const directToken =
    readString(payload, "token") ||
    readString(payload, "accessToken") ||
    readString(payload, "access_token");

  if (directToken) {
    return directToken;
  }

  const nestedPayload = payload.payload;

  if (isRecord(nestedPayload)) {
    return (
      readString(nestedPayload, "token") ||
      readString(nestedPayload, "accessToken") ||
      readString(nestedPayload, "access_token")
    );
  }

  return undefined;
}

function normalizeBridgeResult(result?: IframePayloadBridgeResult) {
  if (!result) {
    return {
      ok: false,
      error: "useIframePayloadBridge unavailable",
    };
  }

  return {
    requestId: result.requestId,
    ok: result.ok,
    reason: result.reason,
    error: result.error,
    payload: result.payload,
  };
}

function readString(record: Record<string, unknown>, key: string) {
  const value = record[key];
  return typeof value === "string" ? value : undefined;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
</script>

<template>
  <main class="example">
    <ErrorNotice />

    <section class="card">
      <h2>状态</h2>
      <p>页面地址：{{ pageAddress }}</p>
      <p>iframe 地址：{{ iframeAddress }}</p>
      <p>开机自启动：{{ autostartEnabled ? "已开启" : "已关闭" }}</p>
      <p>网络状态：{{ isOnline ? "online" : "offline" }}</p>
      <el-button :loading="loading" @click="loadIframeContainer">刷新 iframe 地址</el-button>
    </section>

    <section class="card">
      <h2>通知检测</h2>
      <div class="actions">
        <el-button type="primary" plain @click="runNotificationDetect">发送通知</el-button>
      </div>
      <pre v-if="notifyResult" class="result">{{ notifyResult }}</pre>
    </section>

    <section class="card">
      <h2>通信检测</h2>
      <el-input v-model="communicationInput" placeholder="输入模拟传输文本" />
      <div class="actions">
        <el-button type="primary" plain @click="runCommunicationDetect">执行通信检测</el-button>
      </div>
      <pre v-if="communicationSendText" class="result">{{ communicationSendText }}</pre>
      <pre v-if="communicationReceiveText" class="result">{{ communicationReceiveText }}</pre>
    </section>

    <section class="card">
      <h2>Token 检测</h2>
      <p>本地 token：{{ localTokenStatus }}</p>
      <p>iframe token：{{ iframeTokenStatus }}</p>
      <div class="actions">
        <el-button type="primary" plain :loading="tokenLoading" @click="runTokenDetect"
          >执行 Token 检测</el-button
        >
      </div>
      <pre v-if="tokenResult" class="result">{{ tokenResult }}</pre>
    </section>

    <section class="card">
      <h2>本地 CPMS 客户端（PrintClient）</h2>
      <p>安装路径：{{ printClient?.dir || "未检测到" }}</p>
      <p>配置文件：{{ printClient?.configPath || "未检测到" }}</p>
      <p>WebsocketPort：{{ printClient?.websocketPort ?? "未知" }}</p>
      <p>解析地址：{{ printClient?.socketUrl || "未知" }}</p>
      <div class="actions">
        <el-button :loading="printClientLoading" @click="refreshPrintClientInfo"
          >刷新客户端信息</el-button
        >
      </div>
      <pre v-if="printClient?.iniContent" class="result">{{ printClient.iniContent }}</pre>
      <p v-else-if="printClient && !printClient.installed">未读取到 DriverClient.ini 内容。</p>
    </section>

    <section class="card">
      <h2>请求检测</h2>
      <h3>HTTP</h3>
      <el-button type="primary" :loading="httpLoading" @click="runHttpDetect"
        >执行 HTTP 请求检测</el-button
      >
      <pre v-if="httpResult" class="result">{{ httpResult }}</pre>
      <h3>Socket</h3>
      <p>完整链接地址：{{ socketLinkUrl }}</p>
      <p>socket 端口：{{ socketLinkPort }}</p>
      <p>连接状态：{{ socketStatusText }}</p>
      <p v-if="socketLink.message">最近说明：{{ socketLink.message }}</p>
      <div class="actions">
        <el-button type="success" plain @click="runSocketDetect">执行 Socket 请求检测</el-button>
        <el-button
          type="warning"
          plain
          :loading="socketReconnectLoading"
          @click="runSocketReconnect"
          >重连 socket 服务</el-button
        >
      </div>
      <pre v-if="socketResult" class="result">{{ socketResult }}</pre>
    </section>

    <section class="card">
      <h2>开机自启动</h2>
      <el-button :loading="autostartLoading" @click="toggleAutostart">
        {{ autostartEnabled ? "关闭自启动" : "开启自启动" }}
      </el-button>
      <p>当前状态：{{ autostartEnabled ? "已开启" : "已关闭" }}</p>
    </section>
  </main>
</template>

<style scoped>
.example {
  display: grid;
  gap: var(--cpms-space-base);
  padding: var(--cpms-space-base);
}

h2 {
  margin: 0;
  font-size: var(--cpms-font-size-base);
  color: var(--cpms-color-text-primary);
}

h3 {
  margin: 0;
  font-size: var(--cpms-font-size-base);
  color: var(--cpms-color-text-secondary);
}

.card {
  display: grid;
  gap: var(--cpms-space-small);
  background: var(--cpms-color-bg-panel);
  border: 1px solid var(--cpms-color-border);
  border-radius: var(--cpms-radius-panel);
  padding: var(--cpms-space-base);
}

.actions {
  display: flex;
  gap: var(--cpms-space-small);
  flex-wrap: wrap;
}

.result {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  background: var(--cpms-color-bg-code);
  border-radius: var(--cpms-radius-small);
  padding: var(--cpms-space-small);
  font-size: var(--cpms-font-size-small);
}
</style>
