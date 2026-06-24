<script setup lang="ts">
import { LoaderCircle } from "@lucide/vue";
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    type?: "primary" | "success" | "warning" | "danger" | "info" | "default";
    plain?: boolean;
    circle?: boolean;
    loading?: boolean;
    disabled?: boolean;
  }>(),
  {
    type: "default",
    plain: false,
    circle: false,
    loading: false,
    disabled: false,
  },
);

const classes = computed(() => [
  "cpms-button",
  `cpms-button--${props.type}`,
  {
    "is-plain": props.plain,
    "is-circle": props.circle,
    "is-loading": props.loading,
  },
]);
</script>

<template>
  <button :class="classes" type="button" :disabled="disabled || loading">
    <LoaderCircle v-if="loading" class="cpms-button__spinner" aria-hidden="true" />
    <span v-if="$slots.icon && !loading" class="cpms-button__icon" aria-hidden="true">
      <slot name="icon" />
    </span>
    <slot />
  </button>
</template>
