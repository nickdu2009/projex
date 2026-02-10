/**
 * 同步状态栏组件
 * 显示在页面底部，实时显示同步状态
 */

import { Group, Text, Loader, ThemeIcon, ActionIcon, Tooltip } from '@mantine/core';
import { IconCloud, IconCloudCheck, IconCloudX, IconRefresh } from '@tabler/icons-react';
import { useEffect, useState } from 'react';
import { syncManager, type SyncState } from '../sync/SyncManager';

export function SyncStatusBar() {
  const [state, setState] = useState<SyncState>(syncManager.getState());

  useEffect(() => {
    const unsubscribe = syncManager.subscribe(setState);
    return unsubscribe;
  }, []);

  const handleManualSync = async () => {
    try {
      await syncManager.sync();
    } catch (error) {
      console.error('Manual sync failed:', error);
    }
  };

  const getStatusIcon = () => {
    if (state.status === 'syncing') {
      return <Loader size="xs" />;
    }
    if (state.status === 'error') {
      return (
        <ThemeIcon color="red" variant="light" size="sm">
          <IconCloudX size={14} />
        </ThemeIcon>
      );
    }
    return (
      <ThemeIcon color="teal" variant="light" size="sm">
        <IconCloudCheck size={14} />
      </ThemeIcon>
    );
  };

  const getStatusText = () => {
    if (state.status === 'syncing') {
      return '同步中...';
    }
    if (state.status === 'error') {
      return `同步失败: ${state.error}`;
    }
    if (state.lastSync) {
      const diff = Date.now() - state.lastSync.getTime();
      const minutes = Math.floor(diff / 60000);
      if (minutes === 0) return '刚刚同步';
      if (minutes < 60) return `${minutes} 分钟前同步`;
      const hours = Math.floor(minutes / 60);
      return `${hours} 小时前同步`;
    }
    return '未同步';
  };

  const getStatusColor = () => {
    if (state.status === 'syncing') return 'blue';
    if (state.status === 'error') return 'red';
    return 'dimmed';
  };

  return (
    <Group
      justify="space-between"
      px="md"
      py="xs"
      style={{
        borderTop: '1px solid var(--mantine-color-default-border)',
        background: 'rgba(255, 255, 255, 0.6)',
        backdropFilter: 'blur(10px)',
      }}
    >
      <Group gap="xs">
        {getStatusIcon()}
        <Text size="sm" c={getStatusColor()}>
          {getStatusText()}
        </Text>
        {state.pendingChanges > 0 && (
          <Text size="xs" c="orange">
            ({state.pendingChanges} 个待同步更改)
          </Text>
        )}
      </Group>

      <Group gap="xs">
        <Tooltip label="手动同步">
          <ActionIcon
            variant="subtle"
            size="sm"
            onClick={handleManualSync}
            loading={state.status === 'syncing'}
          >
            <IconRefresh size={16} />
          </ActionIcon>
        </Tooltip>
      </Group>
    </Group>
  );
}
