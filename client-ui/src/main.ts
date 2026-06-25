import Vue from "vue";
import { createPinia, PiniaVuePlugin } from "pinia";
import ElementUI from "element-ui";
import "element-ui/lib/theme-chalk/index.css";
import locale from "element-ui/lib/locale/lang/zh-CN";
import App from "./App.vue";
import { useAppStore } from "@/stores/app";
import "@/assets/styles/tokens.css";

Vue.config.productionTip = false;
Vue.use(PiniaVuePlugin);
Vue.use(ElementUI, { locale });

const pinia = createPinia();
const appStore = useAppStore(pinia);

Vue.config.errorHandler = (error) => {
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

new Vue({
  pinia,
  render: (h) => h(App),
}).$mount("#app");
