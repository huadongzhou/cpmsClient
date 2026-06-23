<script setup lang="ts" name="WindowFrame">
import { vLoading } from 'element-plus'
import 'element-plus/es/components/loading/style/css'
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
      <main v-loading="loading" class="window-frame-body" :class="bodyClass" :element-loading-text="loadingText">
        <slot />
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
  border-radius: var(--cpms-radius-large);
  overflow: hidden;
}

.window-frame-body {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1 1 auto;
  min-height: 0;
  width: 100%;
  background: #fff;
  overflow: hidden;
}
</style>
