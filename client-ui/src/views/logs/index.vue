<script setup lang="ts" name="LogView">
import { storeToRefs } from "pinia";
import { Copy, FileText, Inbox, LoaderCircle, RefreshCw, Trash2 } from "@lucide/vue";
import { getClientLogState } from "@/api/tauri/log";
import { confirmAction, message } from "@/services/ui/message";
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
  if (key === "http") {
    return "请求 / HTTP";
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

/** 选中类别的日志条目（时间正序），用于按级别着色展示。 */
const formattedLogEntries = computed(() =>
  [...filteredEntries.value].reverse().map((entry) => {
    const head = `[${formatTime(entry)}] [${entry.level.toUpperCase()}] ${entry.title}`;
    return {
      id: `${entry.at}-${entry.source}-${entry.title}`,
      level: entry.level,
      head,
      detail: entry.detail ? entry.detail.replace(/\n/g, "\n    ") : undefined,
    };
  }),
);

/** 供复制用的纯文本拼接（与展示顺序一致）。 */
const logText = computed(() =>
  formattedLogEntries.value
    .map((entry) => (entry.detail ? `${entry.head}\n    ${entry.detail}` : entry.head))
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
    message.info("暂无可复制日志");
    return;
  }

  await navigator.clipboard.writeText(logText.value);
  message.success("日志已复制");
}

async function confirmClearLogs() {
  try {
    const confirmed = await confirmAction("确定要清空当前显示的客户端日志吗？此操作不可恢复。");
    if (!confirmed) return;

    logStore.clearLogs();
    message.success("日志已清空");
  } catch {
    // 用户取消，不做任何操作。
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

function levelClass(level: string) {
  switch (level) {
    case "error":
      return "is-error";
    case "warn":
      return "is-warn";
    case "info":
      return "is-info";
    case "debug":
      return "is-debug";
    default:
      return "is-info";
  }
}
</script>

<template>
  <main class="logs-view">
    <section class="toolbar">
      <div class="toolbar-left">
        <Select v-model="activeCategory">
          <SelectTrigger class="category-select" aria-label="类别">
            <SelectValue placeholder="类别" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem v-for="category in categories" :key="category" :value="category">
              {{ category }}
            </SelectItem>
          </SelectContent>
        </Select>
        <span class="entry-count">共 {{ formattedLogEntries.length }} 条</span>
      </div>
      <div class="actions">
        <Button variant="outline" :disabled="fileStateLoading" @click="refreshFileState">
          <LoaderCircle v-if="fileStateLoading" class="animate-spin" />
          <RefreshCw v-else />
          刷新
        </Button>
        <Button variant="outline" @click="copyLogs">
          <Copy />
          复制
        </Button>
        <Button variant="outline" class="text-destructive hover:bg-destructive/10" @click="confirmClearLogs">
          <Trash2 />
          清空
        </Button>
      </div>
    </section>

    <section v-if="fileState" class="file-state">
      <div class="file-state-main">
        <FileText class="file-state-icon" />
        <span class="file-path" :title="fileState.path">{{ fileState.path }}</span>
      </div>
      <span class="file-size">{{ formatFileSize(fileState.sizeBytes) }}</span>
    </section>

    <section class="log-panel">
      <Empty v-if="filteredEntries.length === 0" class="logs-empty">
        <Inbox class="logs-empty-icon" />
        <p>暂无客户端日志</p>
      </Empty>
      <div v-else class="log-text">
        <div
          v-for="entry in formattedLogEntries"
          :key="entry.id"
          :class="['log-entry', levelClass(entry.level)]"
        >
          <div class="log-head">{{ entry.head }}</div>
          <div v-if="entry.detail" class="log-detail">{{ entry.detail }}</div>
        </div>
      </div>
    </section>
  </main>
</template>

<style scoped>
.logs-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: var(--cpms-space-base);
  gap: var(--cpms-space-base);
  overflow: hidden;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--cpms-space-2);
  padding: var(--cpms-space-3);
  background: var(--cpms-color-surface);
  border: 1px solid var(--cpms-color-border);
  border-radius: var(--cpms-radius-panel);
  box-shadow: var(--cpms-shadow-xs);
  flex-wrap: wrap;
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: var(--cpms-space-3);
}

.category-select {
  width: 160px;
}

.entry-count {
  font-size: var(--cpms-font-size-small);
  color: var(--cpms-color-text-muted);
}

.actions {
  display: flex;
  gap: var(--cpms-space-2);
}

.file-state {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--cpms-space-2);
  padding: var(--cpms-space-2) var(--cpms-space-3);
  color: var(--cpms-color-text-secondary);
  background: var(--cpms-color-bg-code);
  border: 1px solid var(--cpms-color-border);
  border-radius: var(--cpms-radius-small);
  font-size: var(--cpms-font-size-small);
}

.file-state-main {
  display: flex;
  align-items: center;
  gap: var(--cpms-space-2);
  min-width: 0;
}

.file-state-icon {
  width: 16px;
  height: 16px;
  flex: none;
  color: var(--cpms-color-text-muted);
}

.logs-empty {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--cpms-space-2);
  color: var(--cpms-color-text-muted);
}

.logs-empty-icon {
  width: 48px;
  height: 48px;
  opacity: 0.6;
}

.file-path {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-size {
  flex: none;
  font-weight: var(--cpms-font-weight-medium);
  color: var(--cpms-color-text-primary);
}

.log-panel {
  flex: 1 1 auto;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--cpms-color-surface);
  border: 1px solid var(--cpms-color-border);
  border-radius: var(--cpms-radius-panel);
  box-shadow: var(--cpms-shadow-xs);
  overflow: hidden;
}

.log-text {
  flex: 1 1 auto;
  min-height: 0;
  margin: 0;
  overflow: auto;
  padding: var(--cpms-space-base);
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--cpms-color-text-primary);
  font-family: var(--cpms-font-family-mono);
  font-size: var(--cpms-font-size-small);
  line-height: var(--cpms-line-height-relaxed);
}

.log-entry {
  padding: var(--cpms-space-1) 0;
  border-bottom: 1px solid transparent;
  transition: background-color var(--cpms-duration-fast) var(--cpms-easing-base);
}

.log-entry:hover {
  background: var(--cpms-color-bg-hover);
  border-bottom-color: var(--cpms-color-border);
}

.log-entry.is-error {
  color: var(--cpms-color-danger-text);
}

.log-entry.is-warn {
  color: var(--cpms-color-warning-text);
}

.log-entry.is-info {
  color: var(--cpms-color-text-secondary);
}

.log-entry.is-debug {
  color: var(--cpms-color-text-muted);
}

.log-head {
  font-weight: var(--cpms-font-weight-medium);
}

.log-detail {
  padding-left: var(--cpms-space-base);
  white-space: pre-wrap;
  word-break: break-word;
  opacity: 0.92;
}

@media (prefers-reduced-motion: reduce) {
  .log-entry {
    transition: none;
  }
}
</style>
