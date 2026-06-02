import { defineStore } from "pinia";
import type { ClientIframeStatePayload } from "@/types/app/runtime";

const INITIAL_IFRAME_STATE: ClientIframeStatePayload = {
  state: "idle",
  url: null,
  message: null,
  updatedAt: "",
};

export const useRuntimeStore = defineStore("runtime", () => {
  const iframe = ref<ClientIframeStatePayload>({ ...INITIAL_IFRAME_STATE });
  const iframeToken = ref<string>("");

  function setIframeState(payload: ClientIframeStatePayload) {
    iframe.value = payload;
  }

  function setIframeToken(token: string) {
    iframeToken.value = token;
  }

  return {
    iframe,
    iframeToken,
    setIframeState,
    setIframeToken,
  };
});
