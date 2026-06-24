<script setup lang="ts">
import { inject, onMounted } from "vue";

const props = defineProps<{
  label: string;
  name: string;
}>();

const tabs = inject<{
  active: { modelValue: string };
  register: (tab: { name: string; label: string }) => void;
}>("cpms-tabs");

onMounted(() => {
  tabs?.register({ name: props.name, label: props.label });
});
</script>

<template>
  <section v-show="tabs?.active.modelValue === name" class="cpms-tab-pane" role="tabpanel">
    <slot />
  </section>
</template>
