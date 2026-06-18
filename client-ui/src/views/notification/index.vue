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
}

const META_BY_TYPE: Record<AppNotificationType, NotificationMeta> = {
  info: {
    label: "通知",
  },
  success: {
    label: "通知",
  },
  warning: {
    label: "通知",
  },
  error: {
    label: "通知",
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

const notificationTitle = computed(() => {
  const sourceText = [
    currentNotification.value?.title ?? "",
    currentNotification.value?.message ?? "",
  ].join(" ");

  if (/打印|作业|任务|PrintClient|Socket/i.test(sourceText)) {
    return "打印任务通知";
  }

  return notificationMeta.value.label;
});
const notificationMessage = computed(() => currentNotification.value?.message?.trim() || "无消息内容");
</script>

<template>
  <main class="notification-host" aria-live="polite">
    <article
      v-if="currentNotification"
      class="notification-card"
    >
      <WindowHeaderBar
        :title="notificationTitle"
        :controls="['close']"
        @close="closeNotification"
      />
      <section class="notification-body">
        <p class="notification-message">{{ notificationMessage }}</p>
      </section>
    </article>
  </main>
</template>

<style scoped>
.notification-host {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  background: var(--cpms-color-bg-panel);
  overflow: hidden;
}

.notification-card {
  display: flex;
  flex-direction: column;
  flex: 1 1 auto;
  min-height: 0;
  width: 100%;
  background: var(--cpms-color-bg-panel);
  color: var(--cpms-color-text-primary);
  box-shadow: var(--cpms-shadow-md);
}

.notification-body {
  display: flex;
  flex-direction: column;
  flex: 1 1 auto;
  min-height: 0;
  padding: var(--cpms-space-base);
  color: var(--cpms-color-text-secondary);
  font-size: var(--cpms-font-size-base);
  line-height: var(--cpms-line-height-relaxed);
  overflow: auto;
}

.notification-message {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
