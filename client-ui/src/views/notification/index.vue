<script setup lang="ts" name="NotificationView">
import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import {
  DESKTOP_NOTIFICATION_ACK_EVENT,
  DESKTOP_NOTIFICATION_PUSH_EVENT,
  DESKTOP_NOTIFICATION_READY_EVENT,
  type DesktopNotificationAckPayload
} from '@/api/tauri/notification'
import type { AppNotification, AppNotificationType } from '@/types/app/notification'

interface NotificationMeta {
  label: string
  icon: string
}

const META_BY_TYPE: Record<AppNotificationType, NotificationMeta> = {
  info: {
    label: '通知',
    icon: 'el-icon-info'
  },
  success: {
    label: '成功',
    icon: 'el-icon-circle-check'
  },
  warning: {
    label: '警告',
    icon: 'el-icon-warning'
  },
  error: {
    label: '错误',
    icon: 'el-icon-circle-close'
  }
}

const currentWindow = getCurrentWindow()
const currentNotification = ref<AppNotification>()
let unlistenPush: UnlistenFn | undefined

onMounted(async () => {
  unlistenPush = await listen<AppNotification>(DESKTOP_NOTIFICATION_PUSH_EVENT, (event) => {
    void pushNotification(event.payload)
  })
  await emit(DESKTOP_NOTIFICATION_READY_EVENT)
})

onBeforeUnmount(() => {
  unlistenPush?.()
})

async function pushNotification(notification: AppNotification) {
  currentNotification.value = notification
  await emitNotificationAck(notification, 'received')

  try {
    await currentWindow.show()
    await emitNotificationAck(notification, 'shown')
  } catch (error) {
    await emitNotificationAck(notification, 'show-error', error instanceof Error ? error.message : String(error))
  }
}

function closeNotification() {
  currentNotification.value = undefined
  void currentWindow.hide()
}

async function emitNotificationAck(
  notification: AppNotification,
  stage: DesktopNotificationAckPayload['stage'],
  error?: string
) {
  await emit(DESKTOP_NOTIFICATION_ACK_EVENT, {
    id: notification.id,
    stage,
    error,
    time: Date.now()
  } satisfies DesktopNotificationAckPayload)
}

const notificationMeta = computed(() => {
  if (!currentNotification.value) {
    return META_BY_TYPE.info
  }

  return META_BY_TYPE[currentNotification.value.type] ?? META_BY_TYPE.info
})

const notificationTitle = computed(() => {
  const sourceText = [currentNotification.value?.title ?? '', currentNotification.value?.message ?? ''].join(' ')

  if (/打印|作业|任务|PrintClient|Socket/i.test(sourceText)) {
    return '打印任务通知'
  }

  return notificationMeta.value.label
})
const notificationMessage = computed(() => currentNotification.value?.message?.trim() || '无消息内容')
</script>

<template>
  <main class="notification-host" aria-live="polite">
    <section v-if="currentNotification" class="notification-frame">
      <header class="notification-headerbar" data-tauri-drag-region>
        <div class="notification-headerbar-title" data-tauri-drag-region>
          <i class="notification-headerbar-icon" :class="notificationMeta.icon" aria-hidden="true" />
          <strong class="notification-headerbar-text" data-tauri-drag-region>{{ notificationTitle }}</strong>
        </div>
        <button
          type="button"
          class="notification-headerbar-close"
          title="关闭"
          aria-label="关闭"
          @click="closeNotification"
        >
          <i class="el-icon-close" aria-hidden="true" />
        </button>
      </header>
      <div class="notification-frame-body">
        <p class="notification-message">{{ notificationMessage }}</p>
      </div>
    </section>
  </main>
</template>

<style scoped>
.notification-host {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.notification-frame {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.notification-headerbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--cpms-space-base);
  flex: none;
  height: var(--cpms-headerbar-height);
  padding-left: var(--cpms-space-base);
  background: var(--cpms-color-bg-panel);
  border-bottom: 1px solid var(--cpms-color-border);
  color: var(--cpms-color-text-primary);
  user-select: none;
}

.notification-headerbar-title {
  display: flex;
  align-items: center;
  gap: var(--cpms-space-2);
  min-width: 0;
}

.notification-headerbar-icon {
  flex: none;
  font-size: var(--cpms-font-size-base);
  color: var(--cpms-color-text-secondary);
}

.notification-headerbar-text {
  font-size: var(--cpms-font-size-base);
  font-weight: var(--cpms-font-weight-semibold);
  line-height: var(--cpms-line-height-small);
  color: var(--cpms-color-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.notification-headerbar-close {
  display: inline-grid;
  place-items: center;
  width: 35px;
  height: 35px;
  flex: none;
  padding: 8px;
  border: none;
  background: transparent;
  color: var(--cpms-color-text-secondary);
  cursor: pointer;
  box-sizing: border-box;
  transition:
    color var(--cpms-duration-fast) var(--cpms-easing-base),
    background-color var(--cpms-duration-fast) var(--cpms-easing-base),
    transform var(--cpms-duration-fast) var(--cpms-easing-base);
}

.notification-headerbar-close:hover {
  background: var(--cpms-color-danger);
  color: var(--cpms-color-text-on-primary);
}

.notification-headerbar-close:active {
  transform: scale(0.96);
}

.notification-frame-body {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1 1 auto;
  min-height: 0;
  width: 100%;
  background: var(--cpms-color-bg-app);
  overflow: hidden;
}

.notification-message {
  flex: 1 1 auto;
  min-height: 0;
  margin: 0;
  padding: var(--cpms-space-3);
  background: var(--cpms-color-surface);
  color: var(--cpms-color-text-secondary);
  font-size: var(--cpms-font-size-base);
  line-height: var(--cpms-line-height-relaxed);
  white-space: pre-wrap;
  word-break: break-word;
  overflow: auto;
}

@media (prefers-reduced-motion: reduce) {
  .notification-headerbar-close {
    transition: none;
  }

  .notification-headerbar-close:active {
    transform: none;
  }
}
</style>
