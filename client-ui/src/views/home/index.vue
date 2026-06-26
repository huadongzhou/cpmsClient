<script setup lang="ts" name="HomeView">
import { CLIENT_EVENT, emitViewEvent } from '@/api/tauri/events'
import WindowFrame from '@/components/layout/WindowFrame.vue'
import { clearClientSessionDirectDeviceId, clearClientSessionServerAddress } from '@/api/tauri/desktop'
import { useIframeContainer } from '@/composables/useIframeContainer'
import { startHubClientMessageBridge, useIframePayloadBridge } from '@/bridges'
import { useRuntimeStore } from '@/stores/runtime'
import EntryView from '@/views/entry/index.vue'
import ExampleView from '@/views/example/index.vue'
import LogView from '@/views/logs/index.vue'

const IFRAME_LOAD_TIMEOUT_MS = 15000

const iframeRef = ref<HTMLIFrameElement>()
const iframeDomLoaded = ref(false)
const iframeLoadError = ref(false)
const iframeReloadKey = ref(0)
const exampleDrawerVisible = ref(false)
const drawerTab = ref('detect')
const pinned = ref(false)
const fullscreen = ref(false)
const { iframe } = useIframeContainer()
const runtimeStore = useRuntimeStore()
const { queryIframePayload } = useIframePayloadBridge(iframeRef)
const stopHubClientBridge = startHubClientMessageBridge(iframeRef)
const iframeSrc = computed(() => iframe.value.url || 'about:blank')
const showEntryPage = computed(() => iframe.value.state === 'idle' || !iframe.value.url)
const isIframeLoading = computed(() => {
  if (iframeLoadError.value || showEntryPage.value) {
    return false
  }

  return iframe.value.state === 'loaded' && Boolean(iframe.value.url) && !iframeDomLoaded.value
})
let iframeLoadTimer: ReturnType<typeof setTimeout> | undefined

function clearIframeLoadTimer() {
  if (iframeLoadTimer) {
    clearTimeout(iframeLoadTimer)
    iframeLoadTimer = undefined
  }
}

function startIframeLoadTimer() {
  clearIframeLoadTimer()
  iframeLoadTimer = setTimeout(() => {
    iframeLoadError.value = true
  }, IFRAME_LOAD_TIMEOUT_MS)
}

function resetIframeLoadState() {
  iframeDomLoaded.value = false
  iframeLoadError.value = false
  clearIframeLoadTimer()
}

watch(iframeSrc, (url) => {
  resetIframeLoadState()

  if (url && url !== 'about:blank') {
    startIframeLoadTimer()
  }
})

onMounted(() => {
  if (iframeSrc.value && iframeSrc.value !== 'about:blank' && !iframeDomLoaded.value) {
    startIframeLoadTimer()
  }
})

onBeforeUnmount(() => {
  clearIframeLoadTimer()
  stopHubClientBridge?.()
})

function handleIframeLoad() {
  iframeDomLoaded.value = true
  iframeLoadError.value = false
  clearIframeLoadTimer()
}

function retryIframeLoad() {
  resetIframeLoadState()
  iframeReloadKey.value += 1
  startIframeLoadTimer()
}

async function backToEntry() {
  resetIframeLoadState()
  runtimeStore.setIframeState({ state: 'idle', url: null, message: null, updatedAt: '' })
  try {
    await clearClientSessionServerAddress()
    await clearClientSessionDirectDeviceId()
  } catch (error) {
    console.warn('[home] 清空会话地址/设备 ID 失败', error)
  }
}

/** 固定按钮事件：固定/取消固定客户端窗口。 */
async function toggleWindowPin() {
  pinned.value = !pinned.value
  await emitViewEvent(pinned.value ? CLIENT_EVENT.WINDOW_PIN : CLIENT_EVENT.WINDOW_UNPIN)
}

/** 收起按钮事件：收起客户端窗口。 */
async function collapseWindow() {
  await emitViewEvent(CLIENT_EVENT.WINDOW_MINIMIZE)
}

/** 全屏按钮事件：全屏/退出全屏客户端窗口。 */
async function toggleWindowFullscreen() {
  fullscreen.value = !fullscreen.value
  await emitViewEvent(fullscreen.value ? CLIENT_EVENT.WINDOW_FULLSCREEN : CLIENT_EVENT.WINDOW_EXIT_FULLSCREEN, {
    fullscreen: fullscreen.value
  })
}

/** 关闭按钮事件：关闭客户端窗口（客户端隐藏到托盘）。 */
async function closeWindow() {
  await emitViewEvent(CLIENT_EVENT.WINDOW_CLOSE)
}
</script>

<template>
  <WindowFrame
    title="CPMS Client"
    icon="/tauri.svg"
    :pinned="pinned"
    :fullscreen="fullscreen"
    :loading="isIframeLoading"
    loading-text="正在加载页面…"
    :controls="
      showEntryPage ? ['pin', 'collapse', 'fullscreen', 'close'] : ['entry', 'pin', 'collapse', 'fullscreen', 'close']
    "
    @entry="backToEntry"
    @pin="toggleWindowPin"
    @collapse="collapseWindow"
    @fullscreen="toggleWindowFullscreen"
    @close="closeWindow"
  >
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
            <i class="el-icon-warning" />
          </div>
          <h2 class="error-title">页面加载失败</h2>
          <p class="error-desc">无法在预定时间内加载 iframe 页面，请检查网络或客户端配置后重试。</p>
          <div class="error-actions">
            <el-button type="primary" class="error-action-btn" @click="retryIframeLoad">
              <i class="el-icon-refresh-right" />
              重新加载
            </el-button>
            <el-button @click="backToEntry">
              <i class="el-icon-edit" />
              重新输入地址
            </el-button>
          </div>
        </div>
      </div>
    </template>

    <button type="button" class="example-trigger" @click="exampleDrawerVisible = true">
      <i class="el-icon-s-tools example-trigger-icon" />
    </button>

    <el-drawer
      :visible.sync="exampleDrawerVisible"
      direction="rtl"
      :with-header="false"
      :modal="true"
      size="80%"
      custom-class="debug-drawer"
    >
      <div class="debug-drawer-body">
        <header class="drawer-header">
          <h2 class="drawer-title">客户端调试</h2>
          <p class="drawer-description">检测客户端能力、查看运行日志</p>
        </header>
        <el-tabs v-model="drawerTab" class="drawer-tabs">
          <el-tab-pane label="能力检测" name="detect">
            <div class="tab-content">
              <div class="tab-scroll-area">
                <ExampleView :query-iframe-payload="queryIframePayload" />
              </div>
            </div>
          </el-tab-pane>
          <el-tab-pane label="客户端日志" name="logs">
            <div class="tab-content">
              <div class="tab-scroll-area">
                <LogView />
              </div>
            </div>
          </el-tab-pane>
        </el-tabs>
      </div>
    </el-drawer>
  </WindowFrame>
</template>

<style scoped>
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

.error-action-btn {
  min-width: 120px;
}

.error-actions .el-button i {
  margin-right: var(--cpms-space-1);
}

.example-trigger {
  position: fixed;
  right: var(--cpms-space-5);
  bottom: var(--cpms-space-5);
  z-index: 10;
  display: inline-grid;
  place-items: center;
  width: 48px;
  height: 48px;
  padding: 0;
  font-size: 20px;
  cursor: pointer;
  color: var(--cpms-color-primary);
  background: var(--cpms-color-surface);
  border: 1px solid var(--cpms-color-border);
  border-radius: var(--cpms-radius-full);
  box-shadow: var(--cpms-shadow-sm);
  transition:
    color var(--cpms-duration-base) var(--cpms-easing-base),
    background-color var(--cpms-duration-base) var(--cpms-easing-base),
    border-color var(--cpms-duration-base) var(--cpms-easing-base),
    box-shadow var(--cpms-duration-base) var(--cpms-easing-base);
}

.example-trigger:hover {
  color: var(--cpms-color-text-on-primary);
  background: var(--cpms-color-primary);
  border-color: var(--cpms-color-primary);
  box-shadow: var(--cpms-shadow-md);
}

.example-trigger-icon {
  font-size: 20px;
  color: currentcolor;
}

.debug-drawer {
  min-width: 560px;
  max-width: 900px;
}

.debug-drawer-body {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.drawer-header {
  padding: var(--cpms-space-5) var(--cpms-space-4) var(--cpms-space-2);
}

.drawer-title {
  margin: 0;
  font-size: var(--cpms-font-size-xl);
  font-weight: var(--cpms-font-weight-semibold);
  color: var(--cpms-color-text-primary);
}

.drawer-description {
  margin: var(--cpms-space-1) 0 0;
  font-size: var(--cpms-font-size-base);
  color: var(--cpms-color-text-secondary);
}

.drawer-tabs {
  display: flex;
  flex-direction: column;
  flex: 1 1 auto;
  min-height: 0;
  padding: 0 var(--cpms-space-4) var(--cpms-space-4);
}

.drawer-tabs ::v-deep .el-tabs__content {
  flex: 1 1 auto;
  min-height: 0;
  overflow: visible;
}

.drawer-tabs ::v-deep .el-tab-pane {
  height: 100%;
}

.tab-content {
  display: flex;
  flex-direction: column;
  flex: 1 1 auto;
  min-height: 0;
}

.tab-scroll-area {
  flex: 1 1 auto;
  min-height: 0;
  width: 100%;
  overflow: auto;
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
