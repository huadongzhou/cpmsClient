<script setup lang="ts" name="NotificationView">
import { listen, emit, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  DESKTOP_NOTIFICATION_PUSH_EVENT,
  DESKTOP_NOTIFICATION_READY_EVENT,
} from "@/api/tauri/notification";
import WindowHeaderBar from "@/components/layout/WindowHeaderBar.vue";
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
      <WindowHeaderBar
        :title="currentNotification.title"
        :controls="['close']"
        @close="closeNotification"
      />
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
}

.notification-body {
  padding: var(--cpms-space-base);
  color: var(--cpms-color-text-secondary);
  font-size: var(--cpms-font-size-small);
  line-height: 20px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
