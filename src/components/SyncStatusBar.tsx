/**
 * Sync status bar component
 * Displayed at the bottom of the page, showing real-time sync status
 */

import { Group, Text, Loader, ThemeIcon, ActionIcon, Tooltip } from '@mantine/core';
import { IconCloudCheck, IconCloudX, IconRefresh } from '@tabler/icons-react';
import { useEffect, useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { syncManager, type SyncState } from '../sync/SyncManager';
import { logger } from '../utils/logger';

export function SyncStatusBar() {
  const { t } = useTranslation();
  const [state, setState] = useState<SyncState>(syncManager.getState());
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const unsubscribe = syncManager.subscribe(setState);
    return unsubscribe;
  }, []);

  // Tick every 30s so relative time display stays reasonably fresh
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(timer);
  }, []);

  const handleManualSync = async () => {
    try {
      await syncManager.sync();
    } catch (error) {
      logger.error('Manual sync failed:', error);
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

  const getStatusText = useCallback(() => {
    if (state.status === 'syncing') {
      return t('sync.syncing');
    }
    if (state.status === 'error') {
      return t('sync.failed', { error: state.error });
    }
    if (state.lastSync) {
      const diff = now - state.lastSync.getTime();
      const minutes = Math.floor(diff / 60000);
      if (minutes === 0) return t('sync.justSynced');
      if (minutes < 60) return t('sync.minutesAgo', { minutes });
      const hours = Math.floor(minutes / 60);
      return t('sync.hoursAgo', { hours });
    }
    return t('sync.notSynced');
  }, [state, now, t]);

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
            {t('sync.pendingChanges', { count: state.pendingChanges })}
          </Text>
        )}
      </Group>

      <Group gap="xs">
        <Tooltip label={t('sync.manualSync')}>
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
