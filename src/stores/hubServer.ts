import { defineStore } from "pinia";
import type { ServerData } from "@/types/hub/server";

export const SYS_PRODUCT_UNKNOWN = -1;
export const SYS_PRODUCT_A = 0;
export const SYS_PRODUCT_C = 1;
export const SYS_PRODUCT_B = 2;

export const useHubServerStore = defineStore("hubServer", () => {
  const server = ref<ServerData | null>(null);
  const productType = ref<number>(SYS_PRODUCT_UNKNOWN);
  const systemInitData = ref<unknown>(null);

  /** 写入当前 CPMS 服务器。 */
  function setServer(nextServer: ServerData | null) {
    server.value = nextServer;
  }

  /** 写入服务端产品类型和初始化数据。 */
  function setSystemInfo(nextProductType: number, nextInitData?: unknown) {
    productType.value = nextProductType;
    systemInitData.value = nextInitData ?? null;
  }

  return {
    server,
    productType,
    systemInitData,
    setServer,
    setSystemInfo,
  };
});
