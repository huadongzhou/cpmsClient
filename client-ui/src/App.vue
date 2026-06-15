<script setup lang="ts" name="App">
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useClientEventBridge } from "@/composables/useClientEventBridge";
import { useClientLogBridge } from "@/composables/useClientLogBridge";
import { useClientNotificationBridge } from "@/composables/useClientNotificationBridge";
import { useClientRuntimeBridge } from "@/composables/useClientRuntimeBridge";
import { useDesktopNotificationBridge } from "@/composables/useDesktopNotificationBridge";
import HomeView from "@/views/home/index.vue";
import NotificationView from "@/views/notification/index.vue";

const isNotificationWindow = ref(isTauri() && getCurrentWindow().label === "notification");
const isMainWindow = computed(() => !isNotificationWindow.value);

if (isMainWindow.value) {
  useDesktopNotificationBridge();
  useClientNotificationBridge();
  useClientEventBridge();
  useClientRuntimeBridge();
  useClientLogBridge();
}
</script>

<template>
  <NotificationView v-if="isNotificationWindow" />
  <HomeView v-else />
</template>

<style>
:root {
  font-family: var(--cpms-font-family);
  font-size: var(--cpms-font-size-base);
  line-height: var(--cpms-line-height-base);
  font-weight: 400;

  color: var(--cpms-color-text-primary);
  background-color: var(--cpms-color-bg-app);

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

body {
  margin: 0;
}

html,
body {
  min-height: 100vh;
}

* {
  box-sizing: border-box;
}

#app {
  min-height: 100vh;
}
</style>
