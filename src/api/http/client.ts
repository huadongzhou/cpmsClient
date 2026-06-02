import type { ApiResult } from "@/types/common/result";

export interface HttpClientOptions {
  baseUrl: string | (() => string);
  timeoutMs?: number;
  getToken?: () => string | undefined;
}

export interface HttpRequestOptions extends Omit<RequestInit, "body"> {
  body?: unknown;
}

export interface HttpClient {
  request<T>(path: string, requestOptions?: HttpRequestOptions): Promise<T>;
  get<T>(path: string, requestOptions?: HttpRequestOptions): Promise<T>;
  post<T>(path: string, body?: unknown, requestOptions?: HttpRequestOptions): Promise<T>;
  put<T>(path: string, body?: unknown, requestOptions?: HttpRequestOptions): Promise<T>;
  patch<T>(path: string, body?: unknown, requestOptions?: HttpRequestOptions): Promise<T>;
  delete<T>(path: string, requestOptions?: HttpRequestOptions): Promise<T>;
}

export class HttpError extends Error {
  readonly status?: number;
  readonly code: string;

  constructor(message: string, code = "HTTP_ERROR", status?: number) {
    super(message);
    this.name = "HttpError";
    this.code = code;
    this.status = status;
  }
}

/** 创建统一 HTTP 客户端，负责超时、token 注入、JSON 序列化和统一响应拆包。 */
export function createHttpClient(options: HttpClientOptions): HttpClient {
  const timeoutMs = options.timeoutMs ?? 15_000;

  /** 发起底层 HTTP 请求，并把 CPMS 标准 ApiResult 结构拆成业务数据。 */
  async function request<T>(path: string, requestOptions: HttpRequestOptions = {}): Promise<T> {
    const controller = new AbortController();
    const timeout = window.setTimeout(() => controller.abort(), timeoutMs);
    const token = options.getToken?.();
    const requestBody = serializeBody(requestOptions.body);
    const headers = buildHeaders(requestOptions.headers, token, requestBody.useJsonContentType);

    try {
      const response = await fetch(resolveUrl(resolveBaseUrl(options.baseUrl), path), {
        ...requestOptions,
        headers,
        body: requestBody.body,
        signal: controller.signal,
      });

      const payload = (await readJson(response)) as ApiResult<T> | T;

      if (!response.ok) {
        throw new HttpError(`HTTP ${response.status}`, "HTTP_STATUS_ERROR", response.status);
      }

      if (isApiResult<T>(payload)) {
        if (!payload.success) {
          throw new HttpError(payload.message, payload.code, response.status);
        }

        return payload.data as T;
      }

      return payload as T;
    } catch (error) {
      if (error instanceof HttpError) {
        throw error;
      }

      if (error instanceof DOMException && error.name === "AbortError") {
        throw new HttpError("请求超时", "HTTP_TIMEOUT");
      }

      throw new HttpError(normalizeHttpError(error), "HTTP_NETWORK_ERROR");
    } finally {
      window.clearTimeout(timeout);
    }
  }

  return {
    request,
    get: <T>(path: string, requestOptions?: HttpRequestOptions) =>
      request<T>(path, { ...requestOptions, method: "GET" }),
    post: <T>(path: string, body?: unknown, requestOptions?: HttpRequestOptions) =>
      request<T>(path, { ...requestOptions, method: "POST", body }),
    put: <T>(path: string, body?: unknown, requestOptions?: HttpRequestOptions) =>
      request<T>(path, { ...requestOptions, method: "PUT", body }),
    patch: <T>(path: string, body?: unknown, requestOptions?: HttpRequestOptions) =>
      request<T>(path, { ...requestOptions, method: "PATCH", body }),
    delete: <T>(path: string, requestOptions?: HttpRequestOptions) =>
      request<T>(path, { ...requestOptions, method: "DELETE" }),
  };
}

function resolveBaseUrl(baseUrl: string | (() => string)) {
  return typeof baseUrl === "function" ? baseUrl() : baseUrl;
}

function resolveUrl(baseUrl: string, path: string) {
  return `${baseUrl.replace(/\/$/, "")}/${path.replace(/^\//, "")}`;
}

async function readJson(response: Response) {
  const text = await response.text();

  if (!text) {
    return null;
  }

  try {
    return JSON.parse(text);
  } catch {
    throw new HttpError("响应不是合法 JSON", "HTTP_RESPONSE_PARSE_ERROR", response.status);
  }
}

function serializeBody(body: unknown): { body?: BodyInit; useJsonContentType: boolean } {
  if (body === undefined || body === null) {
    return { body: undefined, useJsonContentType: false };
  }

  if (
    typeof body === "string" ||
    body instanceof Blob ||
    body instanceof FormData ||
    body instanceof URLSearchParams ||
    body instanceof ArrayBuffer
  ) {
    return { body, useJsonContentType: false };
  }

  return {
    body: JSON.stringify(body),
    useJsonContentType: true,
  };
}

function buildHeaders(
  inputHeaders: HeadersInit | undefined,
  token: string | undefined,
  useJsonContentType: boolean,
) {
  const headers = new Headers(inputHeaders);

  if (useJsonContentType && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }

  if (token && !headers.has("Authorization")) {
    headers.set("Authorization", `Bearer ${token}`);
  }

  return headers;
}

function isApiResult<T>(payload: unknown): payload is ApiResult<T> {
  return (
    typeof payload === "object" &&
    payload !== null &&
    "success" in payload &&
    "code" in payload &&
    "message" in payload
  );
}

function normalizeHttpError(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === "string") {
    return error;
  }

  return "网络请求失败";
}
