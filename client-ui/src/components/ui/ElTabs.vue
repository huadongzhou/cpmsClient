<script setup lang="ts">
import { provide, reactive } from "vue";

type TabItem = {
  name: string;
  label: string;
};

const props = defineProps<{
  modelValue: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

const tabs = reactive<TabItem[]>([]);

provide("cpms-tabs", {
  active: props,
  register(tab: TabItem) {
    if (!tabs.some((item) => item.name === tab.name)) {
      tabs.push(tab);
    }
  },
});
</script>

<template>
  <div class="cpms-tabs">
    <div class="cpms-tabs__list" role="tablist">
      <button
        v-for="tab in tabs"
        :key="tab.name"
        class="cpms-tabs__trigger"
        :class="{ 'is-active': modelValue === tab.name }"
        type="button"
        role="tab"
        :aria-selected="modelValue === tab.name"
        @click="emit('update:modelValue', tab.name)"
      >
        {{ tab.label }}
      </button>
    </div>
    <div class="cpms-tabs__content">
      <slot />
    </div>
  </div>
</template>
