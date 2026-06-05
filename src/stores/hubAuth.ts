import { defineStore } from "pinia";
import type { UserData } from "@/types/hub/auth";

export const useHubAuthStore = defineStore("hubAuth", () => {
  const user = ref<UserData | null>(null);
  const token = computed(() => user.value?.token ?? "");
  const isAuthenticated = computed(() => Boolean(token.value));

  /** 写入 CPMS 登录用户。 */
  function setUser(nextUser: UserData | null) {
    user.value = nextUser;
  }

  /** 清空 CPMS 登录用户。 */
  function clearUser() {
    user.value = null;
  }

  return {
    user,
    token,
    isAuthenticated,
    setUser,
    clearUser,
  };
});
