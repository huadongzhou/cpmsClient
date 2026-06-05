import { unwrapCommand } from "./client";

/** 生成 CPMS access_sign。 */
export function signRequest(uri: string, params = "") {
  return unwrapCommand<string>("sign_request", { uri, params });
}

/** 使用客户端兼容实现加密密码。 */
export function sm4Encrypt(text: string) {
  return unwrapCommand<string>("sm4_encrypt", { text });
}
