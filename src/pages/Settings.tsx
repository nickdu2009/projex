import { Button, Group, Paper, SegmentedControl, Stack, Text, Title, TextInput, PasswordInput, Switch, Divider } from '@mantine/core';
import { IconDownload, IconUpload, IconCloud, IconCloudUpload, IconRestore } from '@tabler/icons-react';
import { useState, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { exportApi } from '../api/export';
import { showError, showSuccess } from '../utils/errorToast';
import { syncManager } from '../sync/SyncManager';
import { usePartnerStore } from '../stores/usePartnerStore';
import { usePersonStore } from '../stores/usePersonStore';
import { useTagStore } from '../stores/useTagStore';
import type { SyncConfigDto } from '../api/sync';

export function Settings() {
  const { t, i18n } = useTranslation();
  const [exporting, setExporting] = useState(false);
  const [importing, setImporting] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const invalidatePartners = usePartnerStore((s) => s.invalidate);
  const invalidatePersons = usePersonStore((s) => s.invalidate);
  const invalidateTags = useTagStore((s) => s.invalidate);
  const [syncConfig, setSyncConfig] = useState<SyncConfigDto | null>(null);
  const [syncEnabled, setSyncEnabled] = useState(false);
  const [bucket, setBucket] = useState('');
  const [endpoint, setEndpoint] = useState('');
  const [accessKey, setAccessKey] = useState('');
  const [secretKey, setSecretKey] = useState('');
  const [saving, setSaving] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [snapshotting, setSnapshotting] = useState(false);
  const [restoring, setRestoring] = useState(false);

  useEffect(() => {
    loadSyncConfig();
  }, []);

  const loadSyncConfig = async () => {
    try {
      const config = await syncManager.getConfig();
      setSyncConfig(config);
      setSyncEnabled(config.enabled);
      setBucket(config.bucket || '');
      setEndpoint(config.endpoint || '');
    } catch (error: unknown) {
      console.error('Load sync config failed:', error);
    }
  };

  const handleExport = async () => {
    setExporting(true);
    try {
      const jsonString = await exportApi.exportJson();

      const filePath = await save({
        title: t('settings.export.dialogTitle'),
        filters: [
          {
            name: 'JSON',
            extensions: ['json'],
          },
        ],
        defaultPath: `project-management-backup-${new Date().toISOString().split('T')[0]}.json`,
      });

      if (filePath) {
        await writeTextFile(filePath, jsonString);
        showSuccess(t('settings.export.exportSuccess'));
      }
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('settings.export.exportFailed'));
    } finally {
      setExporting(false);
    }
  };

  const handleImport = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;
    setImporting(true);
    try {
      const text = await file.text();
      const result = await exportApi.importJson(text);
      const parts = [];
      if (result.persons > 0) parts.push(t('settings.export.persons', { count: result.persons }));
      if (result.partners > 0) parts.push(t('settings.export.partners', { count: result.partners }));
      if (result.projects > 0) parts.push(t('settings.export.projects', { count: result.projects }));
      if (result.assignments > 0) parts.push(t('settings.export.assignments', { count: result.assignments }));
      if (result.status_history > 0) parts.push(t('settings.export.statusHistory', { count: result.status_history }));
      const msg = parts.length > 0
        ? t('settings.export.importSuccess', { details: parts.join(', ') }) + (result.skipped_duplicates > 0 ? ` ${t('settings.export.skippedDuplicates', { count: result.skipped_duplicates })}` : '')
        : t('settings.export.importAllExist', { count: result.skipped_duplicates });
      showSuccess(msg);
      // Invalidate all stores after import
      invalidatePartners();
      invalidatePersons();
      invalidateTags();
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('settings.export.importFailed'));
    } finally {
      setImporting(false);
      // Reset file input so same file can be re-selected
      if (fileInputRef.current) fileInputRef.current.value = '';
    }
  };

  const handleSaveSyncConfig = async () => {
    setSaving(true);
    try {
      await syncManager.updateConfig({
        enabled: syncEnabled,
        bucket,
        endpoint: endpoint || undefined,
        accessKey,
        secretKey,
      });
      showSuccess(t('settings.sync.configSaved'));
      await loadSyncConfig();
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('settings.sync.configSaveFailed'));
    } finally {
      setSaving(false);
    }
  };

  const handleSync = async () => {
    setSyncing(true);
    try {
      await syncManager.sync();
      showSuccess(t('settings.sync.syncComplete'));
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('settings.sync.syncFailed'));
    } finally {
      setSyncing(false);
    }
  };

  const handleCreateSnapshot = async () => {
    setSnapshotting(true);
    try {
      const result = await syncManager.createSnapshot();
      showSuccess(t('settings.sync.snapshotCreated', { id: result.substring(0, 12) }));
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('settings.sync.snapshotFailed'));
    } finally {
      setSnapshotting(false);
    }
  };

  const handleRestoreSnapshot = async () => {
    setRestoring(true);
    try {
      const result = await syncManager.restoreSnapshot();
      showSuccess(t('settings.sync.snapshotRestored', { id: result.substring(0, 12) }));
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('settings.sync.restoreFailed'));
    } finally {
      setRestoring(false);
    }
  };

  return (
    <Stack gap="md" w="100%" pb="xl" style={{ minWidth: 0 }}>
      <Title order={3}>{t('settings.title')}</Title>

      {/* S3 Sync Configuration */}
      <Paper>
        <Stack gap="md">
          <div>
            <Group justify="space-between" mb="xs">
              <Text size="sm" fw={500}>
                {t('settings.sync.title')}
              </Text>
              <Switch
                checked={syncEnabled}
                onChange={(e) => setSyncEnabled(e.currentTarget.checked)}
              />
            </Group>
            <Text size="xs" c="dimmed" mb="md">
              {t('settings.sync.description')}
            </Text>

            {syncConfig && (
              <Text size="xs" c="dimmed" mb="md">
                {t('settings.sync.deviceId', { id: syncConfig.device_id })}
              </Text>
            )}
          </div>

          <TextInput
            label={t('settings.sync.bucket')}
            placeholder="my-project-sync"
            value={bucket}
            onChange={(e) => setBucket(e.currentTarget.value)}
            required
          />

          <TextInput
            label={t('settings.sync.endpoint')}
            placeholder="https://xxxxxxxx.r2.cloudflarestorage.com"
            description={t('settings.sync.endpointDesc')}
            value={endpoint}
            onChange={(e) => setEndpoint(e.currentTarget.value)}
          />

          <TextInput
            label={t('settings.sync.accessKey')}
            placeholder={t('settings.sync.accessKeyPlaceholder')}
            value={accessKey}
            onChange={(e) => setAccessKey(e.currentTarget.value)}
            required
          />

          <PasswordInput
            label={t('settings.sync.secretKey')}
            placeholder={t('settings.sync.secretKeyPlaceholder')}
            value={secretKey}
            onChange={(e) => setSecretKey(e.currentTarget.value)}
            required
          />

          <Group>
            <Button
              variant="gradient"
              gradient={{ from: 'cyan', to: 'blue' }}
              onClick={handleSaveSyncConfig}
              loading={saving}
            >
              {t('settings.sync.saveConfig')}
            </Button>
          </Group>
        </Stack>
      </Paper>

      {/* Sync Operations */}
      {syncEnabled && (
        <Paper>
          <Stack gap="md">
            <Text size="sm" fw={500}>
              {t('settings.sync.operations')}
            </Text>

            <Group>
              <Button
                leftSection={<IconCloud size={18} />}
                variant="gradient"
                gradient={{ from: 'teal', to: 'cyan' }}
                onClick={handleSync}
                loading={syncing}
              >
                {t('settings.sync.syncNow')}
              </Button>

              <Button
                leftSection={<IconCloudUpload size={18} />}
                variant="light"
                color="blue"
                onClick={handleCreateSnapshot}
                loading={snapshotting}
              >
                {t('settings.sync.createSnapshot')}
              </Button>

              <Button
                leftSection={<IconRestore size={18} />}
                variant="light"
                color="orange"
                onClick={handleRestoreSnapshot}
                loading={restoring}
              >
                {t('settings.sync.restoreSnapshot')}
              </Button>
            </Group>

            <Text size="xs" c="dimmed">
              {t('settings.sync.operationsDesc')}
            </Text>
          </Stack>
        </Paper>
      )}

      <Divider />

      {/* Data Export / Import */}
      <Paper>
        <Stack gap="md">
          <div>
            <Text size="sm" fw={500} mb="xs">
              {t('settings.export.title')}
            </Text>
            <Text size="xs" c="dimmed" mb="md">
              {t('settings.export.description')}
            </Text>
            <Group>
              <Button
                leftSection={<IconDownload size={18} />}
                variant="gradient"
                gradient={{ from: 'indigo', to: 'violet' }}
                onClick={handleExport}
                loading={exporting}
              >
                {t('settings.export.exportButton')}
              </Button>
              <Button
                leftSection={<IconUpload size={18} />}
                variant="light"
                color="teal"
                onClick={() => fileInputRef.current?.click()}
                loading={importing}
              >
                {t('settings.export.importButton')}
              </Button>
              <input
                ref={fileInputRef}
                type="file"
                accept=".json"
                style={{ display: 'none' }}
                onChange={handleImport}
              />
            </Group>
          </div>
        </Stack>
      </Paper>

      <Divider />

      {/* Language */}
      <Paper>
        <Stack gap="xs">
          <Text size="sm" fw={500}>
            {t('settings.language.title')}
          </Text>
          <Text size="xs" c="dimmed" mb="xs">
            {t('settings.language.description')}
          </Text>
          <SegmentedControl
            value={i18n.language}
            onChange={(lng) => i18n.changeLanguage(lng)}
            data={[
              { value: 'en', label: t('settings.language.en') },
              { value: 'zh', label: t('settings.language.zh') },
            ]}
            style={{ alignSelf: 'flex-start' }}
          />
        </Stack>
      </Paper>

      <Divider />

      {/* About */}
      <Paper>
        <Stack gap="xs">
          <Text size="sm" fw={500}>
            {t('settings.about.title')}
          </Text>
          <Text size="xs" c="dimmed">
            Projex v1.0.0
          </Text>
          <Text size="xs" c="dimmed">
            Schema Version: 1
          </Text>
        </Stack>
      </Paper>
    </Stack>
  );
}
