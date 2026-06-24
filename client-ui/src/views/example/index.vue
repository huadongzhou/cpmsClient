<script setup lang="ts" name="ExampleView">
import { storeToRefs } from 'pinia'
import { emitViewEvent, listenClientEvent, listenClientSocketEvent } from '@/api/tauri/events'
import {
  clientHttpRequest,
  getAutostartEnabled,
  getPrintClientInfo,
  getSocketState,
  pushClientNotificationEvent,
  reconnectSocket,
  setAutostartEnabled
} from '@/api/tauri/desktop'
import { diagnoseDesktopNotification } from '@/api/tauri/notification'
import type { AppNotification } from '@/types/app/notification'
import type { ClientSocketStatePayload, PrintClientInfo } from '@/types/app/runtime'
import ErrorNotice from '@/components/common/ErrorNotice.vue'
import {
  Bell,
  ChevronDown as ArrowDownBold,
  ChevronRight as ArrowRightBold,
  CircleCheck as CircleCheckFilled,
  CircleX as CircleCloseFilled,
  Copy as CopyDocument,
  Cpu,
  Crosshair as Aim,
  KeyRound as Key,
  Link,
  LoaderCircle,
  MessageCircle as ChatDotRound,
  Monitor,
  Printer,
  RefreshCw as RefreshRight,
  Send as Promotion,
  Settings as Setting
} from '@lucide/vue'
import { useIframeContainer } from '@/composables/useIframeContainer'
import { useAppStore } from '@/stores/app'
import { useNetworkStore } from '@/stores/network'
import { useRuntimeStore } from '@/stores/runtime'
import { useTaskStore } from '@/stores/task'
import type { IframePayloadBridgeResult } from '@/composables/useIframePayloadBridge'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { message } from '@/services/ui/message'

const props = defineProps<{
  queryIframePayload?: (reason?: string) => Promise<IframePayloadBridgeResult>
}>()

const appStore = useAppStore()
const networkStore = useNetworkStore()
const runtimeStore = useRuntimeStore()
const taskStore = useTaskStore()
const { isOnline } = storeToRefs(networkStore)
const { iframe, loadIframeContainer, loading } = useIframeContainer()
const autostartEnabled = ref(false)
const autostartLoading = ref(false)
const notifyResult = ref('')
const communicationInput = ref('view -> client: hello cpms')
const communicationSendText = ref('')
const communicationReceiveText = ref('')
const tokenResult = ref('')
const tokenLoading = ref(false)
const httpResult = ref('')
const httpLoading = ref(false)
const socketResult = ref('')
const socketReconnectLoading = ref(false)
const socketLink = ref<ClientSocketStatePayload>({
  url: '',
  port: null,
  status: '',
  updatedAt: ''
})
const printClient = ref<PrintClientInfo>()
const printClientLoading = ref(false)
let unlistenClientEvent: UnlistenFn | undefined
let unlistenClientSocket: UnlistenFn | undefined

const SOCKET_STATUS_TEXT: Record<string, string> = {
  '': '未初始化',
  binding: '绑定中',
  listening: '监听中（等待推送连接）',
  failed: '监听失败'
}
const socketStatusText = computed(() => SOCKET_STATUS_TEXT[socketLink.value.status] ?? socketLink.value.status)
const socketLinkUrl = computed(() => socketLink.value.url || socketEndpoint.value)
const socketLinkPort = computed(() => socketLink.value.port ?? '未知')

const pageAddress = computed(() => window.location.href)
const iframeAddress = computed(() => iframe.value.url || 'about:blank')
const collapsedCards = ref<Set<string>>(new Set())

function isCollapsed(key: string) {
  return collapsedCards.value.has(key)
}

function toggleCard(key: string) {
  const next = new Set(collapsedCards.value)
  if (next.has(key)) {
    next.delete(key)
  } else {
    next.add(key)
  }
  collapsedCards.value = next
}

async function copyResult(text: string) {
  if (!text) {
    message.info('暂无可复制内容')
    return
  }

  await navigator.clipboard.writeText(text)
  message.success('结果已复制')
}

// 客户端只有一个 token：经 iframe 获取后本地缓存，等待拉取/推送更新。
const iframeTokenStatus = computed(() => (runtimeStore.iframeToken ? '存在' : '不存在'))
const socketEndpoint = computed(() => toSocketEndpoint(appStore.config.localServiceUrl))
const latestSocketTask = computed(() => taskStore.todoTasks[0])

onMounted(async () => {
  try {
    autostartEnabled.value = await getAutostartEnabled()
  } catch {
    autostartEnabled.value = false
  }

  unlistenClientEvent = await listenClientEvent((payload) => {
    communicationReceiveText.value = ['[Client -> View]', JSON.stringify(payload, null, 2)].join('\n')
  })

  try {
    socketLink.value = await getSocketState()
  } catch {
    // 非 Tauri 环境或尚未初始化时忽略。
  }
  unlistenClientSocket = await listenClientSocketEvent((payload) => {
    socketLink.value = payload
  })

  await refreshPrintClientInfo()
})

async function refreshPrintClientInfo() {
  printClientLoading.value = true
  try {
    printClient.value = await getPrintClientInfo()
  } catch {
    printClient.value = undefined
  } finally {
    printClientLoading.value = false
  }
}

onBeforeUnmount(() => {
  unlistenClientEvent?.()
  unlistenClientSocket?.()
})

async function runNotificationDetect() {
  const payload = {
    type: 'info' as const,
    title: '通知检测',
    message: '这是一条客户端调试通知，用于验证通知窗口是否正常弹出。',
    durationMs: 5000
  }

  const notification: AppNotification = {
    id: createDiagnosticNotificationId(),
    type: payload.type,
    title: payload.title,
    message: payload.message,
    createdAt: new Date().toISOString(),
    durationMs: payload.durationMs
  }
  let clientEventForwarded = false
  let clientEventError: string | undefined
  const desktopWindowDiagnostic = await diagnoseDesktopNotification(notification)
  const blockedStep = desktopWindowDiagnostic.steps.find((step) => step.status !== 'ok')
  const desktopWindowError = blockedStep
    ? [blockedStep.name, blockedStep.status, blockedStep.detail].filter(Boolean).join(': ')
    : undefined

  if (desktopWindowDiagnostic.ok) {
    message.success('测试通知已发送')
  } else {
    message.error(`通知弹窗未确认：${desktopWindowError ?? 'unknown'}`)
  }

  try {
    clientEventForwarded = await pushClientNotificationEvent(payload)
  } catch (error) {
    clientEventError = error instanceof Error ? error.message : '客户端通知回推失败'
  }

  notifyResult.value = [
    '[Notify Payload]',
    JSON.stringify(
      {
        payload,
        notificationId: notification.id,
        frontendQueue: false,
        frontendQueueDetail: 'skipped to avoid duplicate desktop dispatch during diagnostics',
        desktopWindowShown: desktopWindowDiagnostic.ok,
        desktopWindowError,
        desktopWindowDiagnostic,
        clientEventForwarded,
        clientEventError
      },
      null,
      2
    )
  ].join('\n')
}

function createDiagnosticNotificationId() {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID()
  }

  return `notification-detect-${Date.now()}-${Math.random().toString(16).slice(2)}`
}

async function runCommunicationDetect() {
  const payload = {
    text: communicationInput.value,
    channel: 'cpms:view-to-client',
    at: new Date().toISOString()
  }

  await emitViewEvent('example.communication.detect', payload)
  communicationSendText.value = [
    '[View -> Client]',
    JSON.stringify(
      {
        event: 'example.communication.detect',
        payload
      },
      null,
      2
    )
  ].join('\n')
}

async function runTokenDetect() {
  tokenLoading.value = true
  tokenResult.value = ''

  try {
    const bridgeResult = await props.queryIframePayload?.('token-detect')
    const iframePayloadToken = bridgeResult?.token

    runtimeStore.setIframeToken(iframePayloadToken || '')

    // 客户端只有一个 token：来自 iframe，获取后即本地缓存（不存在 localStorage/store 等多份存储）。
    tokenResult.value = [
      '[Token Detect]',
      JSON.stringify(
        {
          token: {
            exists: Boolean(iframePayloadToken),
            value: iframePayloadToken || '',
            source: 'iframe（获取后本地缓存）'
          },
          iframePayloadBridge: normalizeBridgeResult(bridgeResult),
          iframeUrl: iframeAddress.value,
          checkedAt: new Date().toISOString()
        },
        null,
        2
      )
    ].join('\n')
  } finally {
    tokenLoading.value = false
  }
}

async function toggleAutostart() {
  autostartLoading.value = true
  try {
    autostartEnabled.value = await setAutostartEnabled(!autostartEnabled.value)
  } finally {
    autostartLoading.value = false
  }
}

async function runHttpDetect() {
  httpLoading.value = true
  httpResult.value = ''
  try {
    const result = await clientHttpRequest({
      method: 'GET',
      url: appStore.config.cpmsBaseUrl,
      timeoutMs: 4000
    })
    httpResult.value = [
      `[HTTP Request] GET ${appStore.config.cpmsBaseUrl}`,
      '[HTTP Response]',
      JSON.stringify(result, null, 2)
    ].join('\n')
  } catch (error) {
    httpResult.value = [
      `[HTTP Request] GET ${appStore.config.cpmsBaseUrl}`,
      '[HTTP Error]',
      error instanceof Error ? error.message : '代理请求失败'
    ].join('\n')
  } finally {
    httpLoading.value = false
  }
}

function runSocketDetect() {
  if (latestSocketTask.value) {
    socketResult.value = [
      `[Socket Connected] ${socketEndpoint.value}`,
      '[Socket -> Client -> View Payload]',
      JSON.stringify(latestSocketTask.value, null, 2)
    ].join('\n')
    return
  }

  socketResult.value = [
    `[Socket Connected] ${socketEndpoint.value}`,
    '[Socket Simulated Payload]',
    JSON.stringify(
      {
        taskId: 'mock-task-001',
        title: 'mock socket task',
        status: 'running',
        at: new Date().toISOString()
      },
      null,
      2
    )
  ].join('\n')
}

async function runSocketReconnect() {
  socketReconnectLoading.value = true
  try {
    await reconnectSocket()
    socketResult.value = [
      `[Socket Restart] ${socketLinkUrl.value}`,
      '已请求客户端重启本地 socket 监听服务，可在「客户端日志」的「任务 / Socket」类别查看结果。'
    ].join('\n')
  } catch (error) {
    socketResult.value = ['[Socket Reconnect Error]', error instanceof Error ? error.message : '重连请求失败'].join(
      '\n'
    )
  } finally {
    socketReconnectLoading.value = false
  }
}

function toSocketEndpoint(baseUrl: string) {
  const normalized = baseUrl.replace(/\/$/, '')

  if (normalized.endsWith('/ws/task')) {
    return normalized
  }

  if (normalized.startsWith('ws://') || normalized.startsWith('wss://')) {
    return `${normalized}/ws/task`
  }

  if (normalized.startsWith('https://')) {
    return `${normalized.replace('https://', 'wss://')}/ws/task`
  }

  return `${normalized.replace('http://', 'ws://')}/ws/task`
}

function normalizeBridgeResult(result?: IframePayloadBridgeResult) {
  if (!result) {
    return {
      ok: false,
      error: 'useIframePayloadBridge unavailable'
    }
  }

  return {
    id: result.id,
    ok: result.ok,
    reason: result.reason,
    error: result.error,
    token: result.token
  }
}

// 内联结果块子组件，避免额外文件依赖。
const ResultBlock = defineComponent({
  props: {
    text: { type: String, required: true },
    title: { type: String, default: '' }
  },
  emits: ['copy'],
  setup(props, { emit }) {
    return () =>
      h('div', { class: 'result-block' }, [
        props.title ? h('h4', { class: 'result-title' }, props.title) : null,
        h(
          'button',
          {
            class: 'result-copy',
            type: 'button',
            onClick: () => emit('copy')
          },
          [h(CopyDocument, { class: 'result-copy-icon' }), h('span', '复制')]
        ),
        h('pre', { class: 'result' }, props.text)
      ])
  }
})
</script>

<template>
  <main class="example">
    <ErrorNotice />

    <div class="grid grid-cols-[repeat(auto-fill,minmax(360px,1fr))] gap-3 content-start">
      <section class="card" :class="{ 'is-collapsed': isCollapsed('status') }">
        <header class="flex items-center justify-between gap-2">
          <div class="card-title">
            <Monitor class="card-icon" />
            <span>状态</span>
          </div>
          <button type="button" class="collapse-button" @click="toggleCard('status')">
            <ArrowDownBold v-if="!isCollapsed('status')" />
            <ArrowRightBold v-else />
          </button>
        </header>
        <div class="card-body">
          <div class="flex flex-col gap-2">
            <div class="status-row">
              <span class="status-label">页面地址</span>
              <span class="status-value" :title="pageAddress">{{ pageAddress }}</span>
            </div>
            <div class="status-row">
              <span class="status-label">iframe 地址</span>
              <span class="status-value" :title="iframeAddress">{{ iframeAddress }}</span>
            </div>
            <div class="status-row">
              <span class="status-label">开机自启动</span>
              <span class="status-value">
                <Badge :variant="autostartEnabled ? 'default' : 'secondary'">
                  {{ autostartEnabled ? '已开启' : '已关闭' }}
                </Badge>
              </span>
            </div>
            <div class="status-row">
              <span class="status-label">网络状态</span>
              <span class="status-value">
                <Badge :variant="isOnline ? 'default' : 'destructive'">
                  {{ isOnline ? 'online' : 'offline' }}
                </Badge>
              </span>
            </div>
          </div>
          <div class="flex gap-2 flex-wrap">
            <Button :disabled="loading" @click="loadIframeContainer">
              <LoaderCircle v-if="loading" class="animate-spin" />
              <RefreshRight v-else />
              刷新 iframe 地址
            </Button>
          </div>
        </div>
      </section>

      <section class="card" :class="{ 'is-collapsed': isCollapsed('notification') }">
        <header class="flex items-center justify-between gap-2">
          <div class="card-title">
            <Bell class="card-icon" />
            <span>通知检测</span>
          </div>
          <button type="button" class="collapse-button" @click="toggleCard('notification')">
            <ArrowDownBold v-if="!isCollapsed('notification')" />
            <ArrowRightBold v-else />
          </button>
        </header>
        <div class="card-body">
          <p class="card-desc">向客户端发送一条测试通知，验证桌面通知子窗口是否正常弹出。</p>
          <div class="flex gap-2 flex-wrap">
            <Button variant="outline" @click="runNotificationDetect">
              <Bell />
              发送通知
            </Button>
          </div>
          <ResultBlock v-if="notifyResult" :text="notifyResult" @copy="copyResult(notifyResult)" />
        </div>
      </section>

      <section class="card" :class="{ 'is-collapsed': isCollapsed('communication') }">
        <header class="flex items-center justify-between gap-2">
          <div class="card-title">
            <Link class="card-icon" />
            <span>通信检测</span>
          </div>
          <button type="button" class="collapse-button" @click="toggleCard('communication')">
            <ArrowDownBold v-if="!isCollapsed('communication')" />
            <ArrowRightBold v-else />
          </button>
        </header>
        <div class="card-body">
          <Input v-model="communicationInput" placeholder="输入模拟传输文本" />
          <div class="flex gap-2 flex-wrap">
            <Button variant="outline" @click="runCommunicationDetect">
              <Promotion />
              执行通信检测
            </Button>
          </div>
          <ResultBlock
            v-if="communicationSendText"
            title="发送内容"
            :text="communicationSendText"
            @copy="copyResult(communicationSendText)"
          />
          <ResultBlock
            v-if="communicationReceiveText"
            title="接收内容"
            :text="communicationReceiveText"
            @copy="copyResult(communicationReceiveText)"
          />
        </div>
      </section>

      <section class="card" :class="{ 'is-collapsed': isCollapsed('token') }">
        <header class="flex items-center justify-between gap-2">
          <div class="card-title">
            <Key class="card-icon" />
            <span>Token 检测</span>
          </div>
          <button type="button" class="collapse-button" @click="toggleCard('token')">
            <ArrowDownBold v-if="!isCollapsed('token')" />
            <ArrowRightBold v-else />
          </button>
        </header>
        <div class="card-body">
          <div class="status-row">
            <span class="status-label">客户端 token</span>
            <span class="status-value">
              <Badge :variant="runtimeStore.iframeToken ? 'default' : 'secondary'">
                {{ iframeTokenStatus }}
              </Badge>
            </span>
          </div>
          <p class="card-desc">从 iframe 获取 token 并写入会话内存。</p>
          <div class="flex gap-2 flex-wrap">
            <Button variant="outline" :disabled="tokenLoading" @click="runTokenDetect">
              <LoaderCircle v-if="tokenLoading" class="animate-spin" />
              <RefreshRight v-else />
              执行 Token 检测
            </Button>
          </div>
          <ResultBlock v-if="tokenResult" :text="tokenResult" @copy="copyResult(tokenResult)" />
        </div>
      </section>

      <section class="card is-wide" :class="{ 'is-collapsed': isCollapsed('printclient') }">
        <header class="flex items-center justify-between gap-2">
          <div class="card-title">
            <Printer class="card-icon" />
            <span>本地 CPMS 客户端（PrintClient）</span>
          </div>
          <button type="button" class="collapse-button" @click="toggleCard('printclient')">
            <ArrowDownBold v-if="!isCollapsed('printclient')" />
            <ArrowRightBold v-else />
          </button>
        </header>
        <div class="card-body">
          <div class="flex flex-col gap-2">
            <div class="status-row">
              <span class="status-label">运行目录</span>
              <span class="status-value">{{ printClient?.processDir || '未检测到运行中的 PrintClient' }}</span>
            </div>
            <div class="status-row">
              <span class="status-label">安装路径</span>
              <span class="status-value">{{ printClient?.dir || '未检测到' }}</span>
            </div>
            <div class="status-row">
              <span class="status-label">配置文件</span>
              <span class="status-value">{{ printClient?.configPath || '未检测到' }}</span>
            </div>
            <div class="status-row">
              <span class="status-label">WebsocketPort</span>
              <span class="status-value">{{ printClient?.websocketPort ?? '未知' }}</span>
            </div>
            <div class="status-row">
              <span class="status-label">ServerAddr</span>
              <span class="status-value">{{ printClient?.serverAddr || '未检测到' }}</span>
            </div>
            <div class="status-row">
              <span class="status-label">CenterServerAddr</span>
              <span class="status-value">{{ printClient?.centerServerAddr || '未检测到' }}</span>
            </div>
            <div class="status-row">
              <span class="status-label">解析地址</span>
              <span class="status-value">{{ printClient?.socketUrl || '未知' }}</span>
            </div>
          </div>
          <div class="flex gap-2 flex-wrap">
            <Button :disabled="printClientLoading" @click="refreshPrintClientInfo">
              <LoaderCircle v-if="printClientLoading" class="animate-spin" />
              <RefreshRight v-else />
              刷新客户端信息
            </Button>
          </div>
        </div>
      </section>

      <section class="card is-wide" :class="{ 'is-collapsed': isCollapsed('http-socket') }">
        <header class="flex items-center justify-between gap-2">
          <div class="card-title">
            <Cpu class="card-icon" />
            <span>请求检测</span>
          </div>
          <button type="button" class="collapse-button" @click="toggleCard('http-socket')">
            <ArrowDownBold v-if="!isCollapsed('http-socket')" />
            <ArrowRightBold v-else />
          </button>
        </header>
        <div class="card-body">
          <div class="flex flex-col gap-3">
            <h3 class="subsection-title">
              <Link class="subsection-icon" />
              HTTP
            </h3>
            <p class="card-desc">向 CPMS 基础地址发送一次 GET 代理请求，检测客户端 HTTP 代理能力。</p>
            <div class="flex gap-2 flex-wrap">
              <Button variant="default" :disabled="httpLoading" @click="runHttpDetect">
                <LoaderCircle v-if="httpLoading" class="animate-spin" />
                <Promotion v-else />
                执行 HTTP 请求检测
              </Button>
            </div>
            <ResultBlock v-if="httpResult" :text="httpResult" @copy="copyResult(httpResult)" />
          </div>

          <Separator />

          <div class="flex flex-col gap-3">
            <h3 class="subsection-title">
              <ChatDotRound class="subsection-icon" />
              Socket（监听端，等待推送连接）
            </h3>
            <div class="flex flex-col gap-2">
              <div class="status-row">
                <span class="status-label">监听地址</span>
                <span class="status-value">{{ socketLinkUrl }}</span>
              </div>
              <div class="status-row">
                <span class="status-label">监听端口</span>
                <span class="status-value">{{ socketLinkPort }}</span>
              </div>
              <div class="status-row">
                <span class="status-label">监听状态</span>
                <span class="status-value">
                  <Badge
                    :variant="
                      socketLink.status === 'listening' ? 'default' : socketLink.status === 'failed' ? 'destructive' : 'secondary'
                    "
                  >
                    {{ socketStatusText }}
                  </Badge>
                </span>
              </div>
              <div v-if="socketLink.message" class="status-row">
                <span class="status-label">最近说明</span>
                <span class="status-value">{{ socketLink.message }}</span>
              </div>
            </div>
            <div class="flex gap-2 flex-wrap">
              <Button variant="outline" @click="runSocketDetect">
                <Aim />
                执行 Socket 请求检测
              </Button>
              <Button variant="outline" :disabled="socketReconnectLoading" @click="runSocketReconnect">
                <LoaderCircle v-if="socketReconnectLoading" class="animate-spin" />
                <RefreshRight v-else />
                重启监听服务
              </Button>
            </div>
            <ResultBlock v-if="socketResult" :text="socketResult" @copy="copyResult(socketResult)" />
          </div>
        </div>
      </section>

      <section class="card" :class="{ 'is-collapsed': isCollapsed('autostart') }">
        <header class="flex items-center justify-between gap-2">
          <div class="card-title">
            <Setting class="card-icon" />
            <span>开机自启动</span>
          </div>
          <button type="button" class="collapse-button" @click="toggleCard('autostart')">
            <ArrowDownBold v-if="!isCollapsed('autostart')" />
            <ArrowRightBold v-else />
          </button>
        </header>
        <div class="card-body">
          <div class="status-row">
            <span class="status-label">当前状态</span>
            <span class="status-value">
              <Badge :variant="autostartEnabled ? 'default' : 'secondary'">
                {{ autostartEnabled ? '已开启' : '已关闭' }}
              </Badge>
            </span>
          </div>
          <div class="flex gap-2 flex-wrap">
            <Button :disabled="autostartLoading" @click="toggleAutostart">
              <LoaderCircle v-if="autostartLoading" class="animate-spin" />
              <CircleCheckFilled v-else-if="!autostartEnabled" />
              <CircleCloseFilled v-else />
              {{ autostartEnabled ? '关闭自启动' : '开启自启动' }}
            </Button>
          </div>
        </div>
      </section>
    </div>
  </main>
</template>

<style scoped>
.example {
  display: flex;
  flex-direction: column;
  padding: var(--cpms-space-base);
}

.card {
  display: flex;
  flex-direction: column;
  background: var(--cpms-color-surface);
  border: 1px solid var(--cpms-color-border);
  border-radius: var(--cpms-radius-panel);
  padding: var(--cpms-space-base);
  box-shadow: var(--cpms-shadow-xs);
  transition: box-shadow var(--cpms-duration-base) var(--cpms-easing-base);
}

.card:hover {
  box-shadow: var(--cpms-shadow-sm);
}

.card.is-wide {
  grid-column: 1 / -1;
}

.card.is-collapsed .card-body {
  display: none;
}

.card-title {
  display: flex;
  align-items: center;
  gap: var(--cpms-space-2);
  margin: 0;
  font-size: var(--cpms-font-size-base);
  font-weight: var(--cpms-font-weight-semibold);
  color: var(--cpms-color-text-primary);
}

.card-icon {
  width: 18px;
  height: 18px;
  color: var(--cpms-color-primary);
}

.collapse-button {
  display: grid;
  place-items: center;
  width: 44px;
  height: 44px;
  padding: 0;
  border: 0;
  border-radius: var(--cpms-radius-small);
  background: transparent;
  color: var(--cpms-color-text-muted);
  font-size: 14px;
  cursor: pointer;
  transition:
    color var(--cpms-duration-fast) var(--cpms-easing-base),
    background-color var(--cpms-duration-fast) var(--cpms-easing-base);
}

.collapse-button:hover {
  background: var(--cpms-color-bg-hover);
  color: var(--cpms-color-text-primary);
}

.collapse-button:focus-visible {
  outline: 2px solid var(--cpms-color-primary);
  outline-offset: 2px;
}

.card-body {
  display: flex;
  flex-direction: column;
  gap: var(--cpms-space-3);
  margin-top: var(--cpms-space-3);
}

.card-desc {
  margin: 0;
  font-size: var(--cpms-font-size-small);
  color: var(--cpms-color-text-muted);
  line-height: var(--cpms-line-height-small);
}

.status-row {
  display: flex;
  align-items: baseline;
  gap: var(--cpms-space-2);
  font-size: var(--cpms-font-size-small);
}

.status-label {
  flex: none;
  width: 96px;
  color: var(--cpms-color-text-muted);
}

.status-value {
  min-width: 0;
  flex: 1 1 auto;
  color: var(--cpms-color-text-secondary);
  word-break: break-word;
}

.subsection-title {
  display: flex;
  align-items: center;
  gap: var(--cpms-space-2);
  margin: 0;
  font-size: var(--cpms-font-size-base);
  font-weight: var(--cpms-font-weight-semibold);
  color: var(--cpms-color-text-secondary);
}

.subsection-icon {
  width: 16px;
  height: 16px;
  color: var(--cpms-color-primary);
}

.result-block {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: var(--cpms-space-1);
}

.result-title {
  margin: 0;
  font-size: var(--cpms-font-size-small);
  font-weight: var(--cpms-font-weight-medium);
  color: var(--cpms-color-text-muted);
}

.result-copy {
  position: absolute;
  top: var(--cpms-space-2);
  right: var(--cpms-space-2);
  z-index: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--cpms-space-1);
  min-height: 32px;
  padding: 4px 8px;
  font-size: var(--cpms-font-size-xs);
  color: var(--cpms-color-text-secondary);
  background: var(--cpms-color-surface);
  border: 1px solid var(--cpms-color-border);
  border-radius: var(--cpms-radius-small);
  cursor: pointer;
  transition:
    color var(--cpms-duration-fast) var(--cpms-easing-base),
    border-color var(--cpms-duration-fast) var(--cpms-easing-base),
    background-color var(--cpms-duration-fast) var(--cpms-easing-base);
}

.result-copy:hover {
  color: var(--cpms-color-primary-text);
  border-color: var(--cpms-color-primary-border);
  background: var(--cpms-color-primary-bg);
}

.result-copy:focus-visible {
  outline: 2px solid var(--cpms-color-primary);
  outline-offset: 2px;
}

.result-copy-icon {
  width: 12px;
  height: 12px;
}

.result {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  background: var(--cpms-color-bg-code);
  border: 1px solid var(--cpms-color-border);
  border-radius: var(--cpms-radius-small);
  padding: var(--cpms-space-3);
  padding-right: 72px;
  font-family: var(--cpms-font-family-mono);
  font-size: var(--cpms-font-size-small);
  color: var(--cpms-color-text-secondary);
  line-height: var(--cpms-line-height-relaxed);
  max-height: 280px;
  overflow: auto;
}

@media (prefers-reduced-motion: reduce) {
  .card,
  .collapse-button,
  .result-copy {
    transition: none;
  }
}
</style>
