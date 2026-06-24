<script setup lang="ts" name="WindowFrame">
import { LoaderCircle } from '@lucide/vue'
import WindowHeaderBar from '@/components/layout/WindowHeaderBar.vue'

type WindowControl = 'entry' | 'pin' | 'collapse' | 'fullscreen' | 'close'
type ClassValue = string | string[] | Record<string, boolean>

withDefaults(
  defineProps<{
    title: string
    icon?: string
    pinned?: boolean
    fullscreen?: boolean
    controls?: WindowControl[]
    loading?: boolean
    loadingText?: string
    bodyClass?: ClassValue
  }>(),
  {
    icon: '',
    pinned: false,
    fullscreen: false,
    controls: () => ['pin', 'collapse', 'fullscreen', 'close'],
    loading: false,
    loadingText: '加载中…',
    bodyClass: ''
  }
)

const emit = defineEmits<{
  entry: []
  pin: []
  collapse: []
  fullscreen: []
  close: []
}>()
</script>

<template>
  <section class="window-frame-shell">
    <section class="window-frame">
      <WindowHeaderBar
        :title="title"
        :icon="icon"
        :pinned="pinned"
        :fullscreen="fullscreen"
        :controls="controls"
        @entry="emit('entry')"
        @pin="emit('pin')"
        @collapse="emit('collapse')"
        @fullscreen="emit('fullscreen')"
        @close="emit('close')"
      />
      <main class="window-frame-body" :class="bodyClass" :aria-busy="loading">
        <slot />
        <div v-if="loading" class="window-frame-loading" role="status">
          <LoaderCircle class="window-frame-loading-icon" aria-hidden="true" />
          <span>{{ loadingText }}</span>
        </div>
      </main>
    </section>
  </section>
</template>

<style scoped>
.window-frame-shell {
  width: 100%;
  height: 100%;
  padding: 0;
}

.window-frame {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.window-frame-body {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1 1 auto;
  min-height: 0;
  width: 100%;
  background: var(--cpms-color-bg-app);
  overflow: hidden;
}

.window-frame-loading {
  position: absolute;
  inset: 0;
  z-index: 20;
  display: grid;
  place-items: center;
  align-content: center;
  gap: var(--cpms-space-3);
  color: var(--cpms-color-text-secondary);
  background: rgba(238, 243, 248, 0.86);
  backdrop-filter: blur(6px);
}

.window-frame-loading-icon {
  width: 28px;
  height: 28px;
  color: var(--cpms-color-primary);
  animation: cpms-spin 1s linear infinite;
}

@media (prefers-reduced-motion: reduce) {
  .window-frame-loading-icon {
    animation: none;
  }
}
</style>
