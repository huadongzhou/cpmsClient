import { defineStore } from "pinia";
import { useLocalStorage } from "@vueuse/core";

export const useUserStore = defineStore("user", () => {
  const token = useLocalStorage<string | null>("cpmsClient:token", null);
  const displayName = useLocalStorage<string>("cpmsClient:displayName", "");
  const isAuthenticated = computed(() => Boolean(token.value));

  /** 写入登录会话信息。 */
  function setSession(nextToken: string, nextDisplayName: string) {
    token.value = nextToken;
    displayName.value = nextDisplayName;
  }

  /** 清空当前登录会话。 */
  function clearSession() {
    token.value = null;
    displayName.value = "";
  }

  return {
    token,
    displayName,
    isAuthenticated,
    setSession,
    clearSession,
  };
});
