<script setup lang="ts" name="HomeView">
import { vLoading } from "element-plus";
import "element-plus/es/components/loading/style/css";
import { emitViewEvent } from "@/api/tauri/events";
import Icon from "@/components/common/Icon.vue";
import WindowHeaderBar from "@/components/layout/WindowHeaderBar.vue";
import { useIframeContainer } from "@/composables/useIframeContainer";
import { useIframePayloadBridge } from "@/composables/useIframePayloadBridge";
import { useRuntimeStore } from "@/stores/runtime";
import { startHubClientMessageBridge } from "@/utils/hubBridge";
import EntryView from "@/views/entry/index.vue";
import ExampleView from "@/views/example/index.vue";
import LogView from "@/views/logs/index.vue";

const IFRAME_LOAD_TIMEOUT_MS = 15000;

const iframeRef = ref<HTMLIFrameElement>();
const iframeDomLoaded = ref(false);
const iframeLoadError = ref(false);
const iframeReloadKey = ref(0);
const exampleDrawerVisible = ref(false);
const drawerTab = ref("detect");
const pinned = ref(false);
const fullscreen = ref(false);
const { iframe } = useIframeContainer();
const runtimeStore = useRuntimeStore();
const { queryIframePayload } = useIframePayloadBridge(iframeRef);
const stopHubClientBridge = startHubClientMessageBridge(iframeRef);
const iframeSrc = computed(() => iframe.value.url || "about:blank");
const showEntryPage = computed(() => iframe.value.state === "idle" || !iframe.value.url);
const isIframeLoading = computed(() => {
  if (iframeLoadError.value || showEntryPage.value) {
    return false;
  }

  return iframe.value.state === "loaded" && Boolean(iframe.value.url) && !iframeDomLoaded.value;
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
  if (iframeSrc.value && iframeSrc.value !== "about:blank" && !iframeDomLoaded.value) {
    startIframeLoadTimer();
  }
});

onBeforeUnmount(() => {
  clearIframeLoadTimer();
  stopHubClientBridge?.();
});

function handleIframeLoad() {
  iframeDomLoaded.value = true;
  iframeLoadError.value = false;
  clearIframeLoadTimer();
}

function retryIframeLoad() {
  resetIframeLoadState();
  iframeReloadKey.value += 1;
  startIframeLoadTimer();
}

function backToEntry() {
  resetIframeLoadState();
  runtimeStore.setIframeState({ state: "idle", url: null, message: null, updatedAt: "" });
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
    <main v-loading="isIframeLoading" element-loading-text="正在加载业务页面…" class="iframe-root">
      <EntryView v-if="showEntryPage" />
      <template v-else>
        <iframe
          :key="iframeReloadKey"
          ref="iframeRef"
          :src="iframeSrc"
          class="business-iframe"
          @load="handleIframeLoad"
        />
        <div v-if="iframeLoadError" class="iframe-error">
          <div class="error-card">
            <div class="error-icon">
              <Icon icon="solar:danger-triangle-bold" />
            </div>
            <h2 class="error-title">业务页面加载失败</h2>
            <p class="error-desc">
              无法在预定时间内加载 iframe 业务页面，请检查网络或客户端配置后重试。
            </p>
            <div class="error-actions">
              <el-button type="primary" class="retry-button" @click="retryIframeLoad">
                <template #icon>
                  <Icon icon="solar:refresh-square-bold" />
                </template>
                重新加载
              </el-button>
              <el-button @click="backToEntry">
                <template #icon>
                  <Icon icon="solar:pen-new-square-bold" />
                </template>
                重新输入地址
              </el-button>
            </div>
          </div>
        </div>
      </template>

      <div
        v-if="exampleDrawerVisible"
        class="iframe-overlay"
        aria-hidden="true"
        @click="exampleDrawerVisible = false"
      />

      <el-button class="example-trigger" type="primary" circle @click="exampleDrawerVisible = true">
        <Icon icon="solar:bug-minimalistic-bold" class="example-trigger-icon" />
      </el-button>

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
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  background: var(--cpms-color-bg-app);
  overflow: hidden;
}

.iframe-root {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1 1 auto;
  min-height: 0;
  width: 100%;
  background: var(--cpms-color-bg-app);
  overflow: hidden;
}

.business-iframe {
  flex: 1 1 auto;
  min-height: 0;
  width: 100%;
  border: 0;
  display: block;
  background: var(--cpms-color-bg-panel);
}

.iframe-error {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--cpms-space-5);
  background: var(--cpms-color-bg-app);
}

.error-card {
  max-width: 420px;
  width: 100%;
  padding: var(--cpms-space-8);
  text-align: center;
  background: var(--cpms-color-surface);
  border: 1px solid var(--cpms-color-border);
  border-radius: var(--cpms-radius-large);
  box-shadow: var(--cpms-shadow-lg);
}

.error-icon {
  display: inline-grid;
  place-items: center;
  width: 56px;
  height: 56px;
  margin-bottom: var(--cpms-space-4);
  font-size: 28px;
  color: var(--cpms-color-warning);
  background: var(--cpms-color-warning-bg);
  border-radius: var(--cpms-radius-full);
}

.error-title {
  margin: 0 0 var(--cpms-space-2);
  font-size: var(--cpms-font-size-xl);
  font-weight: var(--cpms-font-weight-semibold);
  color: var(--cpms-color-text-primary);
}

.error-desc {
  margin: 0 0 var(--cpms-space-5);
  font-size: var(--cpms-font-size-base);
  color: var(--cpms-color-text-secondary);
  line-height: var(--cpms-line-height-relaxed);
}

.error-actions {
  display: flex;
  justify-content: center;
  gap: var(--cpms-space-3);
  flex-wrap: wrap;
}

.retry-button {
  min-width: 120px;
}

.iframe-overlay {
  position: absolute;
  inset: 0;
  z-index: 5;
  background: var(--cpms-color-bg-overlay);
}

.example-trigger {
  position: fixed;
  right: var(--cpms-space-5);
  bottom: var(--cpms-space-5);
  z-index: 10;
  width: 48px;
  height: 48px;
  font-size: 20px;
  box-shadow: var(--cpms-shadow-md);
  transition:
    transform var(--cpms-duration-base) var(--cpms-easing-base),
    box-shadow var(--cpms-duration-base) var(--cpms-easing-base);
}

.example-trigger:hover {
  transform: translateY(-2px);
  box-shadow: var(--cpms-shadow-lg);
}

.example-trigger-icon {
  width: 20px;
  height: 20px;
  color: var(--cpms-color-text-on-primary);
  fill: currentcolor;
}

/* 抽屉外壳与窗口外壳统一：标题栏复用 WindowHeaderBar，页签/内容走令牌。 */
.debug-drawer :deep(.el-drawer__header) {
  margin: 0;
  padding: 0;
}

.debug-drawer :deep(.el-drawer__body) {
  padding: 0;
  overflow: hidden;
}

.debug-drawer :deep(.el-drawer) {
  border-radius: var(--cpms-radius-large) 0 0 var(--cpms-radius-large);
  box-shadow: var(--cpms-shadow-xl);
  min-width: 560px;
  max-width: 900px;
  background: var(--cpms-color-bg-app);
}

.drawer-tabs {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.drawer-tabs :deep(.el-tabs__header) {
  margin: 0;
  padding: 0 var(--cpms-space-base);
  border-bottom: 1px solid var(--cpms-color-border);
  background: var(--cpms-color-bg-panel);
}

.drawer-tabs :deep(.el-tabs__nav-wrap::after) {
  display: none;
}

.drawer-tabs :deep(.el-tabs__item) {
  font-weight: var(--cpms-font-weight-medium);
  color: var(--cpms-color-text-secondary);
}

.drawer-tabs :deep(.el-tabs__item.is-active) {
  color: var(--cpms-color-primary);
}

.drawer-tabs :deep(.el-tabs__active-bar) {
  background-color: var(--cpms-color-primary);
}

.drawer-tabs :deep(.el-tabs__content),
.drawer-tabs :deep(.el-tab-pane) {
  flex: 1 1 auto;
  min-height: 0;
  height: 100%;
}

@media (prefers-reduced-motion: reduce) {
  .example-trigger {
    transition: none;
  }

  .example-trigger:hover {
    transform: none;
  }
}
</style>
