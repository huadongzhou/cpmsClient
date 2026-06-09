<script setup lang="ts" name="HomeView">
import { vLoading } from "element-plus";
import "element-plus/es/components/loading/style/css";
import { useIframeContainer } from "@/composables/useIframeContainer";
import { useIframePayloadBridge } from "@/composables/useIframePayloadBridge";
import ExampleView from "@/views/example/index.vue";

const iframeRef = ref<HTMLIFrameElement>();
const iframeDomLoaded = ref(false);
const exampleDrawerVisible = ref(false);
const { iframe, loadIframeContainer, loading } = useIframeContainer();
const { queryIframePayload } = useIframePayloadBridge(iframeRef);
const iframeSrc = computed(() => iframe.value.url || "about:blank");
const isIframeLoading = computed(() => {
  return (
    loading.value ||
    iframe.value.state === "idle" ||
    iframe.value.state === "loading" ||
    (iframe.value.state === "loaded" && Boolean(iframe.value.url) && !iframeDomLoaded.value)
  );
});

watch(iframeSrc, () => {
  iframeDomLoaded.value = false;
});

onMounted(() => {
  if (iframe.value.state === "idle") {
    void loadIframeContainer();
  }
});

function handleIframeLoad() {
  iframeDomLoaded.value = true;
}
</script>

<template>
  <main v-loading="isIframeLoading" element-loading-text="正在加载业务页面" class="iframe-root">
    <iframe ref="iframeRef" :src="iframeSrc" class="business-iframe" @load="handleIframeLoad" />
    <el-button class="example-trigger" type="primary" @click="exampleDrawerVisible = true"
      >能力检测</el-button
    >
    <el-drawer v-model="exampleDrawerVisible" title="客户端能力检测" size="520px" destroy-on-close>
      <ExampleView :query-iframe-payload="queryIframePayload" />
    </el-drawer>
  </main>
</template>

<style scoped>
.iframe-root {
  position: relative;
  width: 100%;
  min-height: 100vh;
  background: #ffffff;
}

.business-iframe {
  width: 100%;
  min-height: 100vh;
  border: 0;
  display: block;
}

.example-trigger {
  position: fixed;
  right: 20px;
  bottom: 20px;
  z-index: 10;
}
</style>
