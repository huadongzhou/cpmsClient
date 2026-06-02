import { createHttpClient } from "@/api/http/client";
import { getLocalServiceBaseUrl } from "@/api/request/config";

/** 本地服务请求客户端，用于访问本机 agent、健康检查和本地任务接口。 */
export const localServiceApi = createHttpClient({
  baseUrl: getLocalServiceBaseUrl,
});
