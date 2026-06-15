<script setup lang="ts" name="LogView">
import { storeToRefs } from "pinia";
import { ElMessage } from "element-plus";
import { getClientLogState } from "@/api/tauri/log";
import { useLogStore } from "@/stores/log";
import type { ClientLogEntry, ClientLogFileState } from "@/types/app/log";

const logStore = useLogStore();
const { logs } = storeToRefs(logStore);
const fileState = ref<ClientLogFileState>();
const fileStateLoading = ref(false);
const activeCategory = ref("全部");

/** 把日志来源归类到可读的业务类别。 */
function categoryOf(source: string): string {
  if (source.startsWith("error/") || source === "client/panic") {
    return "错误";
  }

  const key = source.replace(/^client\//, "");
  if (key === "business" || key === "lifecycle" || source === "client-event") {
    return "业务流程";
  }
  if (key === "iframe") {
    return "iframe";
  }
  if (key === "socket") {
    return "任务 / Socket";
  }
  if (key === "token") {
    return "Token";
  }
  if (key === "startup" || key === "window" || key === "single-instance") {
    return "启动 / 窗口";
  }

  return "其他";
}

/** 当前出现过的类别（含「全部」），供下拉选择。 */
const categories = computed(() => {
  const present = new Set<string>();
  for (const entry of logs.value) {
    present.add(categoryOf(entry.source));
  }
  return ["全部", ...Array.from(present)];
});

const filteredEntries = computed(() =>
  activeCategory.value === "全部"
    ? logs.value
    : logs.value.filter((entry) => categoryOf(entry.source) === activeCategory.value),
);

/** 选中类别的日志拼成一段长文本（时间正序），不使用每条一块的日志块。 */
const logText = computed(() =>
  [...filteredEntries.value]
    .reverse()
    .map((entry) => {
      const head = `[${formatTime(entry)}] [${entry.level.toUpperCase()}] [${entry.source}] ${entry.title}`;
      return entry.detail ? `${head}\n    ${entry.detail.replace(/\n/g, "\n    ")}` : head;
    })
    .join("\n"),
);

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
      <el-select v-model="activeCategory" class="category-select" placeholder="类别">
        <el-option v-for="category in categories" :key="category" :label="category" :value="category" />
      </el-select>
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

    <el-empty v-if="filteredEntries.length === 0" description="暂无客户端日志" />
    <pre v-else class="log-text">{{ logText }}</pre>
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

.category-select {
  width: 160px;
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

.log-text {
  margin: 0;
  max-height: 60vh;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
  background: var(--cpms-color-bg-code);
  border: 1px solid var(--cpms-color-border);
  border-radius: var(--cpms-radius-small);
  padding: var(--cpms-space-base);
  color: var(--cpms-color-text-primary);
  font-size: var(--cpms-font-size-small);
  line-height: 20px;
}
</style>
