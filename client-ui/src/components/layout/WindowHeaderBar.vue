<script setup lang="ts" name="WindowHeaderBar">
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
    controls: () => []
  }
)

const emit = defineEmits<{
  entry: []
  pin: []
  collapse: []
  fullscreen: []
  close: []
}>()

const pinTitle = computed(() => (props.pinned ? '取消固定' : '固定'))
const fullscreenTitle = computed(() => (props.fullscreen ? '退出全屏' : '全屏'))

function activateControl(control: WindowControl) {
  switch (control) {
    case 'entry':
      emit('entry')
      break
    case 'pin':
      emit('pin')
      break
    case 'collapse':
      emit('collapse')
      break
    case 'fullscreen':
      emit('fullscreen')
      break
    case 'close':
      emit('close')
      break
  }
}
</script>

<template>
  <header class="window-headerbar" data-tauri-drag-region>
    <div class="headerbar-title-group" data-tauri-drag-region>
      <img v-if="icon" :src="icon" class="headerbar-logo" alt="应用图标" data-tauri-drag-region />
      <strong class="headerbar-text" data-tauri-drag-region>{{ title }}</strong>
    </div>
    <nav class="headerbar-controls">
      <button
        v-if="controls.includes('entry')"
        type="button"
        class="headerbar-icon-control"
        title="入口页"
        @click="activateControl('entry')"
        @keydown.enter.prevent="activateControl('entry')"
        @keydown.space.prevent="activateControl('entry')"
      >
        <i class="el-icon-s-home" aria-hidden="true" />
      </button>
      <button
        v-if="controls.includes('pin')"
        type="button"
        class="headerbar-icon-control"
        :class="{ 'is-active': pinned }"
        :title="pinTitle"
        @click="activateControl('pin')"
        @keydown.enter.prevent="activateControl('pin')"
        @keydown.space.prevent="activateControl('pin')"
      >
        <svg
          class="headerbar-control-svg"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <template v-if="pinned">
            <line x1="2" y1="2" x2="22" y2="22" />
            <line x1="12" y1="17" x2="12" y2="22" />
            <path d="M9 9v1.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16h11" />
            <path d="M15 9.34V6h1a2 2 0 0 0 0-4H7.89" />
          </template>
          <template v-else>
            <line x1="12" y1="17" x2="12" y2="22" />
            <path d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z" />
          </template>
        </svg>
      </button>
      <button
        v-if="controls.includes('collapse')"
        type="button"
        class="headerbar-icon-control"
        title="收起"
        @click="activateControl('collapse')"
        @keydown.enter.prevent="activateControl('collapse')"
        @keydown.space.prevent="activateControl('collapse')"
      >
        <i class="el-icon-minus" aria-hidden="true" />
      </button>
      <button
        v-if="controls.includes('fullscreen')"
        type="button"
        class="headerbar-icon-control"
        :class="{ 'is-active': fullscreen }"
        :title="fullscreenTitle"
        @click="activateControl('fullscreen')"
        @keydown.enter.prevent="activateControl('fullscreen')"
        @keydown.space.prevent="activateControl('fullscreen')"
      >
        <i :class="fullscreen ? 'el-icon-copy-document' : 'el-icon-full-screen'" aria-hidden="true" />
      </button>
      <el-tooltip
        v-if="controls.includes('close')"
        content="隐藏到托盘"
        placement="bottom"
      >
        <button
          type="button"
          class="headerbar-icon-control headerbar-icon-control-close"
          title="关闭"
          @click="activateControl('close')"
          @keydown.enter.prevent="activateControl('close')"
          @keydown.space.prevent="activateControl('close')"
        >
          <i class="el-icon-close" aria-hidden="true" />
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
  padding-left: var(--cpms-space-base);
  background: var(--cpms-color-bg-panel);
  border-bottom: 1px solid var(--cpms-color-border);
  color: var(--cpms-color-text-primary);
  backdrop-filter: blur(12px);
  user-select: none;
}

.headerbar-title-group {
  display: flex;
  align-items: center;
  gap: var(--cpms-space-2);
  min-width: 0;
}

.headerbar-logo {
  width: 22px;
  height: 22px;
  flex: none;
  border-radius: 6px;
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

.headerbar-controls {
  display: flex;
  align-items: center;
  gap: var(--cpms-space-1);
  flex: none;
}

.headerbar-icon-control {
  display: inline-grid;
  place-items: center;
  width: 35px;
  height: 35px;
  padding: 8px;
  margin: 0;
  border: 0;
  background: transparent;
  color: var(--cpms-color-text-secondary);
  cursor: pointer;
  box-sizing: border-box;
  font-size: 18px;
  line-height: 1;
  transition:
    color var(--cpms-duration-fast) var(--cpms-easing-base),
    background-color var(--cpms-duration-fast) var(--cpms-easing-base),
    transform var(--cpms-duration-fast) var(--cpms-easing-base);
}

.headerbar-control-svg {
  width: 18px;
  height: 18px;
}

.headerbar-icon-control:hover {
  background: var(--cpms-color-bg-hover);
  color: var(--cpms-color-text-primary);
}

.headerbar-icon-control:focus-visible {
  outline: none;
}

.headerbar-icon-control:active {
  transform: scale(0.96);
}

.headerbar-icon-control.is-active {
  color: var(--cpms-color-primary-text);
  background: var(--cpms-color-primary-bg);
}

.headerbar-icon-control-close:hover {
  background: var(--cpms-color-danger);
  color: var(--cpms-color-text-on-primary);
}

@media (prefers-reduced-motion: reduce) {
  .headerbar-icon-control {
    transition: none;
  }

  .headerbar-icon-control:active {
    transform: none;
  }
}
</style>
