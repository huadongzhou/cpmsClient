<script setup lang="ts">
defineProps<{
  modelValue?: string | number;
  placeholder?: string;
}>();

defineEmits<{
  "update:modelValue": [value: string];
}>();
</script>

<template>
  <select
    class="cpms-select"
    :value="modelValue"
    :aria-label="placeholder"
    @change="$emit('update:modelValue', ($event.target as HTMLSelectElement).value)"
  >
    <slot />
  </select>
</template>
