import { invoke, type InvokeArgs } from "@tauri-apps/api/core";
import type { CommandResult } from "@/types/common/result";

export class CommandInvokeError extends Error {
  readonly code: string;
  readonly result?: CommandResult<unknown>;

  constructor(message: string, code = "COMMAND_ERROR", result?: CommandResult<unknown>) {
    super(message);
    this.name = "CommandInvokeError";
    this.code = code;
    this.result = result;
  }
}

/** 调用 Rust command，并保留后端返回的 CommandResult 原始结构。 */
export async function invokeCommand<T>(
  command: string,
  args?: InvokeArgs,
): Promise<CommandResult<T>> {
  try {
    return await invoke<CommandResult<T>>(command, args);
  } catch (error) {
    throw new CommandInvokeError(normalizeCommandError(error), "TAURI_INVOKE_ERROR");
  }
}

/** 调用 Rust command，并在 success=false 或 data 为空时抛出统一错误。 */
export async function unwrapCommand<T>(command: string, args?: InvokeArgs): Promise<T> {
  const result = await invokeCommand<T>(command, args);
  return unwrapCommandResult(result);
}

/** 拆包 Rust CommandResult，供 command API 和测试代码复用。 */
export function unwrapCommandResult<T>(result: CommandResult<T>): T {
  if (!result.success) {
    throw new CommandInvokeError(result.message, result.code, result as CommandResult<unknown>);
  }

  if (result.data === null) {
    throw new CommandInvokeError("Command returned empty data", "EMPTY_COMMAND_DATA", result);
  }

  return result.data;
}

function normalizeCommandError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === "string") {
    return error;
  }

  return "Tauri command invoke failed";
}
