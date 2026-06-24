<script setup lang="ts">
import { Inbox } from "@lucide/vue";

defineProps<{
  description?: string;
}>();
</script>

<template>
  <div class="cpms-empty">
    <Inbox class="cpms-empty__icon" aria-hidden="true" />
    <p>{{ description || "暂无数据" }}</p>
  </div>
</template>
