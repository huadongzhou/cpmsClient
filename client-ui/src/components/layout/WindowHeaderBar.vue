<script setup lang="ts" name="WindowHeaderBar">
import { computed } from "vue";
import Icon from "@/components/common/Icon.vue";

type WindowControl = "pin" | "collapse" | "fullscreen" | "close";

const props = withDefaults(
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

const pinLabel = computed(() => (props.pinned ? "取消固定窗口" : "固定窗口"));
const pinTitle = computed(() => (props.pinned ? "取消固定" : "固定"));
const fullscreenLabel = computed(() => (props.fullscreen ? "退出全屏" : "全屏窗口"));
const fullscreenTitle = computed(() => (props.fullscreen ? "退出全屏" : "全屏"));
</script>

<template>
  <header class="window-headerbar" data-tauri-drag-region>
    <div class="headerbar-title" data-tauri-drag-region>
      <img v-if="icon" :src="icon" class="headerbar-logo" alt="应用图标" data-tauri-drag-region />
      <strong class="headerbar-text" data-tauri-drag-region>{{ title }}</strong>
    </div>
    <nav class="headerbar-actions" aria-label="窗口控制">
      <button
        v-if="controls.includes('pin')"
        type="button"
        class="headerbar-button"
        :class="{ 'is-active': pinned }"
        :aria-label="pinLabel"
        :title="pinTitle"
        @click="emit('pin')"
      >
        <Icon icon="solar:pin-bold" class="headerbar-icon" />
      </button>
      <button
        v-if="controls.includes('collapse')"
        type="button"
        class="headerbar-button"
        aria-label="收起窗口"
        title="收起"
        @click="emit('collapse')"
      >
        <Icon icon="solar:minimize-square-minimalistic-bold" class="headerbar-icon" />
      </button>
      <button
        v-if="controls.includes('fullscreen')"
        type="button"
        class="headerbar-button"
        :class="{ 'is-active': fullscreen }"
        :aria-label="fullscreenLabel"
        :title="fullscreenTitle"
        @click="emit('fullscreen')"
      >
        <Icon
          :icon="fullscreen ? 'solar:quit-full-screen-square-bold' : 'solar:full-screen-square-bold'"
          class="headerbar-icon"
        />
      </button>
      <el-tooltip content="隐藏到托盘" placement="bottom">
        <button
          v-if="controls.includes('close')"
          type="button"
          class="headerbar-button headerbar-button-close"
          aria-label="关闭窗口"
          title="关闭"
          @click="emit('close')"
        >
          <Icon icon="solar:close-square-bold" class="headerbar-icon" />
        </button>
      </el-tooltip>
    </nav>
  </header>
</template>

<style scoped>
.window-headerbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--cpms-space-base);
  flex: none;
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
  font-size: var(--cpms-font-size-base);
  font-weight: var(--cpms-font-weight-semibold);
  line-height: var(--cpms-line-height-small);
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
  width: 30px;
  height: 30px;
  padding: 0;
  border: 0;
  border-radius: var(--cpms-radius-small);
  background: transparent;
  color: var(--cpms-color-text-secondary);
  cursor: pointer;
  transition:
    color var(--cpms-duration-fast) var(--cpms-easing-base),
    background-color var(--cpms-duration-fast) var(--cpms-easing-base),
    transform var(--cpms-duration-fast) var(--cpms-easing-base);
}

.headerbar-button .headerbar-icon {
  width: 16px;
  height: 16px;
  color: currentcolor;
  fill: currentcolor;
}

.headerbar-button:hover {
  background: var(--cpms-color-bg-hover);
  color: var(--cpms-color-text-primary);
}

.headerbar-button:active {
  transform: scale(0.96);
}

.headerbar-button.is-active {
  color: var(--cpms-color-primary);
  background: var(--cpms-color-primary-bg);
}

.headerbar-button-close:hover {
  background: var(--cpms-color-danger-bg);
  color: var(--cpms-color-danger);
}

@media (prefers-reduced-motion: reduce) {
  .headerbar-button {
    transition: none;
  }

  .headerbar-button:active {
    transform: none;
  }
}
</style>
