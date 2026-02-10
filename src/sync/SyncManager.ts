/**
 * S3 同步管理器
 * 负责协调前端与后端的同步操作
 */

import { syncApi, type SyncConfigDto, type SyncStatusDto } from '../api/sync';

export interface SyncState {
  status: 'idle' | 'syncing' | 'error';
  lastSync?: Date;
  error?: string;
  pendingChanges: number;
}

type SyncStateListener = (state: SyncState) => void;

export class SyncManager {
  private state: SyncState = {
    status: 'idle',
    pendingChanges: 0,
  };

  private listeners: Set<SyncStateListener> = new Set();
  private autoSyncTimer?: number;
  private autoSyncInterval = 60000; // 默认 1 分钟

  constructor() {
    // Initialization
  }

  /**
   * 初始化同步管理器
   */
  async initialize(): Promise<void> {
    try {
      const status = await syncApi.getStatus();
      this.updateState({
        status: 'idle',
        lastSync: status.last_sync ? new Date(status.last_sync) : undefined,
        error: status.last_error,
        pendingChanges: status.pending_changes,
      });

      console.log('✅ SyncManager initialized', this.state);
    } catch (error: any) {
      console.error('❌ SyncManager init failed:', error);
      this.updateState({ status: 'error', error: error.message });
    }
  }

  /**
   * 获取同步配置
   */
  async getConfig(): Promise<SyncConfigDto> {
    return await syncApi.getConfig();
  }

  /**
   * 更新同步配置
   */
  async updateConfig(config: {
    enabled: boolean;
    bucket: string;
    endpoint?: string;
    accessKey: string;
    secretKey: string;
  }): Promise<void> {
    await syncApi.updateConfig({
      enabled: config.enabled,
      bucket: config.bucket,
      endpoint: config.endpoint,
      access_key: config.accessKey,
      secret_key: config.secretKey,
    });

    // 重新初始化
    await this.initialize();
  }

  /**
   * 执行完整同步
   */
  async sync(): Promise<void> {
    if (this.state.status === 'syncing') {
      console.log('Sync already in progress');
      return;
    }

    this.updateState({ status: 'syncing', error: undefined });

    try {
      const result = await syncApi.syncFull();
      console.log('✅ Sync completed:', result);

      // 更新状态
      const status = await syncApi.getStatus();
      this.updateState({
        status: 'idle',
        lastSync: new Date(),
        error: undefined,
        pendingChanges: status.pending_changes,
      });
    } catch (error: any) {
      console.error('❌ Sync failed:', error);
      this.updateState({
        status: 'error',
        error: error.message || 'Unknown error',
      });
      throw error;
    }
  }

  /**
   * 创建快照
   */
  async createSnapshot(): Promise<string> {
    return await syncApi.createSnapshot();
  }

  /**
   * 恢复快照
   */
  async restoreSnapshot(): Promise<string> {
    return await syncApi.restoreSnapshot();
  }

  /**
   * 启动自动同步
   */
  startAutoSync(intervalMs?: number): void {
    if (intervalMs) {
      this.autoSyncInterval = intervalMs;
    }

    this.stopAutoSync(); // 停止之前的定时器

    this.autoSyncTimer = window.setInterval(() => {
      this.sync().catch((err) => {
        console.error('Auto sync failed:', err);
      });
    }, this.autoSyncInterval);

    console.log(`🔄 Auto sync started (interval: ${this.autoSyncInterval}ms)`);
  }

  /**
   * 停止自动同步
   */
  stopAutoSync(): void {
    if (this.autoSyncTimer) {
      clearInterval(this.autoSyncTimer);
      this.autoSyncTimer = undefined;
      console.log('⏸️ Auto sync stopped');
    }
  }

  /**
   * 订阅状态变化
   */
  subscribe(listener: SyncStateListener): () => void {
    this.listeners.add(listener);
    // 立即调用一次以获取当前状态
    listener(this.state);

    // 返回取消订阅函数
    return () => {
      this.listeners.delete(listener);
    };
  }

  /**
   * 获取当前状态
   */
  getState(): SyncState {
    return { ...this.state };
  }

  /**
   * 更新状态并通知所有监听器
   */
  private updateState(partial: Partial<SyncState>): void {
    this.state = { ...this.state, ...partial };
    this.listeners.forEach((listener) => listener(this.state));
  }
}

// 全局单例
export const syncManager = new SyncManager();
