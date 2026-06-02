import { storeToRefs } from "pinia";
import { refreshClientIframeContainer } from "@/api/tauri/desktop";
import { useRuntimeStore } from "@/stores/runtime";

/** 读取客户端下发的 iframe 状态，并支持触发客户端刷新。 */
export function useIframeContainer() {
  const runtimeStore = useRuntimeStore();
  const { iframe } = storeToRefs(runtimeStore);
  const loading = ref(false);

  async function loadIframeContainer() {
    loading.value = true;
    try {
      const payload = await refreshClientIframeContainer();
      runtimeStore.setIframeState(payload);
    } finally {
      loading.value = false;
    }
  }

  return {
    iframe,
    loading,
    loadIframeContainer,
  };
}
