import { defineStore } from "pinia";
import { useOnline } from "@vueuse/core";

export const useNetworkStore = defineStore("network", () => {
  const isOnline = useOnline();

  return {
    isOnline,
  };
});
