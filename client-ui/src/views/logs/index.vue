<script setup lang="ts" name="LogView">
import { storeToRefs } from "pinia";
import { ElMessage } from "element-plus";
import { getClientLogState } from "@/api/tauri/log";
import { useLogStore } from "@/stores/log";
import type { ClientLogEntry, ClientLogFileState, ClientLogLevel } from "@/types/app/log";

const logStore = useLogStore();
const { logs, logText } = storeToRefs(logStore);
const fileState = ref<ClientLogFileState>();
const fileStateLoading = ref(false);

const levelOptions: Array<ClientLogLevel | "all"> = ["all", "info", "warn", "error"];
const activeLevel = ref<ClientLogLevel | "all">("all");

const filteredLogs = computed(() => {
  if (activeLevel.value === "all") {
    return logs.value;
  }

  return logs.value.filter((entry) => entry.level === activeLevel.value);
});

onMounted(() => {
  void refreshFileState();
});

async function refreshFileState() {
  fileStateLoading.value = true;
  try {
    fileState.value = await getClientLogState();
  } catch {
    fileState.value = undefined;
  } finally {
    fileStateLoading.value = false;
  }
}

async function copyLogs() {
  if (!logText.value) {
    ElMessage.info("暂无可复制日志");
    return;
  }

  await navigator.clipboard.writeText(logText.value);
  ElMessage.success("日志已复制");
}

function getLevelType(level: ClientLogLevel) {
  switch (level) {
    case "error":
      return "danger";
    case "warn":
      return "warning";
    default:
      return "info";
  }
}

function formatTime(entry: ClientLogEntry) {
  return new Date(entry.at).toLocaleString();
}

function formatFileSize(sizeBytes: number) {
  if (sizeBytes < 1024) {
    return `${sizeBytes} B`;
  }

  if (sizeBytes < 1024 * 1024) {
    return `${(sizeBytes / 1024).toFixed(1)} KB`;
  }

  return `${(sizeBytes / 1024 / 1024).toFixed(1)} MB`;
}
</script>

<template>
  <main class="logs-view">
    <section class="toolbar">
      <el-segmented v-model="activeLevel" :options="levelOptions" />
      <div class="actions">
        <el-button plain :loading="fileStateLoading" @click="refreshFileState">刷新</el-button>
        <el-button plain @click="copyLogs">复制</el-button>
        <el-button plain type="danger" @click="logStore.clearLogs">清空</el-button>
      </div>
    </section>

    <section v-if="fileState" class="file-state">
      <span class="file-path">{{ fileState.path }}</span>
      <span>{{ formatFileSize(fileState.sizeBytes) }}</span>
    </section>

    <el-empty v-if="filteredLogs.length === 0" description="暂无客户端日志" />
    <section v-else class="log-list">
      <article v-for="entry in filteredLogs" :key="entry.id" class="log-item">
        <header class="log-head">
          <el-tag size="small" :type="getLevelType(entry.level)" effect="light">
            {{ entry.level.toUpperCase() }}
          </el-tag>
          <span class="source">{{ entry.source }}</span>
          <time>{{ formatTime(entry) }}</time>
        </header>
        <strong>{{ entry.title }}</strong>
        <pre v-if="entry.detail">{{ entry.detail }}</pre>
      </article>
    </section>
  </main>
</template>

<style scoped>
.logs-view {
  display: grid;
  gap: var(--cpms-space-base);
  padding: var(--cpms-space-base);
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--cpms-space-small);
  flex-wrap: wrap;
}

.actions {
  display: flex;
  gap: var(--cpms-space-small);
}

.file-state {
  display: flex;
  justify-content: space-between;
  gap: var(--cpms-space-small);
  padding: var(--cpms-space-small);
  color: var(--cpms-color-text-secondary);
  background: var(--cpms-color-bg-code);
  border-radius: var(--cpms-radius-small);
  font-size: var(--cpms-font-size-small);
}

.file-path {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.log-list {
  display: grid;
  gap: var(--cpms-space-small);
}

.log-item {
  display: grid;
  gap: var(--cpms-space-small);
  padding: var(--cpms-space-base);
  background: var(--cpms-color-bg-panel);
  border: 1px solid var(--cpms-color-border);
  border-radius: var(--cpms-radius-panel);
}

.log-head {
  display: flex;
  align-items: center;
  gap: var(--cpms-space-small);
  color: var(--cpms-color-text-secondary);
  font-size: var(--cpms-font-size-small);
}

.source {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

strong {
  color: var(--cpms-color-text-primary);
}

pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  background: var(--cpms-color-bg-code);
  border-radius: var(--cpms-radius-small);
  padding: var(--cpms-space-small);
  font-size: var(--cpms-font-size-small);
}
</style>
