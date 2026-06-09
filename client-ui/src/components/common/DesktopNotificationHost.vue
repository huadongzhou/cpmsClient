<script setup lang="ts" name="DesktopNotificationHost">
import { listen, emit, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  DESKTOP_NOTIFICATION_PUSH_EVENT,
  DESKTOP_NOTIFICATION_READY_EVENT,
} from "@/api/desktop/notification";
import type { AppNotification } from "@/types/app/notification";

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
</script>

<template>
  <main class="notification-host" aria-live="polite">
    <article v-if="currentNotification" class="notification-card">
      <header class="notification-header">
        <strong class="notification-title">{{ currentNotification.title }}</strong>
        <button
          class="notification-close"
          type="button"
          aria-label="关闭通知"
          @click="closeNotification"
        >
          x
        </button>
      </header>
      <section class="notification-body">
        {{ currentNotification.message || "无消息内容" }}
      </section>
    </article>
  </main>
</template>

<style scoped>
.notification-host {
  width: 100vw;
  min-height: 100vh;
  background: #ffffff;
  overflow: hidden;
}

.notification-card {
  width: 100%;
  min-height: 100vh;
  display: grid;
  grid-template-rows: auto 1fr;
  border: 0;
  border-radius: 0;
  background: #ffffff;
  color: #1f2937;
}

.notification-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-height: 44px;
  padding: 0 12px;
  border-bottom: 1px solid #eef2f7;
}

.notification-title {
  font-size: 14px;
  line-height: 20px;
  word-break: break-word;
}

.notification-close {
  display: inline-grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: #687385;
  cursor: pointer;
}

.notification-close:hover {
  background: #f1f4f8;
  color: #1f2937;
}

.notification-body {
  padding: 12px;
  color: #374151;
  font-size: 13px;
  line-height: 20px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
