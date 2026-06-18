<script setup lang="ts" name="EntryView">
import { ElMessage } from "element-plus";
import { setClientIframeUrl } from "@/api/tauri/desktop";
import Icon from "@/components/common/Icon.vue";

const LAST_IFRAME_URL_KEY = "cpms-last-iframe-url";

const url = ref(localStorage.getItem(LAST_IFRAME_URL_KEY) ?? "");
const loading = ref(false);
const error = ref("");

const recentUrl = computed(() => localStorage.getItem(LAST_IFRAME_URL_KEY) ?? "");
const hasRecentUrl = computed(() => recentUrl.value.length > 0);
const isSubmittable = computed(() => !loading.value && url.value.trim().length > 0);

function validate(input: string): string | null {
  const trimmed = input.trim();
  if (!trimmed) return "请输入业务入口地址";
  try {
    const parsed = new URL(trimmed);
    if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
      return "仅支持 http/https 协议";
    }
  } catch {
    return "地址格式不正确";
  }
  return null;
}

async function loadUrl(input?: string) {
  const target = (input ?? url.value).trim();
  const validationError = validate(target);

  if (validationError) {
    error.value = validationError;
    return;
  }

  error.value = "";
  loading.value = true;
  try {
    await setClientIframeUrl(target);
    localStorage.setItem(LAST_IFRAME_URL_KEY, target);
    if (input) url.value = target;
  } catch (err) {
    error.value = err instanceof Error ? err.message : "地址设置失败";
  } finally {
    loading.value = false;
  }
}

function useRecentUrl() {
  if (!recentUrl.value) return;
  url.value = recentUrl.value;
  void ElMessage.info("已填入最近使用的地址");
}
</script>

<template>
  <main class="entry-view">
    <div class="entry-decoration" aria-hidden="true" />

    <section class="entry-card">
      <div class="entry-brand">
        <div class="entry-icon">
          <Icon icon="solar:server-square-update-bold" class="entry-icon-svg" />
        </div>
        <h1 class="entry-title">欢迎来到 CPMS Client</h1>
        <p class="entry-desc">请输入 hub-platform 业务入口地址，客户端将加载该页面。</p>
      </div>

      <div class="entry-form">
        <div class="entry-input-wrap" :class="{ 'has-error': error }">
          <span class="entry-input-prefix" aria-hidden="true">
            <Icon icon="solar:link-minimalistic-bold" />
          </span>
          <input
            v-model="url"
            class="entry-input"
            type="text"
            placeholder="例如：http://192.168.1.10:8085"
            :disabled="loading"
            @keyup.enter="loadUrl()"
          />
          <button
            v-if="url"
            class="entry-input-clear"
            type="button"
            aria-label="清空"
            @click="url = ''"
          >
            <Icon icon="solar:close-circle-bold" />
          </button>
        </div>

        <p v-if="error" class="entry-error" role="alert">
          <Icon icon="solar:danger-triangle-bold" class="entry-error-icon" />
          <span>{{ error }}</span>
        </p>

        <button type="button" class="entry-submit" :disabled="!isSubmittable" @click="loadUrl()">
          <span v-if="loading" class="entry-spinner" aria-hidden="true" />
          <span>{{ loading ? "正在加载…" : "加载业务页面" }}</span>
        </button>

        <div v-if="hasRecentUrl" class="entry-recent">
          <span class="entry-recent-label">最近使用</span>
          <button
            type="button"
            class="entry-recent-chip"
            :disabled="loading"
            @click="loadUrl(recentUrl)"
          >
            {{ recentUrl }}
          </button>
          <button
            type="button"
            class="entry-recent-action"
            aria-label="填入最近地址"
            title="填入"
            @click="useRecentUrl"
          >
            <Icon icon="solar:pen-new-square-bold" />
          </button>
        </div>

        <p class="entry-tip">
          地址将经客户端校验，仅允许 http/https 协议；显式配置白名单时还会校验域名。
        </p>
      </div>
    </section>
  </main>
</template>

<style scoped>
.entry-view {
  position: relative;
  display: grid;
  place-items: center;
  flex: 1 1 auto;
  min-height: 0;
  width: 100%;
  padding: var(--cpms-space-5);
  background: var(--cpms-color-bg-app);
  overflow: hidden;
}

.entry-decoration {
  position: absolute;
  inset: 0;
  pointer-events: none;
  background:
    radial-gradient(circle at 12% 18%, rgba(37, 99, 235, 0.06) 0%, transparent 38%),
    radial-gradient(circle at 88% 82%, rgba(5, 150, 105, 0.04) 0%, transparent 38%);
}

.entry-card {
  position: relative;
  z-index: 1;
  width: 100%;
  max-width: 520px;
  padding: var(--cpms-space-10) var(--cpms-space-8);
  background: var(--cpms-color-surface);
  border: 1px solid var(--cpms-color-border);
  border-radius: var(--cpms-radius-xl);
  box-shadow: var(--cpms-shadow-lg);
  text-align: center;
}

.entry-brand {
  display: grid;
  gap: var(--cpms-space-3);
  justify-items: center;
  margin-bottom: var(--cpms-space-8);
}

.entry-icon {
  display: grid;
  place-items: center;
  width: 60px;
  height: 60px;
  color: var(--cpms-color-text-on-primary);
  background: linear-gradient(
    135deg,
    var(--cpms-color-primary) 0%,
    var(--cpms-color-primary-hover) 100%
  );
  border-radius: var(--cpms-radius-large);
  box-shadow: 0 8px 20px rgba(37, 99, 235, 0.28);
}

.entry-icon-svg {
  width: 28px;
  height: 28px;
  color: currentcolor;
  fill: currentcolor;
}

.entry-title {
  margin: 0;
  font-size: var(--cpms-font-size-2xl);
  font-weight: var(--cpms-font-weight-bold);
  line-height: var(--cpms-line-height-tight);
  color: var(--cpms-color-text-primary);
}

.entry-desc {
  margin: 0;
  font-size: var(--cpms-font-size-base);
  color: var(--cpms-color-text-muted);
  line-height: var(--cpms-line-height-relaxed);
}

.entry-form {
  display: grid;
  gap: var(--cpms-space-3);
}

.entry-input-wrap {
  position: relative;
  display: flex;
  align-items: center;
  width: 100%;
  transition: box-shadow var(--cpms-duration-base) var(--cpms-easing-base);
  border-radius: var(--cpms-radius-panel);
}

.entry-input-wrap:focus-within {
  box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.12);
}

.entry-input-wrap.has-error:focus-within {
  box-shadow: 0 0 0 3px rgba(220, 38, 38, 0.1);
}

.entry-input-prefix {
  position: absolute;
  left: 14px;
  display: grid;
  place-items: center;
  color: var(--cpms-color-text-muted);
  pointer-events: none;
  font-size: 16px;
}

.entry-input {
  width: 100%;
  height: 48px;
  padding: 0 40px 0 42px;
  font-size: var(--cpms-font-size-base);
  color: var(--cpms-color-text-primary);
  background: var(--cpms-color-bg-panel);
  border: 1px solid var(--cpms-color-border);
  border-radius: var(--cpms-radius-panel);
  outline: none;
  transition: border-color var(--cpms-duration-base) var(--cpms-easing-base);
}

.entry-input::placeholder {
  color: var(--cpms-color-text-disabled);
}

.entry-input:hover {
  border-color: var(--cpms-color-border-strong);
}

.entry-input:focus {
  border-color: var(--cpms-color-primary);
}

.entry-input-wrap.has-error .entry-input {
  border-color: var(--cpms-color-danger-border);
  background: var(--cpms-color-danger-bg);
}

.entry-input-wrap.has-error .entry-input:focus {
  border-color: var(--cpms-color-danger);
}

.entry-input:disabled {
  background: var(--cpms-color-bg-hover);
  cursor: not-allowed;
}

.entry-input-clear {
  position: absolute;
  right: 12px;
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  padding: 0;
  color: var(--cpms-color-text-muted);
  background: transparent;
  border: 0;
  border-radius: var(--cpms-radius-full);
  cursor: pointer;
  font-size: 14px;
  transition:
    color var(--cpms-duration-fast) var(--cpms-easing-base),
    background-color var(--cpms-duration-fast) var(--cpms-easing-base);
}

.entry-input-clear:hover {
  color: var(--cpms-color-text-primary);
  background: var(--cpms-color-bg-hover);
}

.entry-error {
  display: flex;
  align-items: center;
  gap: var(--cpms-space-1);
  margin: 0;
  text-align: left;
  font-size: var(--cpms-font-size-small);
  color: var(--cpms-color-danger);
  line-height: var(--cpms-line-height-small);
}

.entry-error-icon {
  font-size: 14px;
}

.entry-submit {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--cpms-space-2);
  width: 100%;
  height: 48px;
  padding: 0 var(--cpms-space-4);
  font-size: var(--cpms-font-size-base);
  font-weight: var(--cpms-font-weight-medium);
  color: var(--cpms-color-text-on-primary);
  background: linear-gradient(
    135deg,
    var(--cpms-color-primary) 0%,
    var(--cpms-color-primary-hover) 100%
  );
  border: 0;
  border-radius: var(--cpms-radius-panel);
  cursor: pointer;
  box-shadow: 0 4px 14px rgba(37, 99, 235, 0.26);
  transition:
    transform var(--cpms-duration-base) var(--cpms-easing-out),
    box-shadow var(--cpms-duration-base) var(--cpms-easing-base),
    opacity var(--cpms-duration-base) var(--cpms-easing-base);
}

.entry-submit:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 6px 18px rgba(37, 99, 235, 0.3);
}

.entry-submit:active:not(:disabled) {
  transform: translateY(0);
}

.entry-submit:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.entry-spinner {
  width: 16px;
  height: 16px;
  border: 2px solid rgba(255, 255, 255, 0.35);
  border-top-color: var(--cpms-color-text-on-primary);
  border-radius: 50%;
  animation: entry-spin 0.8s linear infinite;
}

@keyframes entry-spin {
  to {
    transform: rotate(360deg);
  }
}

.entry-recent {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--cpms-space-2);
  margin-top: var(--cpms-space-1);
}

.entry-recent-label {
  font-size: var(--cpms-font-size-small);
  color: var(--cpms-color-text-muted);
  white-space: nowrap;
}

.entry-recent-chip {
  max-width: 260px;
  padding: 5px 12px;
  font-size: var(--cpms-font-size-small);
  color: var(--cpms-color-text-secondary);
  background: var(--cpms-color-bg-hover);
  border: 1px solid var(--cpms-color-border);
  border-radius: var(--cpms-radius-full);
  cursor: pointer;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition:
    color var(--cpms-duration-fast) var(--cpms-easing-base),
    border-color var(--cpms-duration-fast) var(--cpms-easing-base),
    background-color var(--cpms-duration-fast) var(--cpms-easing-base);
}

.entry-recent-chip:hover:not(:disabled) {
  color: var(--cpms-color-primary-text);
  border-color: var(--cpms-color-primary-border);
  background: var(--cpms-color-primary-bg);
}

.entry-recent-chip:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.entry-recent-action {
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  padding: 0;
  color: var(--cpms-color-text-muted);
  background: transparent;
  border: 0;
  border-radius: var(--cpms-radius-small);
  cursor: pointer;
  font-size: 14px;
  transition:
    color var(--cpms-duration-fast) var(--cpms-easing-base),
    background-color var(--cpms-duration-fast) var(--cpms-easing-base);
}

.entry-recent-action:hover {
  color: var(--cpms-color-primary-text);
  background: var(--cpms-color-primary-bg);
}

.entry-tip {
  margin: var(--cpms-space-2) 0 0;
  font-size: var(--cpms-font-size-small);
  color: var(--cpms-color-text-muted);
  line-height: var(--cpms-line-height-small);
}

@media (prefers-reduced-motion: reduce) {
  .entry-input-wrap,
  .entry-input,
  .entry-submit,
  .entry-recent-chip,
  .entry-input-clear,
  .entry-recent-action {
    transition: none;
  }

  .entry-submit:hover:not(:disabled) {
    transform: none;
  }

  .entry-spinner {
    animation: none;
  }
}
</style>
