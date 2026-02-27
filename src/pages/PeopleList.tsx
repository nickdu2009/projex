import {
  Alert,
  Badge,
  Button,
  Card,
  Code,
  Flex,
  Group,
  List,
  Loader,
  Modal,
  Paper,
  ScrollArea,
  Stack,
  Table,
  Text,
  Title,
} from '@mantine/core';
import { useDisclosure } from '@mantine/hooks';
import { notifications } from '@mantine/notifications';
import {
  IconAlertCircle,
  IconDownload,
  IconPlus,
  IconUpload,
  IconUsers,
} from '@tabler/icons-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { peopleApi, type PersonDto, type PersonImportResult } from '../api/people';
import { EmptyState } from '../components/EmptyState';
import { showError } from '../utils/errorToast';
import { logger } from '../utils/logger';
import { getRoleLabel } from '../utils/roleLabel';
import { useIsMobile } from '../utils/useIsMobile';

export function PeopleList() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const isMobile = useIsMobile();
  const [list, setList] = useState<PersonDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [showInactive, setShowInactive] = useState(false);

  // Import modal state
  const [importOpened, { open: openImport, close: closeImport }] = useDisclosure(false);
  const [importCsvText, setImportCsvText] = useState('');
  const [importPreviewRows, setImportPreviewRows] = useState<string[][]>([]);
  const [importResult, setImportResult] = useState<PersonImportResult | null>(null);
  const [importing, setImporting] = useState(false);
  const [exporting, setExporting] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await peopleApi.list(!showInactive);
      setList(data);
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('common.failedToLoad'));
    } finally {
      setLoading(false);
    }
  }, [showInactive, t]);

  useEffect(() => {
    load();
  }, [load]);

  // ── Export ──────────────────────────────────────────────────────────────────

  const handleExport = useCallback(async () => {
    setExporting(true);
    try {
      const csv = await peopleApi.exportCsv();
      const savePath = await save({
        defaultPath: `people-${new Date().toISOString().slice(0, 10)}.csv`,
        filters: [{ name: 'CSV', extensions: ['csv'] }],
      });
      if (savePath) {
        await writeTextFile(savePath, csv);
        const count = csv.split('\n').filter((l) => l.trim()).length - 1; // subtract header
        notifications.show({
          color: 'teal',
          message: t('person.list.exportSuccess', { count }),
        });
      }
    } catch (e: unknown) {
      logger.error('Export persons CSV failed:', e);
      showError((e as { message?: string })?.message ?? t('person.list.exportFailed'));
    } finally {
      setExporting(false);
    }
  }, [t]);

  // ── Import ──────────────────────────────────────────────────────────────────

  const parsePreview = useCallback((csv: string) => {
    const lines = csv.split('\n').filter((l) => l.trim());
    // Skip header, show up to 5 data rows
    return lines.slice(1, 6).map((line) => {
      const fields: string[] = [];
      let current = '';
      let inQuotes = false;
      for (let i = 0; i < line.length; i++) {
        const c = line[i];
        if (c === '"' && inQuotes && line[i + 1] === '"') {
          current += '"';
          i++;
        } else if (c === '"') {
          inQuotes = !inQuotes;
        } else if (c === ',' && !inQuotes) {
          fields.push(current);
          current = '';
        } else {
          current += c;
        }
      }
      fields.push(current);
      return fields;
    });
  }, []);

  const handleFileSelect = useCallback(
    (file: File) => {
      const reader = new FileReader();
      reader.onload = (e) => {
        const text = (e.target?.result as string) ?? '';
        setImportCsvText(text);
        setImportPreviewRows(parsePreview(text));
        setImportResult(null);
      };
      reader.readAsText(file, 'UTF-8');
    },
    [parsePreview],
  );

  const handleFileInputChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (file) handleFileSelect(file);
      // Reset input so same file can be re-selected
      e.target.value = '';
    },
    [handleFileSelect],
  );

  const handleImportConfirm = useCallback(async () => {
    if (!importCsvText) return;
    setImporting(true);
    try {
      const result = await peopleApi.importCsv(importCsvText);
      setImportResult(result);
      notifications.show({
        color: result.skipped > 0 ? 'yellow' : 'teal',
        message: t('person.list.importSuccess', {
          created: result.created,
          updated: result.updated,
          skipped: result.skipped,
        }),
      });
      await load();
    } catch (e: unknown) {
      logger.error('Import persons CSV failed:', e);
      showError((e as { message?: string })?.message ?? t('person.list.importFailed'));
    } finally {
      setImporting(false);
    }
  }, [importCsvText, t, load]);

  const handleImportClose = useCallback(() => {
    closeImport();
    setImportCsvText('');
    setImportPreviewRows([]);
    setImportResult(null);
  }, [closeImport]);

  // ── Render ──────────────────────────────────────────────────────────────────

  return (
    <Stack gap="md" w="100%" pb="xl" style={{ minWidth: 0 }}>
      {/* Hidden file input for CSV selection */}
      <input
        ref={fileInputRef}
        type="file"
        accept=".csv,text/csv"
        style={{ display: 'none' }}
        onChange={handleFileInputChange}
      />

      <Flex wrap="wrap" gap="sm" justify="space-between" align="center">
        <Title order={3}>{t('person.list.title')}</Title>
        <Group gap="xs">
          <Button
            variant="light"
            color="teal"
            leftSection={<IconDownload size={16} />}
            loading={exporting}
            onClick={handleExport}
            size={isMobile ? 'xs' : 'sm'}
          >
            {t('person.list.exportCsv')}
          </Button>
          <Button
            variant="light"
            color="indigo"
            leftSection={<IconUpload size={16} />}
            onClick={openImport}
            size={isMobile ? 'xs' : 'sm'}
          >
            {t('person.list.importCsv')}
          </Button>
          <Button
            variant="gradient"
            gradient={{ from: 'indigo', to: 'violet' }}
            leftSection={<IconPlus size={18} />}
            onClick={() => navigate('/people/new')}
            size={isMobile ? 'xs' : 'sm'}
          >
            {t('person.list.new')}
          </Button>
        </Group>
      </Flex>

      <Paper>
        <Button variant="subtle" size="xs" onClick={() => setShowInactive((v) => !v)}>
          {showInactive ? t('person.list.activeOnly') : t('person.list.showInactive')}
        </Button>
      </Paper>

      <Paper>
        {loading ? (
          <Flex justify="center" py="xl">
            <Loader size="sm" />
          </Flex>
        ) : list.length === 0 ? (
          <EmptyState
            icon={IconUsers}
            title={t('person.list.emptyTitle')}
            description={t('person.list.emptyDesc')}
            actionLabel={t('person.list.new')}
            onAction={() => navigate('/people/new')}
          />
        ) : isMobile ? (
          /* Mobile card view */
          <Stack gap="xs" p="xs">
            {list.map((p) => (
              <Card
                key={p.id}
                padding="sm"
                radius="md"
                withBorder
                style={{ cursor: 'pointer' }}
                onClick={() => navigate(`/people/${p.id}`)}
              >
                <Group justify="space-between" wrap="nowrap" gap="xs">
                  <Stack gap={4} style={{ minWidth: 0, flex: 1 }}>
                    <Text
                      fw={600}
                      size="sm"
                      style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}
                    >
                      {p.display_name}
                    </Text>
                    {p.email && <Text size="xs" c="dimmed">{p.email}</Text>}
                    {p.role && <Text size="xs" c="dimmed">{getRoleLabel(p.role)}</Text>}
                  </Stack>
                  <Badge size="xs" color={p.is_active ? 'teal' : 'gray'} style={{ flexShrink: 0 }}>
                    {p.is_active ? t('common.active') : t('common.inactive')}
                  </Badge>
                </Group>
              </Card>
            ))}
          </Stack>
        ) : (
          /* Desktop table view */
          <Table.ScrollContainer minWidth={600}>
            <Table striped="even" highlightOnHover>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>{t('person.list.colName')}</Table.Th>
                  <Table.Th>{t('person.list.colEmail')}</Table.Th>
                  <Table.Th>{t('person.list.colRole')}</Table.Th>
                  <Table.Th>{t('person.list.colNote')}</Table.Th>
                  <Table.Th>{t('person.list.colStatus')}</Table.Th>
                  <Table.Th>{t('person.list.colActions')}</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {list.map((p) => (
                  <Table.Tr key={p.id}>
                    <Table.Td>{p.display_name}</Table.Td>
                    <Table.Td>
                      <Text size="sm" c="dimmed">
                        {p.email || '—'}
                      </Text>
                    </Table.Td>
                    <Table.Td>
                      <Text size="sm">{p.role ? getRoleLabel(p.role) : '—'}</Text>
                    </Table.Td>
                    <Table.Td>
                      <Text size="sm" c="dimmed">
                        {p.note || '—'}
                      </Text>
                    </Table.Td>
                    <Table.Td>{p.is_active ? t('common.active') : t('common.inactive')}</Table.Td>
                    <Table.Td>
                      <Button
                        variant="subtle"
                        size="xs"
                        onClick={() => navigate(`/people/${p.id}`)}
                      >
                        {t('common.view')}
                      </Button>
                    </Table.Td>
                  </Table.Tr>
                ))}
              </Table.Tbody>
            </Table>
          </Table.ScrollContainer>
        )}
      </Paper>

      {/* Import Modal */}
      <Modal
        opened={importOpened}
        onClose={handleImportClose}
        title={t('person.list.importModalTitle')}
        size="lg"
      >
        <Stack gap="md">
          <Text size="sm" c="dimmed">
            {t('person.list.importModalDesc')}
          </Text>

          {/* CSV format hint */}
          <Code block style={{ fontSize: 12 }}>
            display_name,email,role,note,is_active{'\n'}
            Alice,alice@example.com,backend_developer,Senior dev,true{'\n'}
            Bob,bob@example.com,tester,,true
          </Code>

          {/* File selector */}
          <Button
            variant="light"
            leftSection={<IconUpload size={16} />}
            onClick={() => fileInputRef.current?.click()}
            fullWidth
          >
            {t('person.list.importDropzone')}
          </Button>

          {/* Preview table */}
          {importPreviewRows.length > 0 && (
            <Stack gap="xs">
              <Text size="sm" fw={500}>
                {t('person.list.importPreview')}
              </Text>
              <ScrollArea>
                <Table fz="xs" withTableBorder withColumnBorders>
                  <Table.Thead>
                    <Table.Tr>
                      {['display_name', 'email', 'role', 'note', 'is_active'].map((h) => (
                        <Table.Th key={h}>{h}</Table.Th>
                      ))}
                    </Table.Tr>
                  </Table.Thead>
                  <Table.Tbody>
                    {importPreviewRows.map((row, i) => (
                      <Table.Tr key={i}>
                        {row.map((cell, j) => (
                          <Table.Td key={j}>{cell}</Table.Td>
                        ))}
                      </Table.Tr>
                    ))}
                  </Table.Tbody>
                </Table>
              </ScrollArea>
            </Stack>
          )}

          {/* Import result */}
          {importResult && (
            <Stack gap="xs">
              {importResult.errors.length > 0 && (
                <Alert
                  icon={<IconAlertCircle size={16} />}
                  color="yellow"
                  title={t('person.list.importErrorsTitle')}
                >
                  <List size="xs" spacing={2}>
                    {importResult.errors.map((err, i) => (
                      <List.Item key={i}>{err}</List.Item>
                    ))}
                  </List>
                </Alert>
              )}
            </Stack>
          )}

          <Group justify="flex-end" gap="sm">
            <Button variant="subtle" onClick={handleImportClose}>
              {t('person.list.importCancel')}
            </Button>
            <Button
              variant="gradient"
              gradient={{ from: 'indigo', to: 'violet' }}
              loading={importing}
              disabled={!importCsvText || importing}
              onClick={handleImportConfirm}
            >
              {t('person.list.importConfirm')}
            </Button>
          </Group>
        </Stack>
      </Modal>
    </Stack>
  );
}
