<script setup lang="ts">
import { useAttrs } from "vue";

defineOptions({
  inheritAttrs: false,
});

const props = withDefaults(
  defineProps<{
    modelValue: boolean;
    size?: string;
    showClose?: boolean;
    destroyOnClose?: boolean;
  }>(),
  {
    size: "80%",
    showClose: true,
    destroyOnClose: false,
  },
);

const emit = defineEmits<{
  "update:modelValue": [value: boolean];
}>();
const attrs = useAttrs();

function close() {
  emit("update:modelValue", false);
}
</script>

<template>
  <Teleport to="body">
    <div v-if="modelValue" class="el-drawer__overlay" role="presentation" @click.self="close">
      <aside
        class="el-drawer"
        :class="attrs.class"
        role="dialog"
        aria-modal="true"
        :style="{ width: props.size }"
      >
        <header class="el-drawer__header">
          <slot name="header" />
          <button v-if="showClose" class="el-drawer__close" type="button" aria-label="关闭" @click="close">×</button>
        </header>
        <div class="el-drawer__body">
          <slot />
        </div>
      </aside>
    </div>
  </Teleport>
</template>
