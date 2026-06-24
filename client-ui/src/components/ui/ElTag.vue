<script setup lang="ts">
withDefaults(
  defineProps<{
    type?: "success" | "warning" | "danger" | "info" | "primary";
    size?: "small" | "default";
  }>(),
  {
    type: "info",
    size: "default",
  },
);
</script>

<template>
  <span :class="['cpms-tag', `cpms-tag--${type}`, `cpms-tag--${size}`]">
    <slot />
  </span>
</template>
