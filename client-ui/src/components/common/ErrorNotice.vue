<script setup lang="ts" name="ErrorNotice">
import { useAppError } from "@/composables/useAppError";

const { latestError, clearError } = useAppError();
</script>

<template>
  <el-alert
    v-if="latestError"
    class="error-notice"
    :type="latestError.level === 'error' ? 'error' : 'warning'"
    :title="latestError.message"
    :description="`${latestError.source} / ${latestError.code}`"
    show-icon
    closable
    @close="clearError(latestError.id)"
  />
</template>

<style scoped>
.error-notice {
  margin-bottom: var(--cpms-space-base);
  border-radius: var(--cpms-radius-small);
}
</style>
