<script setup lang="ts">
withDefaults(
  defineProps<{
    size?: number | string;
  }>(),
  {
    size: undefined,
  },
);
</script>

<template>
  <span class="cpms-icon" :style="size ? { width: `${size}px`, height: `${size}px` } : undefined">
    <slot />
  </span>
</template>
