import { invoke } from '@tauri-apps/api/core';

export interface AppError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
}

export async function invokeCmd<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return (await invoke(cmd, args)) as T;
  } catch (e) {
    throw e as AppError;
  }
}
