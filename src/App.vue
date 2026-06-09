<script setup lang="ts" name="App">
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import DesktopNotificationHost from "@/components/common/DesktopNotificationHost.vue";
import { useClientEventBridge } from "@/composables/useClientEventBridge";
import { useClientNotificationBridge } from "@/composables/useClientNotificationBridge";
import { useClientRuntimeBridge } from "@/composables/useClientRuntimeBridge";
import { useDesktopNotificationBridge } from "@/composables/useDesktopNotificationBridge";
import HomeView from "@/views/home/index.vue";

const isNotificationWindow = ref(isTauri() && getCurrentWindow().label === "notification");
const isMainWindow = computed(() => !isNotificationWindow.value);

if (isMainWindow.value) {
  useDesktopNotificationBridge();
  useClientNotificationBridge();
  useClientEventBridge();
  useClientRuntimeBridge();
}
</script>

<template>
  <DesktopNotificationHost v-if="isNotificationWindow" />
  <HomeView v-else />
</template>

<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f4f6f8;

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
