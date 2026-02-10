import { Button, Group, Paper, Stack, Text, Title } from '@mantine/core';
import { IconDownload } from '@tabler/icons-react';
import { useState } from 'react';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { exportApi } from '../api/export';
import { showError, showSuccess } from '../utils/errorToast';

export function Settings() {
  const [exporting, setExporting] = useState(false);

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

  return (
    <Stack gap="md" w="100%" pb="xl" style={{ minWidth: 0 }}>
      <Title order={3}>设置</Title>

      <Paper>
        <Stack gap="md">
          <div>
            <Text size="sm" fw={500} mb="xs">
              数据导出
            </Text>
            <Text size="xs" c="dimmed" mb="md">
              导出所有项目、成员、合作方、参与关系和状态历史数据为 JSON 格式，用于备份或迁移。
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
            </Group>
          </div>

          <div>
            <Text size="sm" fw={500} mb="xs">
              关于
            </Text>
            <Text size="xs" c="dimmed">
              项目管理工具 v1.0.0
            </Text>
            <Text size="xs" c="dimmed">
              Schema Version: 1
            </Text>
          </div>
        </Stack>
      </Paper>
    </Stack>
  );
}
