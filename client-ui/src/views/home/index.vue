<script setup lang="ts" name="HomeView">
import { vLoading } from "element-plus";
import "element-plus/es/components/loading/style/css";
import { emitViewEvent } from "@/api/tauri/events";
import WindowHeaderBar from "@/components/layout/WindowHeaderBar.vue";
import { useIframeContainer } from "@/composables/useIframeContainer";
import { useIframePayloadBridge } from "@/composables/useIframePayloadBridge";
import ExampleView from "@/views/example/index.vue";
import LogView from "@/views/logs/index.vue";

const IFRAME_LOAD_TIMEOUT_MS = 15000;

const iframeRef = ref<HTMLIFrameElement>();
const iframeDomLoaded = ref(false);
const iframeLoadError = ref(false);
const exampleDrawerVisible = ref(false);
const drawerTab = ref("detect");
const pinned = ref(false);
const fullscreen = ref(false);
const { iframe, loadIframeContainer, loading } = useIframeContainer();
const { queryIframePayload } = useIframePayloadBridge(iframeRef);
const iframeSrc = computed(() => iframe.value.url || "about:blank");
const isIframeLoading = computed(() => {
  if (iframeLoadError.value) {
    return false;
  }

  return (
    loading.value ||
    iframe.value.state === "idle" ||
    iframe.value.state === "loading" ||
    (iframe.value.state === "loaded" && Boolean(iframe.value.url) && !iframeDomLoaded.value)
  );
});
let iframeLoadTimer: ReturnType<typeof setTimeout> | undefined;

function clearIframeLoadTimer() {
  if (iframeLoadTimer) {
    clearTimeout(iframeLoadTimer);
    iframeLoadTimer = undefined;
  }
}

function startIframeLoadTimer() {
  clearIframeLoadTimer();
  iframeLoadTimer = setTimeout(() => {
    iframeLoadError.value = true;
  }, IFRAME_LOAD_TIMEOUT_MS);
}

function resetIframeLoadState() {
  iframeDomLoaded.value = false;
  iframeLoadError.value = false;
  clearIframeLoadTimer();
}

watch(iframeSrc, (url) => {
  resetIframeLoadState();

  if (url && url !== "about:blank") {
    startIframeLoadTimer();
  }
});

onMounted(() => {
  if (iframe.value.state === "idle") {
    void loadIframeContainer();
  }

  if (iframeSrc.value && iframeSrc.value !== "about:blank" && !iframeDomLoaded.value) {
    startIframeLoadTimer();
  }
});

onBeforeUnmount(() => {
  clearIframeLoadTimer();
});

function handleIframeLoad() {
  iframeDomLoaded.value = true;
  iframeLoadError.value = false;
  clearIframeLoadTimer();
}

async function retryIframeLoad() {
  resetIframeLoadState();
  startIframeLoadTimer();
  await loadIframeContainer();
}

/** 固定按钮事件：固定/取消固定客户端窗口。 */
async function toggleWindowPin() {
  pinned.value = !pinned.value;
  await emitViewEvent(pinned.value ? "client.window.pin" : "client.window.unpin");
}

/** 收起按钮事件：收起客户端窗口。 */
async function collapseWindow() {
  await emitViewEvent("client.window.minimize");
}

/** 全屏按钮事件：全屏/退出全屏客户端窗口。 */
async function toggleWindowFullscreen() {
  fullscreen.value = !fullscreen.value;
  await emitViewEvent(
    fullscreen.value ? "client.window.fullscreen" : "client.window.exit-fullscreen",
    { fullscreen: fullscreen.value },
  );
}

/** 关闭按钮事件：关闭客户端窗口（客户端隐藏到托盘）。 */
async function closeWindow() {
  await emitViewEvent("client.window.close");
}
</script>

<template>
  <div class="app-window">
    <WindowHeaderBar
      title="CPMS Client"
      icon="/tauri.svg"
      :pinned="pinned"
      :fullscreen="fullscreen"
      @pin="toggleWindowPin"
      @collapse="collapseWindow"
      @fullscreen="toggleWindowFullscreen"
      @close="closeWindow"
    />
    <main v-loading="isIframeLoading" element-loading-text="正在加载业务页面" class="iframe-root">
      <iframe ref="iframeRef" :src="iframeSrc" class="business-iframe" @load="handleIframeLoad" />
      <div v-if="iframeLoadError" class="iframe-error">
        <el-alert
          type="error"
          title="业务页面加载失败"
          description="无法在预定时间内加载 iframe 业务页面，请检查网络或客户端配置后重试。"
          show-icon
          :closable="false"
        />
        <el-button type="primary" class="retry-button" @click="retryIframeLoad">重新加载</el-button>
      </div>
      <div
        v-if="exampleDrawerVisible"
        class="iframe-overlay"
        aria-hidden="true"
        @click="exampleDrawerVisible = false"
      />
      <el-button class="example-trigger" type="primary" @click="exampleDrawerVisible = true"
        >调试</el-button
      >
      <el-drawer
        v-model="exampleDrawerVisible"
        size="80%"
        :show-close="false"
        destroy-on-close
        class="debug-drawer"
      >
        <template #header>
          <WindowHeaderBar
            title="客户端调试"
            :controls="['close']"
            @close="exampleDrawerVisible = false"
          />
        </template>
        <el-tabs v-model="drawerTab" class="drawer-tabs">
          <el-tab-pane label="能力检测" name="detect">
            <ExampleView :query-iframe-payload="queryIframePayload" />
          </el-tab-pane>
          <el-tab-pane label="客户端日志" name="logs">
            <LogView />
          </el-tab-pane>
        </el-tabs>
      </el-drawer>
    </main>
  </div>
</template>

<style scoped>
.app-window {
  display: grid;
  grid-template-rows: auto 1fr;
  height: 100vh;
  background: var(--cpms-color-bg-panel);
}

.iframe-root {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: var(--cpms-color-bg-panel);
}

.business-iframe {
  width: 100%;
  height: 100%;
  border: 0;
  display: block;
}

.iframe-error {
  position: absolute;
  inset: var(--cpms-space-base);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--cpms-space-base);
  background: var(--cpms-color-bg-panel);
}

.retry-button {
  min-width: 120px;
}

.iframe-overlay {
  position: absolute;
  inset: 0;
  z-index: 5;
  background: rgba(0, 0, 0, 0.2);
}

.example-trigger {
  position: fixed;
  right: var(--cpms-space-xlarge);
  bottom: var(--cpms-space-xlarge);
  z-index: 10;
}

/* 抽屉外壳与窗口外壳统一：标题栏复用 WindowHeaderBar，页签/内容走令牌。 */
.debug-drawer :deep(.el-drawer__header) {
  margin: 0;
  padding: 0;
}

.debug-drawer :deep(.el-drawer__body) {
  padding: 0;
}

.debug-drawer :deep(.el-drawer) {
  border-radius: var(--cpms-radius-large) 0 0 var(--cpms-radius-large);
  box-shadow: var(--cpms-shadow-lg);
  min-width: 560px;
  max-width: 900px;
}

.drawer-tabs :deep(.el-tabs__header) {
  margin: 0;
  padding: 0 var(--cpms-space-base);
  border-bottom: 1px solid var(--cpms-color-border);
}

/* 抽屉内部组件统一占满宽度与高度。 */
.drawer-tabs,
.drawer-tabs :deep(.el-tabs__content),
.drawer-tabs :deep(.el-tab-pane) {
  width: 100%;
  height: 100%;
  box-sizing: border-box;
}

.drawer-tabs :deep(.el-tabs) {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.drawer-tabs :deep(.el-tabs__content) {
  flex: 1;
  min-height: 0;
}
</style>
