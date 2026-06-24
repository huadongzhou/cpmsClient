<script setup lang="ts" name="App">
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { installClientTauriBridge } from "@/api/tauri/events";
import {
  useClientEventBridge,
  useClientLogBridge,
  useClientNotificationBridge,
  useClientRuntimeBridge,
  useDesktopNotificationBridge,
} from "@/bridges";
import HomeView from "@/views/home/index.vue";
import NotificationView from "@/views/notification/index.vue";

const isNotificationWindow = ref(isTauri() && getCurrentWindow().label === "notification");
const isMainWindow = computed(() => !isNotificationWindow.value);

if (isMainWindow.value) {
  // 一次性装配：客户端 Tauri 事件统一汇入事件总线，下方各 bridge 仅订阅总线。
  void installClientTauriBridge();
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
/**
 * 统一容器：html/body 占满视口，不提供滚动；#app 占满容器，内部视图按需滚动。
 * 统一滚动条：WebKit 定制细轨圆角样式，适配明暗主题。
 */
html {
  width: 100vw;
  height: 100vh;
  overflow: hidden;

  font-family: var(--cpms-font-family);
  font-size: var(--cpms-font-size-base);
  line-height: var(--cpms-line-height-base);
  font-weight: var(--cpms-font-weight-normal);

  color: var(--cpms-color-text-primary);
  background-color: transparent;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  user-select: none;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

body {
  width: 100%;
  height: 100%;
  margin: 0;
  overflow: hidden;
  background: transparent;
}

#app {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  overflow: hidden;
  border-radius: var(--cpms-radius-panel);
  background: transparent;
}

* {
  box-sizing: border-box;
}

input,
textarea,
[contenteditable="true"] {
  user-select: text;
}

::selection {
  color: var(--cpms-color-text-on-primary);
  background-color: var(--cpms-color-primary);
}

button:focus-visible,
a:focus-visible,
[tabindex]:not([tabindex="-1"]):focus-visible {
  outline: 2px solid var(--cpms-color-primary);
  outline-offset: 2px;
}

svg:focus,
svg:focus-visible,
svg:active,
svg.is-active {
  outline: none;
}

/* WebKit 统一滚动条 */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-track {
  background: var(--cpms-scrollbar-track);
}

::-webkit-scrollbar-thumb {
  background: var(--cpms-scrollbar-thumb);
  border-radius: var(--cpms-radius-full);
  border: 2px solid transparent;
  background-clip: content-box;
}

::-webkit-scrollbar-thumb:hover {
  background: var(--cpms-scrollbar-thumb-hover);
  background-clip: content-box;
}

::-webkit-scrollbar-corner {
  background: transparent;
}

@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
}
</style>
