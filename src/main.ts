import { createApp } from "vue";
import App from "./App.vue";
import { createPinia } from "pinia";
import { useAppStore } from "@/stores/app";
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
