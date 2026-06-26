<script setup lang="ts" name="ErrorNotice">
import { useAppError } from "@/composables/useAppError";

const { latestError } = useAppError();
</script>

<template>
  <el-alert
    v-if="latestError"
    class="error-notice"
    :type="latestError.level === 'error' ? 'error' : 'info'"
    :title="latestError.message"
    :description="`${latestError.source} / ${latestError.code}`"
    :closable="false"
    show-icon
  />
</template>

<style scoped>
.error-notice {
  margin-bottom: var(--cpms-space-base);
  border-radius: var(--cpms-radius-panel);
  border: 1px solid var(--cpms-color-border);
  background: var(--cpms-color-surface);
}
</style>
