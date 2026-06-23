<script setup lang="ts" name="EntryView">
import { ElMessage } from "element-plus";
import {
  CircleCloseFilled,
  EditPen,
  Link,
  Platform,
  WarningFilled,
} from "@element-plus/icons-vue";
import { setClientIframeUrl } from "@/api/tauri/desktop";

const LAST_IFRAME_URL_KEY = "cpms-last-iframe-url";

const url = ref(localStorage.getItem(LAST_IFRAME_URL_KEY) ?? "");
const loading = ref(false);
const error = ref("");

const recentUrl = computed(() => localStorage.getItem(LAST_IFRAME_URL_KEY) ?? "");
const hasRecentUrl = computed(() => recentUrl.value.length > 0);
const isSubmittable = computed(() => !loading.value && url.value.trim().length > 0);

function validate(input: string): string | null {
  const trimmed = input.trim();
  if (!trimmed) return "请输入入口地址";
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
          <el-icon class="entry-icon-svg"><Platform /></el-icon>
        </div>
        <h1 class="entry-title">CPMS Client</h1>
      </div>

      <div class="entry-form">
        <div class="entry-input-wrap" :class="{ 'has-error': error }">
          <span class="entry-input-prefix" aria-hidden="true">
            <el-icon><Link /></el-icon>
          </span>
          <input
            v-model="url"
            class="entry-input"
            type="text"
            placeholder="请输入地址"
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
            <el-icon><CircleCloseFilled /></el-icon>
          </button>
        </div>

        <p v-if="error" class="entry-error" role="alert">
          <el-icon><WarningFilled /></el-icon>
          <span>{{ error }}</span>
        </p>

        <button type="button" class="entry-submit" :disabled="!isSubmittable" @click="loadUrl()">
          <span v-if="loading" class="entry-spinner" aria-hidden="true" />
          <span>{{ loading ? "正在加载…" : "加载页面" }}</span>
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
            <el-icon><EditPen /></el-icon>
          </button>
        </div>

        <p class="entry-tip">仅支持 http/https 协议。</p>
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
  background: transparent;
  overflow: hidden;
}

.entry-decoration {
  position: absolute;
  inset: 0;
  pointer-events: none;
  background:
    radial-gradient(circle at 12% 18%, var(--cpms-color-primary-bg) 0%, transparent 38%),
    radial-gradient(circle at 88% 82%, var(--cpms-color-success-bg) 0%, transparent 38%);
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
  width: 64px;
  height: 64px;
  color: var(--cpms-color-text-on-primary);
  background: var(--cpms-color-primary);
  border-radius: var(--cpms-radius-large);
  box-shadow: var(--cpms-shadow-sm);
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
  box-shadow: 0 0 0 3px var(--cpms-color-primary-bg);
}

.entry-input-wrap.has-error:focus-within {
  box-shadow: 0 0 0 3px var(--cpms-color-danger-bg);
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
  padding: 0 44px 0 42px;
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
  right: 8px;
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
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

.entry-input-clear:focus-visible {
  outline: 2px solid var(--cpms-color-primary);
  outline-offset: 2px;
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
  min-height: 48px;
  padding: 0 var(--cpms-space-4);
  font-size: var(--cpms-font-size-base);
  font-weight: var(--cpms-font-weight-medium);
  color: var(--cpms-color-text-on-primary);
  background: var(--cpms-color-primary);
  border: 0;
  border-radius: var(--cpms-radius-panel);
  cursor: pointer;
  transition:
    background-color var(--cpms-duration-base) var(--cpms-easing-base),
    box-shadow var(--cpms-duration-base) var(--cpms-easing-base),
    opacity var(--cpms-duration-base) var(--cpms-easing-base);
}

.entry-submit:hover:not(:disabled) {
  background: var(--cpms-color-primary-hover);
  box-shadow: var(--cpms-shadow-md);
}

.entry-submit:active:not(:disabled) {
  background: var(--cpms-color-primary-active);
}

.entry-submit:focus-visible {
  outline: 2px solid var(--cpms-color-primary);
  outline-offset: 2px;
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
  width: 44px;
  height: 44px;
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

.entry-recent-action:focus-visible {
  outline: 2px solid var(--cpms-color-primary);
  outline-offset: 2px;
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
