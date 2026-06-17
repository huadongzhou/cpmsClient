<script setup lang="ts" name="NotificationView">
import { listen, emit, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  DESKTOP_NOTIFICATION_PUSH_EVENT,
  DESKTOP_NOTIFICATION_READY_EVENT,
} from "@/api/tauri/notification";
import WindowHeaderBar from "@/components/layout/WindowHeaderBar.vue";
import type { AppNotification, AppNotificationType } from "@/types/app/notification";

interface NotificationMeta {
  label: string;
  color: string;
  iconPath: string;
}

const META_BY_TYPE: Record<AppNotificationType, NotificationMeta> = {
  info: {
    label: "信息",
    color: "var(--el-color-primary)",
    iconPath: "M8 1a7 7 0 1 0 0 14A7 7 0 0 0 8 1Zm0 3a.75.75 0 1 1 0 1.5A.75.75 0 0 1 8 4Zm0 3c.41 0 .75.34.75.75v3.5a.75.75 0 0 1-1.5 0v-3.5A.75.75 0 0 1 8 7Z",
  },
  success: {
    label: "成功",
    color: "var(--el-color-success)",
    iconPath: "M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14Zm3.47-8.78L7 10.69 5.03 8.72a.75.75 0 0 0-1.06 1.06l2.5 2.5c.29.29.77.29 1.06 0l5-5a.75.75 0 0 0-1.06-1.06Z",
  },
  warning: {
    label: "警告",
    color: "var(--el-color-warning)",
    iconPath: "M8.88 1.36a1.25 1.25 0 0 0-1.76 0l-6.5 6.5c-.49.49-.49 1.27 0 1.76l6.5 6.5c.49.49 1.27.49 1.76 0l6.5-6.5c.49-.49.49-1.27 0-1.76l-6.5-6.5ZM8 5.5a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0v-3.5A.75.75 0 0 1 8 5.5Zm0 6a.75.75 0 1 1 0 1.5.75.75 0 0 1 0-1.5Z",
  },
  error: {
    label: "错误",
    color: "var(--cpms-color-danger)",
    iconPath: "M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14Zm2.78-9.28a.75.75 0 0 0-1.06-1.06L8 6.94 6.28 5.22a.75.75 0 0 0-1.06 1.06L6.94 8 5.22 9.72a.75.75 0 1 0 1.06 1.06L8 9.06l1.72 1.72a.75.75 0 1 0 1.06-1.06L9.06 8l1.72-1.72Z",
  },
};

const currentWindow = getCurrentWindow();
const currentNotification = ref<AppNotification>();
let unlistenPush: UnlistenFn | undefined;

onMounted(async () => {
  unlistenPush = await listen<AppNotification>(DESKTOP_NOTIFICATION_PUSH_EVENT, (event) => {
    pushNotification(event.payload);
  });
  await emit(DESKTOP_NOTIFICATION_READY_EVENT);
});

onBeforeUnmount(() => {
  unlistenPush?.();
});

function pushNotification(notification: AppNotification) {
  currentNotification.value = notification;
  void currentWindow.show();
}

function closeNotification() {
  currentNotification.value = undefined;
  void currentWindow.hide();
}

const notificationMeta = computed(() => {
  if (!currentNotification.value) {
    return META_BY_TYPE.info;
  }

  return META_BY_TYPE[currentNotification.value.type] ?? META_BY_TYPE.info;
});
</script>

<template>
  <main class="notification-host" aria-live="polite">
    <article
      v-if="currentNotification"
      class="notification-card"
      :style="{ '--notification-accent': notificationMeta.color }"
    >
      <WindowHeaderBar
        :title="currentNotification.title"
        :controls="['close']"
        @close="closeNotification"
      />
      <section class="notification-body">
        <div class="notification-type">
          <svg
            class="notification-type-icon"
            viewBox="0 0 16 16"
            width="14"
            height="14"
            aria-hidden="true"
          >
            <path :d="notificationMeta.iconPath" fill="currentColor" />
          </svg>
          <span class="notification-type-label">{{ notificationMeta.label }}</span>
        </div>
        <p class="notification-message">{{ currentNotification.message || "无消息内容" }}</p>
      </section>
    </article>
  </main>
</template>

<style scoped>
.notification-host {
  width: 100vw;
  min-height: 100vh;
  background: var(--cpms-color-bg-panel);
  overflow: hidden;
}

.notification-card {
  width: 100%;
  min-height: 100vh;
  display: grid;
  grid-template-rows: auto 1fr;
  background: var(--cpms-color-bg-panel);
  color: var(--cpms-color-text-primary);
  box-shadow: var(--cpms-shadow-md);
  border-left: 4px solid var(--notification-accent);
}

.notification-body {
  padding: var(--cpms-space-base);
  color: var(--cpms-color-text-secondary);
  font-size: var(--cpms-font-size-small);
  line-height: var(--cpms-line-height-small);
  overflow: auto;
}

.notification-type {
  display: inline-flex;
  align-items: center;
  gap: var(--cpms-space-xs);
  margin-bottom: var(--cpms-space-small);
  padding: var(--cpms-space-xs) var(--cpms-space-small);
  border-radius: var(--cpms-radius-small);
  background: var(--cpms-color-bg-hover);
  color: var(--notification-accent);
  font-weight: 600;
}

.notification-type-icon {
  flex: none;
}

.notification-message {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
