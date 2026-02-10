import { Button, Group, Paper, Stack, Text, Title, TextInput, PasswordInput, Switch, Divider } from '@mantine/core';
import { IconDownload, IconUpload, IconCloud, IconCloudUpload, IconRestore } from '@tabler/icons-react';
import { useState, useEffect, useRef } from 'react';
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
    } catch (error: any) {
      console.error('Load sync config failed:', error);
    }
  };

  const handleExport = async () => {
    setExporting(true);
    try {
      // 1. 获取导出的 JSON 数据
      const jsonString = await exportApi.exportJson();

      // 2. 打开保存对话框
      const filePath = await save({
        title: '导出数据',
        filters: [
          {
            name: 'JSON',
            extensions: ['json'],
          },
        ],
        defaultPath: `project-management-backup-${new Date().toISOString().split('T')[0]}.json`,
      });

      // 3. 如果用户选择了文件路径，保存文件
      if (filePath) {
        await writeTextFile(filePath, jsonString);
        showSuccess('数据导出成功');
      }
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? '导出失败');
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
      if (result.persons > 0) parts.push(`${result.persons} 个成员`);
      if (result.partners > 0) parts.push(`${result.partners} 个合作方`);
      if (result.projects > 0) parts.push(`${result.projects} 个项目`);
      if (result.assignments > 0) parts.push(`${result.assignments} 条参与记录`);
      if (result.status_history > 0) parts.push(`${result.status_history} 条状态历史`);
      const msg = parts.length > 0
        ? `导入成功：${parts.join('、')}${result.skipped_duplicates > 0 ? `（跳过 ${result.skipped_duplicates} 条重复）` : ''}`
        : `全部数据已存在，跳过 ${result.skipped_duplicates} 条重复`;
      showSuccess(msg);
      // Invalidate all stores after import
      invalidatePartners();
      invalidatePersons();
      invalidateTags();
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? '导入失败');
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
      showSuccess('同步配置已保存');
      await loadSyncConfig();
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? '保存配置失败');
    } finally {
      setSaving(false);
    }
  };

  const handleSync = async () => {
    setSyncing(true);
    try {
      await syncManager.sync();
      showSuccess('同步完成');
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? '同步失败');
    } finally {
      setSyncing(false);
    }
  };

  const handleCreateSnapshot = async () => {
    setSnapshotting(true);
    try {
      const result = await syncManager.createSnapshot();
      showSuccess('快照已创建: ' + result.substring(0, 12));
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? '创建快照失败');
    } finally {
      setSnapshotting(false);
    }
  };

  const handleRestoreSnapshot = async () => {
    setRestoring(true);
    try {
      const result = await syncManager.restoreSnapshot();
      showSuccess('快照已恢复: ' + result.substring(0, 12));
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? '恢复快照失败');
    } finally {
      setRestoring(false);
    }
  };

  return (
    <Stack gap="md" w="100%" pb="xl" style={{ minWidth: 0 }}>
      <Title order={3}>设置</Title>

      {/* S3 同步配置 */}
      <Paper>
        <Stack gap="md">
          <div>
            <Group justify="space-between" mb="xs">
              <Text size="sm" fw={500}>
                S3 在线同步
              </Text>
              <Switch
                checked={syncEnabled}
                onChange={(e) => setSyncEnabled(e.currentTarget.checked)}
              />
            </Group>
            <Text size="xs" c="dimmed" mb="md">
              配置 S3 对象存储实现多设备数据同步（支持 AWS S3, Cloudflare R2, MinIO 等）
            </Text>

            {syncConfig && (
              <Text size="xs" c="dimmed" mb="md">
                设备 ID: {syncConfig.device_id}
              </Text>
            )}
          </div>

          <TextInput
            label="Bucket 名称"
            placeholder="my-project-sync"
            value={bucket}
            onChange={(e) => setBucket(e.currentTarget.value)}
            required
          />

          <TextInput
            label="Endpoint URL (可选)"
            placeholder="https://xxxxxxxx.r2.cloudflarestorage.com"
            description="留空使用 AWS S3，填写自定义 endpoint 支持 R2/MinIO"
            value={endpoint}
            onChange={(e) => setEndpoint(e.currentTarget.value)}
          />

          <TextInput
            label="Access Key"
            placeholder="访问密钥"
            value={accessKey}
            onChange={(e) => setAccessKey(e.currentTarget.value)}
            required
          />

          <PasswordInput
            label="Secret Key"
            placeholder="密钥"
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
              保存配置
            </Button>
          </Group>
        </Stack>
      </Paper>

      {/* 同步操作 */}
      {syncEnabled && (
        <Paper>
          <Stack gap="md">
            <Text size="sm" fw={500}>
              同步操作
            </Text>

            <Group>
              <Button
                leftSection={<IconCloud size={18} />}
                variant="gradient"
                gradient={{ from: 'teal', to: 'cyan' }}
                onClick={handleSync}
                loading={syncing}
              >
                立即同步
              </Button>

              <Button
                leftSection={<IconCloudUpload size={18} />}
                variant="light"
                color="blue"
                onClick={handleCreateSnapshot}
                loading={snapshotting}
              >
                创建快照
              </Button>

              <Button
                leftSection={<IconRestore size={18} />}
                variant="light"
                color="orange"
                onClick={handleRestoreSnapshot}
                loading={restoring}
              >
                恢复快照
              </Button>
            </Group>

            <Text size="xs" c="dimmed">
              同步：上传本地更改并下载远程更改 | 快照：完整备份到 S3 | 恢复：从 S3 完整恢复
            </Text>
          </Stack>
        </Paper>
      )}

      <Divider />

      {/* 数据导出 */}
      <Paper>
        <Stack gap="md">
          <div>
            <Text size="sm" fw={500} mb="xs">
              数据导出 / 导入
            </Text>
            <Text size="xs" c="dimmed" mb="md">
              导出所有数据为 JSON 文件用于备份；导入 JSON 文件恢复数据（重复 ID 自动跳过）。
            </Text>
            <Group>
              <Button
                leftSection={<IconDownload size={18} />}
                variant="gradient"
                gradient={{ from: 'indigo', to: 'violet' }}
                onClick={handleExport}
                loading={exporting}
              >
                导出数据
              </Button>
              <Button
                leftSection={<IconUpload size={18} />}
                variant="light"
                color="teal"
                onClick={() => fileInputRef.current?.click()}
                loading={importing}
              >
                导入数据
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

      {/* 关于 */}
      <Paper>
        <Stack gap="xs">
          <Text size="sm" fw={500}>
            关于
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
