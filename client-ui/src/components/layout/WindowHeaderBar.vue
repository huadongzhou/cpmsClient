<script setup lang="ts" name="WindowHeaderBar">
type WindowControl = "pin" | "collapse" | "fullscreen" | "close";

withDefaults(
  defineProps<{
    title: string;
    icon?: string;
    pinned?: boolean;
    fullscreen?: boolean;
    controls?: WindowControl[];
  }>(),
  {
    icon: "",
    pinned: false,
    fullscreen: false,
    controls: () => ["pin", "collapse", "fullscreen", "close"],
  },
);

const emit = defineEmits<{
  pin: [];
  collapse: [];
  fullscreen: [];
  close: [];
}>();
</script>

<template>
  <header class="window-headerbar" data-tauri-drag-region>
    <div class="headerbar-title" data-tauri-drag-region>
      <img v-if="icon" :src="icon" class="headerbar-logo" alt="" data-tauri-drag-region />
      <strong class="headerbar-text" data-tauri-drag-region>{{ title }}</strong>
    </div>
    <nav class="headerbar-actions">
      <button
        v-if="controls.includes('pin')"
        type="button"
        class="headerbar-button"
        :class="{ 'is-active': pinned }"
        :aria-label="pinned ? '取消固定窗口' : '固定窗口'"
        :title="pinned ? '取消固定' : '固定'"
        @click="emit('pin')"
      >
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
          <path
            d="M9.5 1.5 14.5 6.5 13 8l-.7-.2-2.6 2.6.3 2.6-1.5 1.5L5 11 2 14l-1-1 3-3-3.5-3.5L2 5l2.6.3L7.2 2.7 7 2z"
            fill="currentColor"
          />
        </svg>
      </button>
      <button
        v-if="controls.includes('collapse')"
        type="button"
        class="headerbar-button"
        aria-label="收起窗口"
        title="收起"
        @click="emit('collapse')"
      >
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
          <rect x="3" y="7.25" width="10" height="1.5" fill="currentColor" />
        </svg>
      </button>
      <button
        v-if="controls.includes('fullscreen')"
        type="button"
        class="headerbar-button"
        :class="{ 'is-active': fullscreen }"
        :aria-label="fullscreen ? '退出全屏' : '全屏窗口'"
        :title="fullscreen ? '退出全屏' : '全屏'"
        @click="emit('fullscreen')"
      >
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
          <path
            d="M2 6V2h4v1.5H3.5V6zm8-4h4v4h-1.5V3.5H10zM2 10h1.5v2.5H6V14H2zm10.5 0H14v4h-4v-1.5h2.5z"
            fill="currentColor"
          />
        </svg>
      </button>
      <button
        v-if="controls.includes('close')"
        type="button"
        class="headerbar-button headerbar-button-close"
        aria-label="关闭窗口"
        title="关闭"
        @click="emit('close')"
      >
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
          <path
            d="M4.1 3 8 6.9 11.9 3 13 4.1 9.1 8 13 11.9 11.9 13 8 9.1 4.1 13 3 11.9 6.9 8 3 4.1z"
            fill="currentColor"
          />
        </svg>
      </button>
    </nav>
  </header>
</template>

<style scoped>
.window-headerbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--cpms-space-base);
  height: var(--cpms-headerbar-height);
  padding: 0 var(--cpms-space-base);
  background: var(--cpms-color-bg-panel);
  border-bottom: 1px solid var(--cpms-color-border);
  user-select: none;
}

.headerbar-title {
  display: flex;
  align-items: center;
  gap: var(--cpms-space-small);
  min-width: 0;
}

.headerbar-logo {
  width: 18px;
  height: 18px;
  flex: none;
}

.headerbar-text {
  font-size: var(--cpms-font-size-title);
  line-height: 20px;
  color: var(--cpms-color-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.headerbar-actions {
  display: flex;
  align-items: center;
  gap: var(--cpms-space-xs);
  flex: none;
}

.headerbar-button {
  display: inline-grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border: 0;
  border-radius: var(--cpms-radius-small);
  background: transparent;
  color: var(--cpms-color-text-muted);
  cursor: pointer;
}

.headerbar-button:hover {
  background: var(--cpms-color-bg-hover);
  color: var(--cpms-color-text-primary);
}

.headerbar-button.is-active {
  color: var(--el-color-primary);
}

.headerbar-button-close:hover {
  background: var(--cpms-color-danger-bg);
  color: var(--cpms-color-danger);
}
</style>
