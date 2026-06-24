<script setup lang="ts">
import { computed } from 'vue'
import { House, Maximize, Minimize2, Minus, Pin, PinOff, X } from '@lucide/vue'
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
const pinIcon = computed(() => (props.pinned ? PinOff : Pin))
const fullscreenTitle = computed(() => (props.fullscreen ? '退出全屏' : '全屏'))
const fullscreenIcon = computed(() => (props.fullscreen ? Minimize2 : Maximize))

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
    <div class="flex items-center gap-2 min-w-0" data-tauri-drag-region>
      <img v-if="icon" :src="icon" class="headerbar-logo" alt="应用图标" data-tauri-drag-region />
      <strong class="headerbar-text" data-tauri-drag-region>{{ title }}</strong>
    </div>
    <nav class="flex items-center gap-1 flex-none">
      <House
        v-if="controls.includes('entry')"
        class="headerbar-icon-control"
        role="button"
        tabindex="0"
        title="入口页"
        @click="activateControl('entry')"
        @keydown.enter.prevent="activateControl('entry')"
        @keydown.space.prevent="activateControl('entry')"
      />
      <component
        v-if="controls.includes('pin')"
        :is="pinIcon"
        class="headerbar-icon-control"
        :class="{ 'is-active': pinned }"
        role="button"
        tabindex="0"
        :title="pinTitle"
        @click="activateControl('pin')"
        @keydown.enter.prevent="activateControl('pin')"
        @keydown.space.prevent="activateControl('pin')"
      />
      <Minus
        v-if="controls.includes('collapse')"
        class="headerbar-icon-control"
        role="button"
        tabindex="0"
        title="收起"
        @click="activateControl('collapse')"
        @keydown.enter.prevent="activateControl('collapse')"
        @keydown.space.prevent="activateControl('collapse')"
      />
      <component
        v-if="controls.includes('fullscreen')"
        :is="fullscreenIcon"
        class="headerbar-icon-control"
        :class="{ 'is-active': fullscreen }"
        role="button"
        tabindex="0"
        :title="fullscreenTitle"
        @click="activateControl('fullscreen')"
        @keydown.enter.prevent="activateControl('fullscreen')"
        @keydown.space.prevent="activateControl('fullscreen')"
      />
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger as-child>
            <X
              v-if="controls.includes('close')"
              class="headerbar-icon-control headerbar-icon-control-close"
              role="button"
              tabindex="0"
              title="关闭"
              @click="activateControl('close')"
              @keydown.enter.prevent="activateControl('close')"
              @keydown.space.prevent="activateControl('close')"
            />
          </TooltipTrigger>
          <TooltipContent side="bottom">隐藏到托盘</TooltipContent>
        </Tooltip>
      </TooltipProvider>
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

.headerbar-icon-control {
  display: inline-grid;
  place-items: center;
  width: 35px;
  height: 35px;
  padding: 8px;
  color: var(--cpms-color-text-secondary);
  cursor: pointer;
  box-sizing: border-box;
  transition:
    color var(--cpms-duration-fast) var(--cpms-easing-base),
    background-color var(--cpms-duration-fast) var(--cpms-easing-base),
    transform var(--cpms-duration-fast) var(--cpms-easing-base);
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
