import { createApp } from "vue";
import App from "./App.vue";
import { createPinia } from "pinia";
import { useAppStore } from "@/stores/app";
import { injectHubClientBridge } from "@/utils/hubBridge";
import "@/assets/styles/tokens.css";
import "uno.css";

const pinia = createPinia();
const app = createApp(App);

app.use(pinia);

const appStore = useAppStore(pinia);

app.config.errorHandler = (error) => {
  appStore.pushError({
    source: "ui",
    level: "error",
    code: "VUE_RUNTIME_ERROR",
    message: error instanceof Error ? error.message : "前端运行异常",
  });
};

window.addEventListener("unhandledrejection", (event) => {
  appStore.pushError({
    source: "ui",
    level: "error",
    code: "UNHANDLED_REJECTION",
    message: event.reason instanceof Error ? event.reason.message : "未处理的异步异常",
  });
});

app.mount("#app");

/** 向全局注入 Hub 桥接对象，供 iframe 内的 hub-platform 调用 Tauri 能力。 */
injectHubClientBridge();
