<script setup lang="ts">
defineProps<{
  content?: string;
  placement?: string;
}>();
</script>

<template>
  <span class="cpms-tooltip-trigger" :title="content">
    <slot />
  </span>
</template>
