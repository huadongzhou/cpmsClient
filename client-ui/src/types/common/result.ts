export interface CommandResult<T = unknown> {
  success: boolean;
  code: string;
  message: string;
  data: T | null;
  logs: string[];
}
