import {
  ActionIcon,
  Badge,
  Button,
  Divider,
  Group,
  NumberInput,
  Paper,
  SegmentedControl,
  Stack,
  Switch,
  Text,
  TextInput,
  Title,
} from '@mantine/core';
import { IconDownload, IconUpload, IconCloud, IconCloudUpload, IconRestore, IconEye, IconEyeOff, IconEdit, IconFileText } from '@tabler/icons-react';
import { useState, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { getVersion } from '@tauri-apps/api/app';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { exportApi } from '../api/export';
import { showError, showSuccess } from '../utils/errorToast';
import { logger } from '../utils/logger';
import { syncManager } from '../sync/SyncManager';
import { usePartnerStore } from '../stores/usePartnerStore';
import { usePersonStore } from '../stores/usePersonStore';
import { useTagStore } from '../stores/useTagStore';
import type { SyncConfigDto } from '../api/sync';
import { ConfirmModal } from '../components/ConfirmModal';

type AppErrorLike = { code?: string; message?: string };

export function Settings() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const [exporting, setExporting] = useState(false);
  const [importing, setImporting] = useState(false);
  const [appVersion, setAppVersion] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const invalidatePartners = usePartnerStore((s) => s.invalidate);
  const invalidatePersons = usePersonStore((s) => s.invalidate);
  const invalidateTags = useTagStore((s) => s.invalidate);
  const [syncConfig, setSyncConfig] = useState<SyncConfigDto | null>(null);
  const [syncEnabled, setSyncEnabled] = useState(false);
  const [bucket, setBucket] = useState('');
  const [endpoint, setEndpoint] = useState('');
  const [accessKey, setAccessKey] = useState('');
  const [syncConfigEditing, setSyncConfigEditing] = useState(false);
  const [autoSyncIntervalMinutes, setAutoSyncIntervalMinutes] = useState<number>(1);
  const [secretKey, setSecretKey] = useState('');
  const [secretKeySaved, setSecretKeySaved] = useState(false);
  const [secretKeyMasked, setSecretKeyMasked] = useState<string | null>(null);
  const [secretKeyRevealed, setSecretKeyRevealed] = useState(false);
  const [secretKeyEditBaseline, setSecretKeyEditBaseline] = useState<string | null>(null);
  const [revealSecretOpened, setRevealSecretOpened] = useState(false);
  const [revealingSecret, setRevealingSecret] = useState(false);
  const [saving, setSaving] = useState(false);
  const [testingConnection, setTestingConnection] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [snapshotting, setSnapshotting] = useState(false);
  const [restoring, setRestoring] = useState(false);
  const [pendingChanges, setPendingChanges] = useState<number | null>(null);

  const getErrorCodeAndMessage = (e: unknown): { code?: string; message: string } => {
    // Tauri invoke errors may come in different shapes across versions:
    // - { code, message }
    // - { error: { code, message } }
    // - Error with message containing JSON
    // - plain string
    const anyErr = e as AppErrorLike & { error?: AppErrorLike };
    const rawMessage = anyErr?.message ?? anyErr?.error?.message ?? (e instanceof Error ? e.message : String(e));
    const rawCode = anyErr?.code ?? anyErr?.error?.code;

    // Try parse JSON message like: {"code":"...","message":"..."}
    if (!rawCode && rawMessage && rawMessage.trim().startsWith('{')) {
      try {
        const parsed = JSON.parse(rawMessage) as AppErrorLike;
        return {
          code: parsed?.code,
          message: parsed?.message ?? rawMessage,
        };
      } catch {
        // ignore
      }
    }

    return { code: rawCode, message: rawMessage };
  };

  useEffect(() => {
    loadSyncConfig();
    loadSyncStatus();
  }, []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const v = await getVersion();
        if (!cancelled) setAppVersion(v);
      } catch (error: unknown) {
        // In web mode (non-Tauri), this may fail. Keep UI stable with a fallback.
        logger.debug('Get app version skipped (non-Tauri runtime):', error);
        if (!cancelled) setAppVersion(null);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const loadSyncConfig = async () => {
    try {
      const config = await syncManager.getConfig();
      setSyncConfig(config);
      setSyncEnabled(config.enabled);
      setBucket(config.bucket || '');
      setEndpoint(config.endpoint || '');
      setAccessKey(config.access_key || '');
      setAutoSyncIntervalMinutes(Math.max(1, Number(config.auto_sync_interval_minutes || 1)));
      setSecretKey('');
      setSecretKeySaved(Boolean(config.has_secret_key));
      setSecretKeyMasked(config.secret_key_masked || null);
      setSecretKeyRevealed(false);
      setSecretKeyEditBaseline(null);
    } catch (error: unknown) {
      logger.error('Load sync config failed:', error);
    }
  };

  const loadSyncStatus = async () => {
    try {
      const status = await syncManager.getStatus();
      setPendingChanges(status.pending_changes);
    } catch (error: unknown) {
      logger.error('Load sync status failed:', error);
      setPendingChanges(null);
    }
  };

  const handleToggleRevealSecretKey = () => {
    if (secretKeyRevealed) {
      setSecretKey('');
      setSecretKeyRevealed(false);
      setSecretKeyEditBaseline(null);
      return;
    }
    setRevealSecretOpened(true);
  };

  const handleConfirmRevealSecretKey = async () => {
    setRevealingSecret(true);
    try {
      const value = await syncManager.revealSecretKey();
      setSecretKey(value);
      setSecretKeyRevealed(true);
      setSecretKeyEditBaseline(null);
      setRevealSecretOpened(false);
    } catch (error: unknown) {
      showError((error as { message?: string })?.message ?? t('settings.sync.revealSecretFailed'));
    } finally {
      setRevealingSecret(false);
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
    // Validate endpoint: must be empty or start with https://
    const endpointTrimmed = endpoint.trim();
    if (endpointTrimmed && !endpointTrimmed.toLowerCase().startsWith('https://')) {
      showError(t('settings.sync.endpointHttpsRequired'));
      return;
    }

    setSaving(true);
    try {
      const hasExistingAccessKey = Boolean(syncConfig?.access_key);
      const baseline = (secretKeyEditBaseline ?? '').trim();
      const current = secretKey.trim();
      const shouldSendSecretKey = syncConfigEditing && current !== '' && current !== baseline;
      const minutes = Math.max(1, Math.floor(autoSyncIntervalMinutes || 1));
      await syncManager.updateConfig({
        enabled: syncEnabled,
        bucket,
        endpoint: endpoint || undefined,
        // Avoid overwriting stored credentials with empty strings.
        accessKey: accessKey.trim() === '' && hasExistingAccessKey ? undefined : accessKey,
        // Only send secret key when user is explicitly editing it.
        secretKey: shouldSendSecretKey ? secretKey : undefined,
        autoSyncIntervalMinutes: minutes,
      });
      showSuccess(t('settings.sync.configSaved'));
      await loadSyncConfig();
      setSyncConfigEditing(false);
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('settings.sync.configSaveFailed'));
    } finally {
      setSaving(false);
    }
  };

  const handleEnterSyncConfigEdit = async () => {
    setSyncConfigEditing(true);
    // Requirement: show Secret Key in plaintext while editing.
    // If a secret key exists, reveal it immediately when entering edit mode.
    if (secretKeySaved) {
      setRevealingSecret(true);
      try {
        const value = await syncManager.revealSecretKey();
        const v = value.trim();
        setSecretKey(v);
        setSecretKeyEditBaseline(v);
        setSecretKeyRevealed(true);
      } catch (error: unknown) {
        showError((error as { message?: string })?.message ?? t('settings.sync.revealSecretFailed'));
      } finally {
        setRevealingSecret(false);
      }
    } else {
      setSecretKeyEditBaseline('');
    }
  };

  const handleCancelSyncConfigEdit = async () => {
    setSyncConfigEditing(false);
    await loadSyncConfig();
  };

  const handleToggleSyncEnabled = async (nextEnabled: boolean) => {
    // Switch is not gated by edit mode.
    setSyncEnabled(nextEnabled);
    try {
      if (nextEnabled) {
        // Re-check config completeness from backend state (includes has_secret_key).
        const cfg = await syncManager.getConfig();
        const ok = Boolean(cfg.bucket?.trim()) && Boolean(cfg.access_key?.trim()) && Boolean(cfg.has_secret_key);
        if (!ok) {
          showError(t('settings.sync.configIncomplete'));
          setSyncEnabled(false);
          // Help user fix it immediately.
          await handleEnterSyncConfigEdit();
          return;
        }
      }

      await syncManager.setEnabled(nextEnabled);
      await loadSyncStatus();
    } catch (e: unknown) {
      // Backend may reject enabling if config incomplete.
      const { message } = getErrorCodeAndMessage(e);
      showError(message || t('settings.sync.configSaveFailed'));
      setSyncEnabled(false);
    }
  };

  const handleTestConnection = async () => {
    if (syncConfigEditing) {
      // 本地先做一次必填校验，减少无效后端请求。
      // 注意：编辑态允许“沿用已保存密钥”，因此使用“草稿值 + 已保存值”做有效性判断。
      const hasBucket = bucket.trim() !== '';
      const hasAccessKey = accessKey.trim() !== '' || Boolean(syncConfig?.access_key?.trim());
      const hasSecretKey = secretKey.trim() !== '' || secretKeySaved;
      if (!hasBucket || !hasAccessKey || !hasSecretKey) {
        showError(t('settings.sync.configIncomplete'));
        return;
      }
    }

    setTestingConnection(true);
    try {
      const draft = syncConfigEditing
        ? {
            bucket: bucket.trim(),
            endpoint: endpoint.trim() || undefined,
            accessKey: accessKey.trim() || undefined,
            // Allow fallback to persisted secret when user did not re-enter it.
            secretKey: secretKey.trim() || undefined,
          }
        : undefined;

      await syncManager.testConnection(draft);
      showSuccess(t('settings.sync.testConnectionSuccess'));
    } catch (e: unknown) {
      const { code, message } = getErrorCodeAndMessage(e);
      if (code === 'SYNC_CONFIG_INCOMPLETE') {
        showError(t('settings.sync.configIncomplete'));
        if (!syncConfigEditing) {
          await handleEnterSyncConfigEdit();
        }
        return;
      }
      showError(message || t('settings.sync.testConnectionFailed'), t('settings.sync.testConnectionFailed'));
    } finally {
      setTestingConnection(false);
    }
  };

  const handleSync = async () => {
    setSyncing(true);
    try {
      await syncManager.sync();
      showSuccess(t('settings.sync.syncComplete'));
    } catch (e: unknown) {
      const { message } = getErrorCodeAndMessage(e);
      showError(message || t('settings.sync.syncFailed'));
    } finally {
      setSyncing(false);
      await loadSyncStatus();
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
      await loadSyncStatus();
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
      await loadSyncStatus();
    }
  };

  return (
    <Stack gap="md" w="100%" pb="xl" style={{ minWidth: 0 }}>
      <Title order={3}>{t('settings.title')}</Title>
      <ConfirmModal
        opened={revealSecretOpened}
        onClose={() => setRevealSecretOpened(false)}
        onConfirm={handleConfirmRevealSecretKey}
        title={t('settings.sync.revealSecretTitle')}
        message={t('settings.sync.revealSecretMessage')}
        confirmLabel={t('settings.sync.revealSecretConfirm')}
        confirmColor="orange"
        loading={revealingSecret}
      />

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
                onChange={(e) => handleToggleSyncEnabled(e.currentTarget.checked)}
              />
            </Group>
            <Text size="xs" c="dimmed" mb="md">
              {t('settings.sync.description')}
            </Text>

            {syncConfig && (
              <Group gap="xs" mb="md">
                <Text size="xs" c="dimmed">
                  {t('settings.sync.deviceId', { id: syncConfig.device_id })}
                </Text>
                <Badge variant="light" size="sm" color={pendingChanges && pendingChanges > 0 ? 'orange' : 'gray'}>
                  {t('settings.sync.pendingChanges', { count: pendingChanges ?? 0 })}
                </Badge>
              </Group>
            )}
          </div>

          <TextInput
            label={t('settings.sync.bucket')}
            placeholder="my-project-sync"
            value={bucket}
            onChange={(e) => setBucket(e.currentTarget.value)}
            required
            readOnly={!syncConfigEditing}
          />

          <TextInput
            label={t('settings.sync.endpoint')}
            placeholder="https://xxxxxxxx.r2.cloudflarestorage.com"
            description={t('settings.sync.endpointDesc')}
            value={endpoint}
            onChange={(e) => setEndpoint(e.currentTarget.value)}
            readOnly={!syncConfigEditing}
            error={
              syncConfigEditing &&
              endpoint.trim() &&
              !endpoint.trim().toLowerCase().startsWith('https://')
                ? t('settings.sync.endpointHttpsRequired')
                : undefined
            }
          />

          <TextInput
            label={t('settings.sync.accessKey')}
            placeholder={t('settings.sync.accessKeyPlaceholder')}
            value={accessKey}
            onChange={(e) => setAccessKey(e.currentTarget.value)}
            required
            readOnly={!syncConfigEditing}
          />

          <NumberInput
            label={t('settings.sync.autoSyncInterval')}
            description={t('settings.sync.autoSyncIntervalDesc')}
            value={autoSyncIntervalMinutes}
            onChange={(value) => setAutoSyncIntervalMinutes(typeof value === 'number' ? value : 1)}
            min={1}
            step={1}
            allowDecimal={false}
            clampBehavior="strict"
            readOnly={!syncConfigEditing}
          />

          {syncConfigEditing && (
            <TextInput
              label={t('settings.sync.secretKey')}
              placeholder={t('settings.sync.secretKeyPlaceholder')}
              value={secretKey}
              onChange={(e) => setSecretKey(e.currentTarget.value)}
              required={!secretKeySaved}
            />
          )}

          {!syncConfigEditing && secretKeySaved && !secretKeyRevealed && (
            <TextInput
              label={t('settings.sync.secretKey')}
              value={secretKeyMasked ?? t('common.saved')}
              readOnly
              rightSection={
                <Group gap={4}>
                  <ActionIcon
                    variant="subtle"
                    onClick={handleToggleRevealSecretKey}
                    aria-label={t('settings.sync.showSecret')}
                  >
                    <IconEye size={16} />
                  </ActionIcon>
                </Group>
              }
            />
          )}

          {!syncConfigEditing && secretKeySaved && secretKeyRevealed && (
            <TextInput
              label={t('settings.sync.secretKey')}
              value={secretKey}
              readOnly
              rightSection={
                <ActionIcon
                  variant="subtle"
                  onClick={handleToggleRevealSecretKey}
                  aria-label={t('settings.sync.hideSecret')}
                >
                  <IconEyeOff size={16} />
                </ActionIcon>
              }
            />
          )}

          {!syncConfigEditing && !secretKeySaved && (
            <TextInput
              label={t('settings.sync.secretKey')}
              placeholder={t('settings.sync.secretKeyPlaceholder')}
              value={secretKey}
              onChange={(e) => setSecretKey(e.currentTarget.value)}
              required={!secretKeySaved}
              readOnly
            />
          )}

          <Group justify="flex-start" mt="xs">
            {!syncConfigEditing ? (
              <>
                <Button
                  variant="light"
                  leftSection={<IconEdit size={16} />}
                  onClick={handleEnterSyncConfigEdit}
                >
                  {t('common.edit')}
                </Button>
                <Button variant="light" onClick={handleTestConnection} loading={testingConnection}>
                  {t('settings.sync.testConnection')}
                </Button>
              </>
            ) : (
              <>
                <Button variant="light" onClick={handleTestConnection} loading={testingConnection}>
                  {t('settings.sync.testConnection')}
                </Button>
                <Button variant="subtle" onClick={handleCancelSyncConfigEdit} disabled={saving}>
                  {t('common.cancel')}
                </Button>
                <Button
                  variant="gradient"
                  gradient={{ from: 'cyan', to: 'blue' }}
                  onClick={handleSaveSyncConfig}
                  loading={saving}
                >
                  {t('common.save')}
                </Button>
              </>
            )}
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

      {/* Application Logs */}
      <Paper>
        <Stack gap="xs">
          <Text size="sm" fw={500}>
            {t('settings.logs.title')}
          </Text>
          <Text size="xs" c="dimmed" mb="xs">
            {t('settings.logs.description')}
          </Text>
          <Button
            leftSection={<IconFileText size={18} />}
            variant="light"
            onClick={() => navigate('/logs')}
            style={{ alignSelf: 'flex-start' }}
          >
            {t('settings.logs.viewButton')}
          </Button>
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
            {t('settings.about.version', { version: appVersion ?? '—' })}
          </Text>
          <Text size="xs" c="dimmed">
            {t('settings.about.schema')}
          </Text>
        </Stack>
      </Paper>
    </Stack>
  );
}
