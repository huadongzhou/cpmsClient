<script setup lang="ts" name="EntryView">
import { ElMessage } from "element-plus";
import { setClientIframeUrl } from "@/api/tauri/desktop";

const LAST_IFRAME_URL_KEY = "cpms-last-iframe-url";

const url = ref(localStorage.getItem(LAST_IFRAME_URL_KEY) ?? "");
const loading = ref(false);
const error = ref("");

const recentUrl = computed(() => localStorage.getItem(LAST_IFRAME_URL_KEY) ?? "");
const hasRecentUrl = computed(() => recentUrl.value.length > 0);

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
</script>

<template>
  <main class="entry-view">
    <div class="entry-decoration" aria-hidden="true" />

    <section class="entry-card">
      <div class="entry-brand">
        <div class="entry-icon">
          <svg viewBox="0 0 24 24" width="28" height="28" aria-hidden="true">
            <path
              d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
            <path
              d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </div>
        <h1 class="entry-title">欢迎来到 CPMS Client</h1>
        <p class="entry-desc">请输入 hub-platform 业务入口地址，客户端将加载该页面。</p>
      </div>

      <div class="entry-form">
        <div class="entry-input-wrap">
          <span class="entry-input-prefix" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="16" height="16">
              <path
                d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20zm0 18a8 8 0 1 1 0-16 8 8 0 0 1 0 16z"
                fill="currentColor"
              />
              <path
                d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              />
            </svg>
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
            <svg viewBox="0 0 24 24" width="14" height="14">
              <path
                d="M18 6 6 18M6 6l12 12"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </button>
        </div>

        <p v-if="error" class="entry-error" role="alert">{{ error }}</p>

        <button
          type="button"
          class="entry-submit"
          :disabled="loading || !url.trim()"
          @click="loadUrl()"
        >
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
  width: 100%;
  height: 100%;
  padding: var(--cpms-space-xlarge);
  background: var(--cpms-color-bg-app);
  overflow: hidden;
}

.entry-decoration {
  position: absolute;
  inset: 0;
  pointer-events: none;
  background:
    radial-gradient(circle at 10% 20%, rgba(59, 130, 246, 0.08) 0%, transparent 35%),
    radial-gradient(circle at 90% 80%, rgba(16, 185, 129, 0.06) 0%, transparent 35%);
}

.entry-card {
  position: relative;
  z-index: 1;
  width: min(480px, 100%);
  padding: 40px;
  background: var(--cpms-color-bg-panel);
  border: 1px solid var(--cpms-color-border);
  border-radius: var(--cpms-radius-large);
  box-shadow: var(--cpms-shadow-lg);
  text-align: center;
}

.entry-brand {
  display: grid;
  gap: var(--cpms-space-base);
  justify-items: center;
  margin-bottom: 28px;
}

.entry-icon {
  display: grid;
  place-items: center;
  width: 56px;
  height: 56px;
  color: #fff;
  background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%);
  border-radius: var(--cpms-radius-large);
  box-shadow: 0 4px 14px rgba(37, 99, 235, 0.35);
}

.entry-title {
  margin: 0;
  font-size: 22px;
  font-weight: 600;
  line-height: 1.3;
  color: var(--cpms-color-text-primary);
}

.entry-desc {
  margin: 0;
  font-size: var(--cpms-font-size-base);
  color: var(--cpms-color-text-muted);
  line-height: var(--cpms-line-height-base);
}

.entry-form {
  display: grid;
  gap: var(--cpms-space-base);
}

.entry-input-wrap {
  position: relative;
  display: flex;
  align-items: center;
  width: 100%;
}

.entry-input-prefix {
  position: absolute;
  left: 14px;
  display: grid;
  place-items: center;
  color: var(--cpms-color-text-muted);
  pointer-events: none;
}

.entry-input {
  width: 100%;
  height: 46px;
  padding: 0 38px 0 40px;
  font-size: var(--cpms-font-size-base);
  color: var(--cpms-color-text-primary);
  background: var(--cpms-color-bg-panel);
  border: 1px solid var(--cpms-color-border);
  border-radius: var(--cpms-radius-medium);
  outline: none;
  transition: border-color 0.2s ease, box-shadow 0.2s ease;
}

.entry-input::placeholder {
  color: #9ca3af;
}

.entry-input:hover {
  border-color: #c4c9d0;
}

.entry-input:focus {
  border-color: #3b82f6;
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.12);
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
  width: 22px;
  height: 22px;
  padding: 0;
  color: var(--cpms-color-text-muted);
  background: transparent;
  border: 0;
  border-radius: 50%;
  cursor: pointer;
  transition: color 0.2s ease, background 0.2s ease;
}

.entry-input-clear:hover {
  color: var(--cpms-color-text-primary);
  background: var(--cpms-color-bg-hover);
}

.entry-error {
  margin: 0;
  text-align: left;
  font-size: var(--cpms-font-size-small);
  color: var(--cpms-color-danger);
  line-height: var(--cpms-line-height-small);
}

.entry-submit {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--cpms-space-small);
  width: 100%;
  height: 46px;
  padding: 0 var(--cpms-space-large);
  font-size: var(--cpms-font-size-base);
  font-weight: 500;
  color: #fff;
  background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%);
  border: 0;
  border-radius: var(--cpms-radius-medium);
  cursor: pointer;
  box-shadow: 0 4px 14px rgba(37, 99, 235, 0.3);
  transition: transform 0.15s ease, box-shadow 0.2s ease, opacity 0.2s ease;
}

.entry-submit:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 6px 18px rgba(37, 99, 235, 0.35);
}

.entry-submit:active:not(:disabled) {
  transform: translateY(0);
}

.entry-submit:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.entry-spinner {
  width: 16px;
  height: 16px;
  border: 2px solid rgba(255, 255, 255, 0.35);
  border-top-color: #fff;
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
  flex-wrap: wrap;
  align-items: center;
  justify-content: center;
  gap: var(--cpms-space-small);
  margin-top: 4px;
}

.entry-recent-label {
  font-size: var(--cpms-font-size-small);
  color: var(--cpms-color-text-muted);
}

.entry-recent-chip {
  max-width: 260px;
  padding: 4px 10px;
  font-size: var(--cpms-font-size-small);
  color: var(--cpms-color-text-secondary);
  background: var(--cpms-color-bg-hover);
  border: 1px solid var(--cpms-color-border);
  border-radius: 9999px;
  cursor: pointer;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition: color 0.2s ease, border-color 0.2s ease, background 0.2s ease;
}

.entry-recent-chip:hover:not(:disabled) {
  color: #2563eb;
  border-color: #bfdbfe;
  background: #eff6ff;
}

.entry-recent-chip:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.entry-tip {
  margin: 8px 0 0;
  font-size: var(--cpms-font-size-small);
  color: var(--cpms-color-text-muted);
  line-height: var(--cpms-line-height-small);
}

@media (prefers-reduced-motion: reduce) {
  .entry-input,
  .entry-submit,
  .entry-recent-chip {
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
