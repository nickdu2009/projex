import { invoke } from '@tauri-apps/api/core';

export interface LogFileDto {
  name: string;
  size_bytes: number;
  modified_at?: string;
}

export interface LogTailReq {
  file_name: string;
  max_bytes?: number;
  redact?: boolean;
  cursor?: number;
}

export interface LogTailResp {
  content: string;
  next_cursor?: number;
  truncated: boolean;
}

export interface LogClearReq {
  file_name: string;
}

export interface LogLevelResp {
  current_level: string;
  requires_restart: boolean;
}

export const logsApi = {
  async listFiles(): Promise<LogFileDto[]> {
    return await invoke<LogFileDto[]>('cmd_log_list_files');
  },

  async tail(req: LogTailReq): Promise<LogTailResp> {
    return await invoke<LogTailResp>('cmd_log_tail', { req });
  },

  async clear(req: LogClearReq): Promise<string> {
    return await invoke<string>('cmd_log_clear', { req });
  },

  async getLevel(): Promise<LogLevelResp> {
    return await invoke<LogLevelResp>('cmd_log_get_level');
  },

  async setLevel(level: string): Promise<string> {
    return await invoke<string>('cmd_log_set_level', { level });
  },
};
