<script setup lang="ts" name="NotificationView">
import { emit, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import {
  CircleAlert,
  CircleCheck,
  Info,
  TriangleAlert,
} from '@lucide/vue'
import type { Component } from 'vue'
import {
  DESKTOP_NOTIFICATION_ACK_EVENT,
  DESKTOP_NOTIFICATION_PUSH_EVENT,
  DESKTOP_NOTIFICATION_READY_EVENT,
  type DesktopNotificationAckPayload
} from '@/api/tauri/notification'
import WindowFrame from '@/components/layout/WindowFrame.vue'
import type { AppNotification, AppNotificationType } from '@/types/app/notification'

interface NotificationMeta {
  label: string
  icon: Component
}

const META_BY_TYPE: Record<AppNotificationType, NotificationMeta> = {
  info: {
    label: '通知',
    icon: Info
  },
  success: {
    label: '成功',
    icon: CircleCheck
  },
  warning: {
    label: '警告',
    icon: TriangleAlert
  },
  error: {
    label: '错误',
    icon: CircleAlert
  }
}

const currentWindow = getCurrentWindow()
const currentNotification = ref<AppNotification>()
let unlistenPush: UnlistenFn | undefined

onMounted(async () => {
  unlistenPush = await currentWindow.listen<AppNotification>(DESKTOP_NOTIFICATION_PUSH_EVENT, (event) => {
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
    <WindowFrame
      v-if="currentNotification"
      :title="notificationTitle"
      :controls="['close']"
      body-class="notification-frame-body"
      @close="closeNotification"
    >
      <section class="notification-body">
        <div class="notification-type">
          <component :is="notificationMeta.icon" class="notification-type-icon" />
          <span>{{ notificationMeta.label }}</span>
        </div>
        <p class="notification-message">{{ notificationMessage }}</p>
      </section>
    </WindowFrame>
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

:deep(.notification-frame-body) {
  background: var(--cpms-color-bg-app);
}

.notification-body {
  display: flex;
  flex-direction: column;
  flex: 1 1 auto;
  min-height: 0;
  gap: var(--cpms-space-3);
  margin: var(--cpms-space-3);
  padding: var(--cpms-space-4);
  border: 1px solid var(--cpms-color-border);
  border-radius: var(--cpms-radius-large);
  background: var(--cpms-color-surface);
  color: var(--cpms-color-text-secondary);
  font-size: var(--cpms-font-size-base);
  line-height: var(--cpms-line-height-relaxed);
  overflow: auto;
}

.notification-type {
  display: inline-flex;
  align-items: center;
  gap: var(--cpms-space-2);
  align-self: flex-start;
  padding: var(--cpms-space-1) var(--cpms-space-2);
  border-radius: var(--cpms-radius-full);
  background: var(--cpms-color-bg-hover);
  color: var(--cpms-color-text-secondary);
  font-size: var(--cpms-font-size-small);
  font-weight: var(--cpms-font-weight-medium);
}

.notification-type-icon {
  width: 16px;
  height: 16px;
  color: currentcolor;
}

.notification-message {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
