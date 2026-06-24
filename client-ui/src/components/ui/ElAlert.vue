<script setup lang="ts">
import { CircleAlert, Info, TriangleAlert } from "@lucide/vue";

const props = withDefaults(
  defineProps<{
    type?: "error" | "warning" | "info" | "success";
    title?: string;
    description?: string;
    closable?: boolean;
    showIcon?: boolean;
  }>(),
  {
    type: "info",
    title: "",
    description: "",
    closable: false,
    showIcon: false,
  },
);

defineEmits<{
  close: [];
}>();

const iconMap = {
  error: CircleAlert,
  warning: TriangleAlert,
  info: Info,
  success: Info,
};
</script>

<template>
  <section :class="['cpms-alert', `cpms-alert--${props.type}`]" role="alert">
    <component :is="iconMap[props.type]" v-if="showIcon" class="cpms-alert__icon" aria-hidden="true" />
    <div class="cpms-alert__content">
      <strong v-if="title">{{ title }}</strong>
      <p v-if="description">{{ description }}</p>
    </div>
    <button v-if="closable" class="cpms-alert__close" type="button" aria-label="关闭提示" @click="$emit('close')">
      ×
    </button>
  </section>
</template>
