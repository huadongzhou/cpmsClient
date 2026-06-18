<script setup lang="ts" name="Icon">
import { addCollection, renderSVG } from "@iconify/iconify";
import { computed, nextTick, ref, watch } from "vue";
import { solarIconBodies } from "@/components/common/solarIconBodies";

const props = withDefaults(
  defineProps<{
    icon?: string;
    color?: string;
    size?: number | string;
    svgClass?: string;
  }>(),
  {
    icon: "",
    color: undefined,
    size: 16,
    svgClass: "",
  },
);

const elRef = ref<HTMLElement | null>(null);

addCollection({
  prefix: "solar",
  width: 24,
  height: 24,
  icons: Object.fromEntries(
    Object.entries(solarIconBodies).map(([name, body]) => [name, { body }]),
  ),
});

const isLocal = computed(() => props.icon.startsWith("svg-icon:"));
const symbolId = computed(() =>
  isLocal.value ? `#icon-${props.icon.split("svg-icon:")[1]}` : props.icon,
);
const normalizedSize = computed(() =>
  typeof props.size === "number" ? props.size : Number.parseFloat(props.size) || 16,
);
const iconifyStyle = computed(() => ({
  fontSize: `${normalizedSize.value}px`,
  height: "1em",
  color: props.color,
}));
const svgClassName = computed(() => ["iconify", props.svgClass].filter(Boolean).join(" "));

async function updateIcon(icon: string) {
  if (isLocal.value) {
    return;
  }

  await nextTick();

  const el = elRef.value;
  if (!el || !icon) {
    return;
  }

  const svg = renderSVG(icon, {
    height: "1em",
    width: "1em",
  });

  el.textContent = "";

  if (svg) {
    svg.setAttribute("aria-hidden", "true");
    svg.setAttribute("focusable", "false");
    if (props.svgClass) {
      svg.setAttribute("class", svgClassName.value);
    }
    el.appendChild(svg);
    return;
  }

  const span = document.createElement("span");
  span.className = svgClassName.value;
  span.dataset.icon = icon;
  el.appendChild(span);
}

watch(() => props.icon, updateIcon, { immediate: true });
</script>

<template>
  <el-icon class="cpms-icon" :color="color" :size="normalizedSize">
    <svg v-if="isLocal" :class="svgClassName" aria-hidden="true" focusable="false">
      <use :xlink:href="symbolId" />
    </svg>
    <span v-else ref="elRef" class="cpms-icon-host" :style="iconifyStyle" aria-hidden="true" />
  </el-icon>
</template>

<style scoped>
.cpms-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: inherit;
  line-height: 1;
}

.cpms-icon-host {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1em;
  height: 1em;
  color: inherit;
  line-height: 1;
}

.cpms-icon :deep(svg) {
  display: block;
  width: 1em;
  height: 1em;
  color: currentcolor;
  fill: currentcolor;
}
</style>
