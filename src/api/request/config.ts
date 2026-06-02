import type { ClientConfig } from "@/types/app/config";

const DEFAULT_CPMS_BASE_URL = "http://localhost:8080";
const DEFAULT_LOCAL_SERVICE_URL = "http://127.0.0.1:18080";

/** 读取客户端运行时配置，优先使用设置页写入的 localStorage 配置。 */
export function readClientConfig(): ClientConfig {
  const rawConfig = window.localStorage.getItem("cpmsClient:config");

  if (!rawConfig) {
    return getDefaultClientConfig();
  }

  try {
    return {
      ...getDefaultClientConfig(),
      ...JSON.parse(rawConfig),
    };
  } catch {
    return getDefaultClientConfig();
  }
}

/** 获取 CPMS 服务地址，用于线上业务接口请求。 */
export function getCpmsBaseUrl() {
  return import.meta.env.VITE_CPMS_BASE_URL || readClientConfig().cpmsBaseUrl;
}

/** 获取本地服务地址，用于本机 agent / health / task 等接口请求。 */
export function getLocalServiceBaseUrl() {
  return readClientConfig().localServiceUrl;
}

/** 读取当前登录 token，统一为 HTTP 请求层提供 Authorization 来源。 */
export function getAccessToken() {
  return window.localStorage.getItem("cpmsClient:token") || undefined;
}

function getDefaultClientConfig(): ClientConfig {
  return {
    cpmsBaseUrl: DEFAULT_CPMS_BASE_URL,
    localServiceUrl: DEFAULT_LOCAL_SERVICE_URL,
    logLevel: "info",
  };
}
