import { defineStore } from "pinia";
import { useLocalStorage } from "@vueuse/core";

export const useAppStore = defineStore("app", () => {
  const greetCount = useLocalStorage("cpmsClient:greetCount", 0);

  function increaseGreetCount() {
    greetCount.value += 1;
  }

  return {
    greetCount,
    increaseGreetCount,
  };
});
