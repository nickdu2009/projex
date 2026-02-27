import {
  Button,
  Group,
  Paper,
  SegmentedControl,
  Stack,
  Switch,
  Text,
  TextInput,
  Title,
  ScrollArea,
  Badge,
  Select,
  Alert,
} from '@mantine/core';
import {
  IconArrowLeft,
  IconCopy,
  IconDownload,
  IconTrash,
  IconRefresh,
  IconSearch,
  IconAlertCircle,
} from '@tabler/icons-react';
import { useEffect, useState, useRef, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useIsMobile } from '../utils/useIsMobile';
import { logsApi, type LogFileDto, type LogTailResp } from '../api/logs';
import { showError, showSuccess } from '../utils/errorToast';
import { logger } from '../utils/logger';
import { ConfirmModal } from '../components/ConfirmModal';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';

export function Logs() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const isMobile = useIsMobile();
  const [files, setFiles] = useState<LogFileDto[]>([]);
  const [selectedFile, setSelectedFile] = useState<string>('');
  const [logContent, setLogContent] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [autoRefresh, setAutoRefresh] = useState(false);
  const [redact, setRedact] = useState(true);
  const [searchQuery, setSearchQuery] = useState('');
  const [clearing, setClearing] = useState(false);
  const [clearConfirmOpened, setClearConfirmOpened] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const intervalRef = useRef<number | null>(null);
  const [cursor, setCursor] = useState<number | undefined>(undefined);
  const [hasMore, setHasMore] = useState(true);
  const [logLevel, setLogLevel] = useState<string>('WARN');
  const [savingLevel, setSavingLevel] = useState(false);
  const [levelChanged, setLevelChanged] = useState(false);

  const loadFiles = useCallback(async () => {
    try {
      const result = await logsApi.listFiles();
      setFiles(result);
      if (result.length > 0 && !selectedFile) {
        setSelectedFile(result[0].name);
      }
    } catch (error: unknown) {
      logger.error('Load log files failed:', error);
      showError((error as { message?: string })?.message ?? t('logs.loadFailed'));
    }
  }, [selectedFile, t]);

  const loadLogLevel = async () => {
    try {
      const result = await logsApi.getLevel();
      setLogLevel(result.current_level);
      setLevelChanged(false);
    } catch (error: unknown) {
      logger.error('Load log level failed:', error);
    }
  };

  useEffect(() => {
    loadFiles();
    loadLogLevel();
  }, [loadFiles]);

  useEffect(() => {
    if (selectedFile) {
      loadLog();
    } else {
      setLogContent('');
      setCursor(undefined);
      setHasMore(true);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedFile, redact]);

  useEffect(() => {
    if (autoRefresh && selectedFile) {
      intervalRef.current = setInterval(() => {
        loadLog();
      }, 2000);
    } else if (intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [autoRefresh, selectedFile]);

  const loadLog = async () => {
    if (!selectedFile) return;
    setLoading(true);
    try {
      const result: LogTailResp = await logsApi.tail({
        file_name: selectedFile,
        max_bytes: 256 * 1024, // 256KB
        redact,
        cursor,
      });
      setLogContent(result.content);
      setCursor(result.next_cursor);
      setHasMore(result.next_cursor !== undefined);
    } catch (error: unknown) {
      logger.error('Load log content failed:', error);
      showError((error as { message?: string })?.message ?? t('logs.loadFailed'));
    } finally {
      setLoading(false);
    }
  };

  const handleLoadMore = async () => {
    if (!selectedFile || !hasMore || loading) return;
    setLoading(true);
    try {
      const result: LogTailResp = await logsApi.tail({
        file_name: selectedFile,
        max_bytes: 256 * 1024,
        redact,
        cursor,
      });
      setLogContent((prev) => result.content + '\n' + prev);
      setCursor(result.next_cursor);
      setHasMore(result.next_cursor !== undefined);
    } catch (error: unknown) {
      logger.error('Load more logs failed:', error);
      showError((error as { message?: string })?.message ?? t('logs.loadFailed'));
    } finally {
      setLoading(false);
    }
  };

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(logContent);
      showSuccess(t('logs.copySuccess'));
    } catch (error: unknown) {
      logger.error('Copy logs failed:', error);
      showError(t('logs.copyFailed'));
    }
  };

  const handleDownload = async () => {
    if (!selectedFile) return;
    try {
      const filePath = await save({
        title: t('logs.download'),
        filters: [{ name: 'Log Files', extensions: ['log', 'txt'] }],
        defaultPath: selectedFile,
      });

      if (filePath) {
        await writeTextFile(filePath, logContent);
        showSuccess(t('logs.downloadSuccess'));
      }
    } catch (error: unknown) {
      logger.error('Download log failed:', error);
      showError((error as { message?: string })?.message ?? t('logs.downloadFailed'));
    }
  };

  const handleClear = async () => {
    if (!selectedFile) return;
    setClearing(true);
    try {
      await logsApi.clear({ file_name: selectedFile });
      showSuccess(t('logs.clearSuccess'));
      setClearConfirmOpened(false);
      setLogContent('');
      setCursor(undefined);
      setHasMore(true);
      await loadFiles();
    } catch (error: unknown) {
      logger.error('Clear log failed:', error);
      showError((error as { message?: string })?.message ?? t('logs.clearFailed'));
    } finally {
      setClearing(false);
    }
  };

  const handleLogLevelChange = async (newLevel: string) => {
    setSavingLevel(true);
    try {
      await logsApi.setLevel(newLevel);
      setLogLevel(newLevel);
      setLevelChanged(true);
      showSuccess(t('logs.levelChangeSuccess'));
    } catch (error: unknown) {
      logger.error('Set log level failed:', error);
      showError((error as { message?: string })?.message ?? t('logs.levelChangeFailed'));
    } finally {
      setSavingLevel(false);
    }
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const highlightMatches = (text: string, query: string): string => {
    if (!query.trim()) return text;
    const regex = new RegExp(`(${query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
    return text.replace(regex, '<mark>$1</mark>');
  };

  const filteredContent = searchQuery.trim()
    ? logContent
        .split('\n')
        .filter((line) => line.toLowerCase().includes(searchQuery.toLowerCase()))
        .join('\n')
    : logContent;

  const displayContent = searchQuery.trim()
    ? highlightMatches(filteredContent, searchQuery)
    : filteredContent;

  const selectedFileObj = files.find((f) => f.name === selectedFile);

  return (
    <Stack gap="md" w="100%" pb="xl" style={{ minWidth: 0 }}>
      <Group justify="space-between" align="center" wrap="wrap">
        <Group wrap="nowrap">
          <Button
            variant="subtle"
            leftSection={<IconArrowLeft size={16} />}
            onClick={() => navigate('/settings')}
          >
            {t('common.back')}
          </Button>
          <Title order={3}>{t('logs.title')}</Title>
        </Group>
        <Button
          variant="light"
          leftSection={<IconRefresh size={16} />}
          onClick={loadFiles}
          loading={loading}
        >
          {t('common.refresh')}
        </Button>
      </Group>

      <ConfirmModal
        opened={clearConfirmOpened}
        onClose={() => setClearConfirmOpened(false)}
        onConfirm={handleClear}
        title={t('logs.clearConfirmTitle')}
        message={t('logs.clearConfirmMessage', { fileName: selectedFile })}
        confirmLabel={t('logs.clear')}
        confirmColor="red"
        loading={clearing}
      />

      {/* 重启提示 */}
      {levelChanged && (
        <Alert
          icon={<IconAlertCircle size={16} />}
          title={t('logs.restartRequired')}
          color="orange"
          withCloseButton
          onClose={() => setLevelChanged(false)}
        >
          {t('logs.restartRequiredDesc')}
        </Alert>
      )}

      <Paper>
        <Stack gap="md">
          {/* 日志级别设置 */}
          <Group>
            <div style={{ flex: 1 }}>
              <Text size="sm" fw={500} mb="xs">
                {t('logs.logLevel')}
              </Text>
              <Select
                value={logLevel}
                onChange={(value) => value && handleLogLevelChange(value)}
                data={[
                  { value: 'ERROR', label: t('logs.levelError') },
                  { value: 'WARN', label: t('logs.levelWarn') },
                  { value: 'INFO', label: t('logs.levelInfo') },
                  { value: 'DEBUG', label: t('logs.levelDebug') },
                ]}
                disabled={savingLevel}
                style={{ maxWidth: 200 }}
              />
              <Text size="xs" c="dimmed" mt="xs">
                {t('logs.logLevelDesc')}
              </Text>
            </div>
          </Group>

          {/* 文件选择与文件信息 */}
          <Group justify="space-between" align="flex-start">
            <div style={{ flex: 1 }}>
              <Text size="sm" fw={500} mb="xs">
                {t('logs.selectFile')}
              </Text>
              {files.length > 0 ? (
                <SegmentedControl
                  value={selectedFile}
                  onChange={setSelectedFile}
                  data={files.map((f) => ({
                    value: f.name,
                    label: f.name,
                  }))}
                  style={{ maxWidth: '100%' }}
                />
              ) : (
                <Text size="sm" c="dimmed">
                  {t('logs.noFiles')}
                </Text>
              )}
            </div>
            {selectedFileObj && (
              <Badge variant="light" size="lg">
                {t('logs.fileSize', { size: formatFileSize(selectedFileObj.size_bytes) })}
              </Badge>
            )}
          </Group>

          {/* 搜索与开关 */}
          <Stack gap="xs">
            <TextInput
              placeholder={t('logs.search')}
              leftSection={<IconSearch size={16} />}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.currentTarget.value)}
            />
            <Group wrap="wrap" gap="xs">
              <Switch
                label={t('logs.autoRefresh')}
                checked={autoRefresh}
                onChange={(e) => setAutoRefresh(e.currentTarget.checked)}
              />
              <Switch
                label={t('logs.redactSensitive')}
                checked={redact}
                onChange={(e) => setRedact(e.currentTarget.checked)}
              />
            </Group>
          </Stack>

          {/* 操作按钮 */}
          <Group wrap="wrap" gap="xs">
            <Button
              leftSection={<IconCopy size={16} />}
              variant="light"
              onClick={handleCopy}
              disabled={!logContent}
              fullWidth={isMobile}
            >
              {t('logs.copy')}
            </Button>
            <Button
              leftSection={<IconDownload size={16} />}
              variant="light"
              onClick={handleDownload}
              disabled={!logContent}
              fullWidth={isMobile}
            >
              {t('logs.download')}
            </Button>
            <Button
              leftSection={<IconTrash size={16} />}
              variant="light"
              color="red"
              onClick={() => setClearConfirmOpened(true)}
              disabled={!selectedFile}
              fullWidth={isMobile}
            >
              {t('logs.clear')}
            </Button>
          </Group>

          {/* 日志内容显示区 */}
          <div>
            <Text size="xs" c="dimmed" mb="xs">
              {searchQuery.trim() && filteredContent.split('\n').length === 0
                ? t('logs.noMatches')
                : ''}
            </Text>
            <ScrollArea
              h={isMobile ? 300 : 500}
              style={{
                border: '1px solid rgba(0, 0, 0, 0.1)',
                borderRadius: 8,
                padding: '12px',
                backgroundColor: '#f8f9fa',
              }}
              ref={scrollRef}
            >
              {logContent ? (
                <>
                  {hasMore && (
                    <Button
                      variant="subtle"
                      size="xs"
                      onClick={handleLoadMore}
                      loading={loading}
                      mb="xs"
                      fullWidth
                    >
                      {t('logs.loadingMore')}
                    </Button>
                  )}
                  <Text
                    component="pre"
                    style={{
                      fontFamily: 'monospace',
                      fontSize: 12,
                      lineHeight: 1.5,
                      whiteSpace: 'pre-wrap',
                      wordBreak: 'break-word',
                      margin: 0,
                    }}
                    dangerouslySetInnerHTML={{ __html: displayContent }}
                  />
                </>
              ) : (
                <Text size="sm" c="dimmed" ta="center" py="xl">
                  {t('logs.emptyLog')}
                </Text>
              )}
            </ScrollArea>
          </div>
        </Stack>
      </Paper>
    </Stack>
  );
}
