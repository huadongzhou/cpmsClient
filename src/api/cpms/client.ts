import { createHttpClient } from "@/api/http/client";
import { getAccessToken, getCpmsBaseUrl } from "@/api/request/config";

/** CPMS 线上业务接口请求客户端。 */
export const cpmsApi = createHttpClient({
  baseUrl: getCpmsBaseUrl,
  getToken: getAccessToken,
});
