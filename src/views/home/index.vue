<script setup lang="ts" name="HomeView">
import { invoke } from "@tauri-apps/api/core";
import { useAppStore } from "@/stores/app";

const appStore = useAppStore();
const greetMsg = ref("");
const name = ref("");

async function greet() {
  appStore.increaseGreetCount();
  greetMsg.value = await invoke<string>("greet", { name: name.value });
}
</script>

<template>
  <main class="container">
    <h1>CPMS Client</h1>

    <div class="row">
      <a href="https://vite.dev" target="_blank">
        <img src="/vite.svg" class="logo vite" alt="Vite logo" />
      </a>
      <a href="https://tauri.app" target="_blank">
        <img src="/tauri.svg" class="logo tauri" alt="Tauri logo" />
      </a>
      <a href="https://vuejs.org/" target="_blank">
        <img src="@/assets/vue.svg" class="logo vue" alt="Vue logo" />
      </a>
    </div>

    <form class="greet-form" @submit.prevent="greet">
      <el-input v-model="name" class="greet-input" placeholder="Enter a name..." clearable />
      <el-button type="primary" native-type="submit">Greet</el-button>
    </form>

    <p v-if="greetMsg">{{ greetMsg }}</p>
    <p>Greet count: {{ appStore.greetCount }}</p>
  </main>
</template>

<style scoped>
.container {
  margin: 0;
  padding-top: 10vh;
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
}

.row {
  display: flex;
  justify-content: center;
}

.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: 0.75s;
}

.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.tauri:hover {
  filter: drop-shadow(0 0 2em #24c8db);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #249b73);
}

.greet-form {
  display: flex;
  justify-content: center;
  gap: 8px;
  width: min(420px, calc(100vw - 32px));
}

.greet-input {
  flex: 1;
}
</style>
