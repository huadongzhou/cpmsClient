<script setup lang="ts" name="WindowHeaderBar">
import { computed } from 'vue'
import {
  House,
  Maximize,
  Minimize2,
  Minus,
  Pin,
  PinOff,
  X,
} from '@lucide/vue'

type WindowControl = 'entry' | 'pin' | 'collapse' | 'fullscreen' | 'close'

const props = withDefaults(
  defineProps<{
    title: string
    icon?: string
    pinned?: boolean
    fullscreen?: boolean
    controls?: WindowControl[]
  }>(),
  {
    icon: '',
    pinned: false,
    fullscreen: false,
    controls: () => ['pin', 'collapse', 'fullscreen', 'close']
  }
)

const emit = defineEmits<{
  entry: []
  pin: []
  collapse: []
  fullscreen: []
  close: []
}>()

const pinLabel = computed(() => (props.pinned ? '取消固定窗口' : '固定窗口'))
const pinTitle = computed(() => (props.pinned ? '取消固定' : '固定'))
const pinIcon = computed(() => (props.pinned ? PinOff : Pin))
const fullscreenLabel = computed(() => (props.fullscreen ? '退出全屏' : '全屏窗口'))
const fullscreenTitle = computed(() => (props.fullscreen ? '退出全屏' : '全屏'))
const fullscreenIcon = computed(() => (props.fullscreen ? Minimize2 : Maximize))
</script>

<template>
  <header class="window-headerbar" data-tauri-drag-region>
    <div class="headerbar-title" data-tauri-drag-region>
      <img v-if="icon" :src="icon" class="headerbar-logo" alt="应用图标" data-tauri-drag-region />
      <strong class="headerbar-text" data-tauri-drag-region>{{ title }}</strong>
    </div>
    <nav class="headerbar-actions" aria-label="窗口控制">
      <button
        v-if="controls.includes('entry')"
        type="button"
        class="headerbar-button"
        aria-label="回到入口页"
        title="入口页"
        @click="emit('entry')"
      >
        <el-icon class="headerbar-icon"><House /></el-icon>
      </button>
      <button
        v-if="controls.includes('pin')"
        type="button"
        class="headerbar-button"
        :class="{ 'is-active': pinned }"
        :aria-label="pinLabel"
        :title="pinTitle"
        @click="emit('pin')"
      >
        <el-icon class="headerbar-icon"><component :is="pinIcon" /></el-icon>
      </button>
      <button
        v-if="controls.includes('collapse')"
        type="button"
        class="headerbar-button"
        aria-label="收起窗口"
        title="收起"
        @click="emit('collapse')"
      >
        <el-icon class="headerbar-icon"><Minus /></el-icon>
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
        <el-icon class="headerbar-icon"><component :is="fullscreenIcon" /></el-icon>
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
          <el-icon class="headerbar-icon"><X /></el-icon>
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
  background: var(--cpms-color-foreground);
  border-bottom: 1px solid rgba(255, 255, 255, 0.16);
  color: var(--cpms-color-text-on-primary);
  backdrop-filter: blur(12px);
  user-select: none;
}

.headerbar-title {
  display: flex;
  align-items: center;
  gap: var(--cpms-space-small);
  min-width: 0;
}

.headerbar-logo {
  width: 28px;
  height: 28px;
  flex: none;
  border-radius: 8px;
}

.headerbar-text {
  font-size: var(--cpms-font-size-base);
  font-weight: var(--cpms-font-weight-semibold);
  line-height: var(--cpms-line-height-small);
  color: var(--cpms-color-text-on-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.headerbar-actions {
  display: flex;
  align-items: center;
  gap: var(--cpms-space-1);
  flex: none;
}

.headerbar-button {
  display: inline-grid;
  place-items: center;
  width: 44px;
  height: 44px;
  padding: 0;
  border: 0;
  border-radius: 10px;
  background: transparent;
  color: rgba(255, 255, 255, 0.75);
  cursor: pointer;
  transition:
    color var(--cpms-duration-fast) var(--cpms-easing-base),
    background-color var(--cpms-duration-fast) var(--cpms-easing-base),
    transform var(--cpms-duration-fast) var(--cpms-easing-base);
}

.headerbar-button .headerbar-icon {
  width: 18px;
  height: 18px;
  color: currentcolor;
}

.headerbar-button:hover {
  background: rgba(255, 255, 255, 0.1);
  color: var(--cpms-color-text-on-primary);
}

.headerbar-button:focus-visible {
  outline: 2px solid var(--cpms-color-primary-border);
  outline-offset: 2px;
}

.headerbar-button:active {
  transform: scale(0.96);
}

.headerbar-button.is-active {
  color: var(--cpms-color-primary-border);
  background: rgba(255, 255, 255, 0.12);
}

.headerbar-button-close:hover {
  background: var(--cpms-color-danger);
  color: var(--cpms-color-text-on-primary);
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
