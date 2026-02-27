import { invoke } from '@tauri-apps/api/core';

export interface SyncConfigDto {
  enabled: boolean;
  bucket?: string;
  endpoint?: string;
  access_key?: string;
  has_secret_key?: boolean;
  secret_key_masked?: string;
  device_id: string;
  last_sync?: string;
  auto_sync_interval_minutes: number;
}

export interface SyncConfigUpdateDto {
  enabled: boolean;
  bucket: string;
  endpoint?: string;
  access_key?: string;
  secret_key?: string;
  auto_sync_interval_minutes: number;
}

export interface SyncStatusDto {
  is_syncing: boolean;
  pending_changes: number;
  last_sync?: string;
  last_error?: string;
}

export interface SyncTestConnectionReq {
  bucket?: string;
  endpoint?: string;
  access_key?: string;
  secret_key?: string;
}

export const syncApi = {
  async getConfig(): Promise<SyncConfigDto> {
    return await invoke<SyncConfigDto>('cmd_sync_get_config');
  },

  async updateConfig(config: SyncConfigUpdateDto): Promise<string> {
    return await invoke<string>('cmd_sync_update_config', { req: config });
  }, 

  async setEnabled(enabled: boolean): Promise<string> {
    return await invoke<string>('cmd_sync_set_enabled', { req: { enabled } });
  },

  async revealSecretKey(): Promise<string> {
    return await invoke<string>('cmd_sync_reveal_secret_key');
  },

  async testConnection(req?: SyncTestConnectionReq): Promise<string> {
    if (req) {
      return await invoke<string>('cmd_sync_test_connection', { req });
    }
    return await invoke<string>('cmd_sync_test_connection');
  },

  async getStatus(): Promise<SyncStatusDto> {
    return await invoke<SyncStatusDto>('cmd_sync_get_status');
  },

  async syncFull(): Promise<string> {
    return await invoke<string>('cmd_sync_full');
  },

  async createSnapshot(): Promise<string> {
    return await invoke<string>('cmd_sync_create_snapshot');
  },

  async restoreSnapshot(): Promise<string> {
    return await invoke<string>('cmd_sync_restore_snapshot');
  },

  async exportConfig(): Promise<string> {
    return await invoke<string>('cmd_sync_export_config');
  },

  async importConfig(json: string): Promise<SyncConfigDto> {
    return await invoke<SyncConfigDto>('cmd_sync_import_config', { req: { json } });
  },
};
