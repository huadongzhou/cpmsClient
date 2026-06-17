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
import { useAppStore } from "@/stores/app";
import { useNetworkStore } from "@/stores/network";
import { useRuntimeStore } from "@/stores/runtime";
import { useTaskStore } from "@/stores/task";
import type { IframePayloadBridgeResult } from "@/composables/useIframePayloadBridge";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { ElMessage } from "element-plus";

const props = defineProps<{
  queryIframePayload?: (reason?: string) => Promise<IframePayloadBridgeResult>;
}>();

const appStore = useAppStore();
const networkStore = useNetworkStore();
const runtimeStore = useRuntimeStore();
const taskStore = useTaskStore();
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
const socketLink = ref<ClientSocketStatePayload>({
  url: "",
  port: null,
  status: "",
  updatedAt: "",
});
const printClient = ref<PrintClientInfo>();
const printClientLoading = ref(false);
let unlistenClientEvent: UnlistenFn | undefined;
let unlistenClientSocket: UnlistenFn | undefined;

const SOCKET_STATUS_TEXT: Record<string, string> = {
  "": "未初始化",
  binding: "绑定中",
  listening: "监听中（等待推送连接）",
  failed: "监听失败",
};
const socketStatusText = computed(
  () => SOCKET_STATUS_TEXT[socketLink.value.status] ?? socketLink.value.status,
);
const socketLinkUrl = computed(() => socketLink.value.url || socketEndpoint.value);
const socketLinkPort = computed(() => socketLink.value.port ?? "未知");

const pageAddress = computed(() => window.location.href);
const iframeAddress = computed(() => iframe.value.url || "about:blank");
const collapsedCards = ref<Set<string>>(new Set());

function isCollapsed(key: string) {
  return collapsedCards.value.has(key);
}

function toggleCard(key: string) {
  const next = new Set(collapsedCards.value);
  if (next.has(key)) {
    next.delete(key);
  } else {
    next.add(key);
  }
  collapsedCards.value = next;
}

async function copyResult(text: string) {
  if (!text) {
    ElMessage.info("暂无可复制内容");
    return;
  }

  await navigator.clipboard.writeText(text);
  ElMessage.success("结果已复制");
}
// 客户端只有一个 token：经 iframe 获取后本地缓存，等待拉取/推送更新。
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

  try {
    const bridgeResult = await props.queryIframePayload?.("token-detect");
    const iframePayloadToken = bridgeResult?.token;

    runtimeStore.setIframeToken(iframePayloadToken || "");

    // 客户端只有一个 token：来自 iframe，获取后即本地缓存（不存在 localStorage/store 等多份存储）。
    tokenResult.value = [
      "[Token Detect]",
      JSON.stringify(
        {
          token: {
            exists: Boolean(iframePayloadToken),
            value: iframePayloadToken || "",
            source: "iframe（获取后本地缓存）",
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
      `[Socket Restart] ${socketLinkUrl.value}`,
      "已请求客户端重启本地 socket 监听服务，可在「客户端日志」的「任务 / Socket」类别查看结果。",
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

function normalizeBridgeResult(result?: IframePayloadBridgeResult) {
  if (!result) {
    return {
      ok: false,
      error: "useIframePayloadBridge unavailable",
    };
  }

  return {
    id: result.id,
    ok: result.ok,
    reason: result.reason,
    error: result.error,
    token: result.token,
  };
}
</script>

<template>
  <main class="example">
    <ErrorNotice />

    <section class="card" :class="{ 'is-collapsed': isCollapsed('status') }">
      <h2>
        状态
        <button type="button" class="collapse-button" @click="toggleCard('status')">
          {{ isCollapsed('status') ? "展开" : "收起" }}
        </button>
      </h2>
      <div class="card-body">
        <p>页面地址：{{ pageAddress }}</p>
        <p>iframe 地址：{{ iframeAddress }}</p>
        <p>开机自启动：{{ autostartEnabled ? "已开启" : "已关闭" }}</p>
        <p>网络状态：{{ isOnline ? "online" : "offline" }}</p>
        <div class="actions">
          <el-button :loading="loading" @click="loadIframeContainer">刷新 iframe 地址</el-button>
        </div>
      </div>
    </section>

    <section class="card" :class="{ 'is-collapsed': isCollapsed('notification') }">
      <h2>
        通知检测
        <button type="button" class="collapse-button" @click="toggleCard('notification')">
          {{ isCollapsed('notification') ? "展开" : "收起" }}
        </button>
      </h2>
      <div class="card-body">
        <div class="actions">
          <el-button type="primary" plain @click="runNotificationDetect">发送通知</el-button>
        </div>
        <div v-if="notifyResult" class="result-block">
          <el-button class="copy-button" size="small" plain @click="copyResult(notifyResult)">
            复制
          </el-button>
          <pre class="result">{{ notifyResult }}</pre>
        </div>
      </div>
    </section>

    <section class="card" :class="{ 'is-collapsed': isCollapsed('communication') }">
      <h2>
        通信检测
        <button type="button" class="collapse-button" @click="toggleCard('communication')">
          {{ isCollapsed('communication') ? "展开" : "收起" }}
        </button>
      </h2>
      <div class="card-body">
        <el-input v-model="communicationInput" placeholder="输入模拟传输文本" />
        <div class="actions">
          <el-button type="primary" plain @click="runCommunicationDetect">执行通信检测</el-button>
        </div>
        <div v-if="communicationSendText" class="result-block">
          <el-button
            class="copy-button"
            size="small"
            plain
            @click="copyResult(communicationSendText)"
          >
            复制
          </el-button>
          <pre class="result">{{ communicationSendText }}</pre>
        </div>
        <div v-if="communicationReceiveText" class="result-block">
          <el-button
            class="copy-button"
            size="small"
            plain
            @click="copyResult(communicationReceiveText)"
          >
            复制
          </el-button>
          <pre class="result">{{ communicationReceiveText }}</pre>
        </div>
      </div>
    </section>

    <section class="card" :class="{ 'is-collapsed': isCollapsed('token') }">
      <h2>
        Token 检测
        <button type="button" class="collapse-button" @click="toggleCard('token')">
          {{ isCollapsed('token') ? "展开" : "收起" }}
        </button>
      </h2>
      <div class="card-body">
        <p>客户端 token（iframe 获取，本地缓存）：{{ iframeTokenStatus }}</p>
        <div class="actions">
          <el-button type="primary" plain :loading="tokenLoading" @click="runTokenDetect">
            执行 Token 检测
          </el-button>
        </div>
        <div v-if="tokenResult" class="result-block">
          <el-button class="copy-button" size="small" plain @click="copyResult(tokenResult)">
            复制
          </el-button>
          <pre class="result">{{ tokenResult }}</pre>
        </div>
      </div>
    </section>

    <section class="card" :class="{ 'is-collapsed': isCollapsed('printclient') }">
      <h2>
        本地 CPMS 客户端（PrintClient）
        <button type="button" class="collapse-button" @click="toggleCard('printclient')">
          {{ isCollapsed('printclient') ? "展开" : "收起" }}
        </button>
      </h2>
      <div class="card-body">
        <p>运行目录（按进程名）：{{ printClient?.processDir || "未检测到运行中的 PrintClient" }}</p>
        <p>安装路径：{{ printClient?.dir || "未检测到" }}</p>
        <p>配置文件：{{ printClient?.configPath || "未检测到" }}</p>
        <p>WebsocketPort：{{ printClient?.websocketPort ?? "未知" }}</p>
        <p>ServerAddr（服务端域名）：{{ printClient?.serverAddr || "未检测到" }}</p>
        <p>CenterServerAddr：{{ printClient?.centerServerAddr || "未检测到" }}</p>
        <p>解析地址：{{ printClient?.socketUrl || "未知" }}</p>
        <div class="actions">
          <el-button :loading="printClientLoading" @click="refreshPrintClientInfo">
            刷新客户端信息
          </el-button>
        </div>
      </div>
    </section>

    <section class="card" :class="{ 'is-collapsed': isCollapsed('http-socket') }">
      <h2>
        请求检测
        <button type="button" class="collapse-button" @click="toggleCard('http-socket')">
          {{ isCollapsed('http-socket') ? "展开" : "收起" }}
        </button>
      </h2>
      <div class="card-body">
        <h3>HTTP</h3>
        <div class="actions">
          <el-button type="primary" :loading="httpLoading" @click="runHttpDetect">
            执行 HTTP 请求检测
          </el-button>
        </div>
        <div v-if="httpResult" class="result-block">
          <el-button class="copy-button" size="small" plain @click="copyResult(httpResult)">
            复制
          </el-button>
          <pre class="result">{{ httpResult }}</pre>
        </div>

        <h3>Socket（监听端，等待推送连接）</h3>
        <p>监听地址：{{ socketLinkUrl }}</p>
        <p>监听端口：{{ socketLinkPort }}</p>
        <p>监听状态：{{ socketStatusText }}</p>
        <p v-if="socketLink.message">最近说明：{{ socketLink.message }}</p>
        <div class="actions">
          <el-button type="success" plain @click="runSocketDetect">执行 Socket 请求检测</el-button>
          <el-button
            type="warning"
            plain
            :loading="socketReconnectLoading"
            @click="runSocketReconnect"
          >
            重启监听服务
          </el-button>
        </div>
        <div v-if="socketResult" class="result-block">
          <el-button class="copy-button" size="small" plain @click="copyResult(socketResult)">
            复制
          </el-button>
          <pre class="result">{{ socketResult }}</pre>
        </div>
      </div>
    </section>

    <section class="card" :class="{ 'is-collapsed': isCollapsed('autostart') }">
      <h2>
        开机自启动
        <button type="button" class="collapse-button" @click="toggleCard('autostart')">
          {{ isCollapsed('autostart') ? "展开" : "收起" }}
        </button>
      </h2>
      <div class="card-body">
        <div class="actions">
          <el-button :loading="autostartLoading" @click="toggleAutostart">
            {{ autostartEnabled ? "关闭自启动" : "开启自启动" }}
          </el-button>
        </div>
        <p>当前状态：{{ autostartEnabled ? "已开启" : "已关闭" }}</p>
      </div>
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
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--cpms-space-small);
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
  box-shadow: var(--cpms-shadow-sm);
}

.card.is-collapsed .card-body {
  display: none;
}

.card-body {
  display: grid;
  gap: var(--cpms-space-small);
}

.collapse-button {
  border: 0;
  padding: var(--cpms-space-xs) var(--cpms-space-small);
  border-radius: var(--cpms-radius-small);
  background: var(--cpms-color-bg-hover);
  color: var(--cpms-color-text-muted);
  font: inherit;
  font-size: var(--cpms-font-size-small);
  cursor: pointer;
}

.actions {
  display: flex;
  gap: var(--cpms-space-small);
  flex-wrap: wrap;
}

.result-block {
  position: relative;
}

.copy-button {
  position: absolute;
  top: var(--cpms-space-small);
  right: var(--cpms-space-small);
  z-index: 1;
}

.result {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  background: var(--cpms-color-bg-code);
  border-radius: var(--cpms-radius-small);
  padding: var(--cpms-space-small);
  padding-right: 60px;
  font-size: var(--cpms-font-size-small);
  box-shadow: var(--cpms-shadow-sm);
}
</style>
