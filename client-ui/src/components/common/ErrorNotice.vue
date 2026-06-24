<script setup lang="ts" name="ErrorNotice">
import { useAppError } from "@/composables/useAppError";

const { latestError } = useAppError();
</script>

<template>
  <Alert v-if="latestError" class="error-notice" :variant="latestError.level === 'error' ? 'destructive' : 'default'">
    <AlertTitle>{{ latestError.message }}</AlertTitle>
    <AlertDescription>{{ `${latestError.source} / ${latestError.code}` }}</AlertDescription>
  </Alert>
</template>

<style scoped>
.error-notice {
  margin-bottom: var(--cpms-space-base);
  border-radius: var(--cpms-radius-panel);
  border: 1px solid var(--cpms-color-border);
  background: var(--cpms-color-surface);
}
</style>
