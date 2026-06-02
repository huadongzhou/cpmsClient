import { storeToRefs } from "pinia";
import { useAppStore } from "@/stores/app";
import type { AppError, AppErrorLevel, AppErrorSource } from "@/types/app/error";

/** 提供应用错误队列的读写方法，并把错误同步为应用通知。 */
export function useAppError() {
  const appStore = useAppStore();
  const { latestError, errors } = storeToRefs(appStore);

  /** 上报一次应用错误，统一提取 code/message/source/level。 */
  function reportError(
    error: unknown,
    source: AppErrorSource,
    level: AppErrorLevel = "error",
  ): AppError {
    return appStore.pushError({
      source,
      level,
      code: resolveErrorCode(error),
      message: resolveErrorMessage(error),
    });
  }

  return {
    errors,
    latestError,
    reportError,
    clearError: appStore.clearError,
    clearErrors: appStore.clearErrors,
  };
}

function resolveErrorCode(error: unknown) {
  if (typeof error === "object" && error !== null && "code" in error) {
    return String(error.code);
  }

  return "APP_ERROR";
}

function resolveErrorMessage(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === "string") {
    return error;
  }

  return "操作失败";
}
