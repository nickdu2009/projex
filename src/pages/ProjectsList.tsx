import {
  Badge,
  Button,
  Flex,
  Group,
  Loader,
  MultiSelect,
  Pagination,
  Paper,
  Select,
  Stack,
  Table,
  Text,
  Title,
} from '@mantine/core';
import { IconFolder, IconPlus } from '@tabler/icons-react';
import { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { projectApi, type ProjectListItem, type ProjectListReq } from '../api/projects';
import { getCountries, PROJECT_STATUSES } from '../constants/countries';
import { showError } from '../utils/errorToast';
import { usePartnerStore } from '../stores/usePartnerStore';
import { usePersonStore } from '../stores/usePersonStore';
import { useTagStore } from '../stores/useTagStore';
import { getProjectStatusColor, getStatusLabel } from '../utils/statusColor';
import { EmptyState } from '../components/EmptyState';

type SortBy = 'updatedAt' | 'priority' | 'dueDate';

const PAGE_SIZE = 50;

export function ProjectsList() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const [items, setItems] = useState<ProjectListItem[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);

  // filters
  const [statusFilter, setStatusFilter] = useState<string | null>(null);
  const [countryFilter, setCountryFilter] = useState<string | null>(null);
  const [partnerFilter, setPartnerFilter] = useState<string | null>(null);
  const [ownerFilter, setOwnerFilter] = useState<string | null>(null);
  const [memberFilter, setMemberFilter] = useState<string | null>(null);
  const [tagFilter, setTagFilter] = useState<string[]>([]);
  const [showArchived, setShowArchived] = useState(false);
  const [sortBy, setSortBy] = useState<SortBy>('updatedAt');
  const [page, setPage] = useState(1);

  // Zustand stores
  const { loaded: partnersLoaded, fetch: fetchPartners, activeOptions: partnerOptions } = usePartnerStore();
  const { loaded: personsLoaded, fetch: fetchPersons, activeOptions: personOptions } = usePersonStore();
  const { tags: allTags, loaded: tagsLoaded, fetch: fetchTags } = useTagStore();

  useEffect(() => {
    if (!partnersLoaded) fetchPartners(true);
    if (!personsLoaded) fetchPersons(true);
    if (!tagsLoaded) fetchTags();
  }, [partnersLoaded, personsLoaded, tagsLoaded, fetchPartners, fetchPersons, fetchTags]);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const req: ProjectListReq = {
        onlyUnarchived: !showArchived,
        limit: PAGE_SIZE,
        offset: (page - 1) * PAGE_SIZE,
        sortBy,
      };

      // Apply sort order based on sortBy type
      if (sortBy === 'updatedAt') req.sortOrder = 'desc';
      else if (sortBy === 'priority') req.sortOrder = 'asc';
      else if (sortBy === 'dueDate') req.sortOrder = 'asc';

      // Apply filters only when set
      if (statusFilter) req.statuses = [statusFilter];
      if (countryFilter) req.countryCodes = [countryFilter];
      if (partnerFilter) req.partnerIds = [partnerFilter];
      if (ownerFilter) req.ownerPersonIds = [ownerFilter];
      if (memberFilter) req.participantPersonIds = [memberFilter];
      if (tagFilter.length > 0) req.tags = tagFilter;

      const result = await projectApi.list(req);
      setItems(result.items);
      setTotal(result.total);
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('common.failedToLoad'));
    } finally {
      setLoading(false);
    }
  }, [showArchived, page, sortBy, statusFilter, countryFilter, partnerFilter, ownerFilter, memberFilter, tagFilter, t]);

  useEffect(() => {
    load();
  }, [load]);

  // Reset to page 1 when filters change
  useEffect(() => {
    setPage(1);
  }, [statusFilter, countryFilter, partnerFilter, ownerFilter, memberFilter, tagFilter, showArchived, sortBy]);

  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));

  return (
    <Stack gap="md" w="100%" pb="xl" style={{ minWidth: 0 }}>
      <Flex wrap="wrap" gap="sm" justify="space-between" align="center">
        <Title order={3}>{t('project.list.title')}</Title>
        <Button
          variant="gradient"
          gradient={{ from: 'indigo', to: 'violet' }}
          leftSection={<IconPlus size={18} />}
          onClick={() => navigate('/projects/new')}
        >
          {t('project.list.new')}
        </Button>
      </Flex>

      <Paper>
        <Stack gap="xs">
          <Text size="xs" c="dimmed" fw={500}>{t('project.list.filters')}</Text>
          <Flex wrap="wrap" gap="xs" align="flex-end">
            <Select
              placeholder={t('project.list.statusPlaceholder')}
              clearable
              data={PROJECT_STATUSES.map((s) => ({ value: s, label: getStatusLabel(s, t) }))}
              value={statusFilter}
              onChange={setStatusFilter}
              style={{ minWidth: 120, flex: '1 1 120px' }}
            />
            <Select
              placeholder={t('project.list.countryPlaceholder')}
              clearable
              data={getCountries(i18n.language).map((c) => ({ value: c.code, label: `${c.code} ${c.name}` }))}
              value={countryFilter}
              onChange={setCountryFilter}
              style={{ minWidth: 120, flex: '1 1 140px' }}
            />
            <Select
              placeholder={t('project.list.partnerPlaceholder')}
              clearable
              data={partnerOptions()}
              value={partnerFilter}
              onChange={setPartnerFilter}
              searchable
              style={{ minWidth: 120, flex: '1 1 140px' }}
            />
            <Select
              placeholder={t('project.list.ownerPlaceholder')}
              clearable
              data={personOptions()}
              value={ownerFilter}
              onChange={setOwnerFilter}
              searchable
              style={{ minWidth: 120, flex: '1 1 140px' }}
            />
            <Select
              placeholder={t('project.list.memberPlaceholder')}
              clearable
              data={personOptions()}
              value={memberFilter}
              onChange={setMemberFilter}
              searchable
              style={{ minWidth: 120, flex: '1 1 140px' }}
            />
            <MultiSelect
              placeholder={t('project.list.tagPlaceholder')}
              clearable
              data={allTags.map((tag) => ({ value: tag, label: tag }))}
              value={tagFilter}
              onChange={setTagFilter}
              searchable
              style={{ minWidth: 140, flex: '1 1 160px' }}
            />
            <Button
              variant={showArchived ? 'filled' : 'light'}
              size="xs"
              onClick={() => setShowArchived((v) => !v)}
            >
              {showArchived ? t('project.list.hideArchived') : t('project.list.showArchived')}
            </Button>
          </Flex>
          <Flex gap="xs" align="center">
            <Text size="xs" c="dimmed">{t('project.list.sortBy')}</Text>
            <Select
              size="xs"
              data={[
                { value: 'updatedAt', label: t('project.list.sortUpdated') },
                { value: 'priority', label: t('project.list.sortPriority') },
                { value: 'dueDate', label: t('project.list.sortDueDate') },
              ]}
              value={sortBy}
              onChange={(v) => v && setSortBy(v as SortBy)}
              style={{ minWidth: 180 }}
            />
          </Flex>
        </Stack>
      </Paper>

      <Paper>
        {loading ? (
          <Flex justify="center" py="xl">
            <Loader size="sm" />
          </Flex>
        ) : items.length === 0 ? (
          <EmptyState
            icon={IconFolder}
            title={t('project.list.emptyTitle')}
            description={total === 0 && !statusFilter && !countryFilter && !partnerFilter && !ownerFilter && !memberFilter && tagFilter.length === 0 ? t('project.list.emptyDescAll') : t('project.list.emptyDescFiltered')}
            actionLabel={total === 0 && !statusFilter ? t('project.list.new') : undefined}
            onAction={total === 0 && !statusFilter ? () => navigate('/projects/new') : undefined}
          />
        ) : (
          <>
            <Table.ScrollContainer minWidth={700}>
              <Table striped highlightOnHover>
                <Table.Thead>
                  <Table.Tr>
                    <Table.Th>{t('project.list.colName')}</Table.Th>
                    <Table.Th>{t('project.list.colStatus')}</Table.Th>
                    <Table.Th>{t('project.list.colCountry')}</Table.Th>
                    <Table.Th>{t('project.list.colPartner')}</Table.Th>
                    <Table.Th>{t('project.list.colOwner')}</Table.Th>
                    <Table.Th>{t('project.list.colDueDate')}</Table.Th>
                    <Table.Th>{t('project.list.colTags')}</Table.Th>
                    <Table.Th>{t('project.list.colActions')}</Table.Th>
                  </Table.Tr>
                </Table.Thead>
                <Table.Tbody>
                  {items.map((p) => (
                    <Table.Tr key={p.id}>
                      <Table.Td>
                        <Text fw={500}>{p.name}</Text>
                      </Table.Td>
                      <Table.Td>
                        <Badge size="sm" color={getProjectStatusColor(p.current_status)}>
                          {getStatusLabel(p.current_status, t)}
                        </Badge>
                      </Table.Td>
                      <Table.Td>{p.country_code}</Table.Td>
                      <Table.Td>{p.partner_name}</Table.Td>
                      <Table.Td>{p.owner_name}</Table.Td>
                      <Table.Td>{p.due_date ?? '—'}</Table.Td>
                      <Table.Td>
                        {p.tags?.length ? p.tags.join(', ') : '—'}
                      </Table.Td>
                      <Table.Td>
                        <Button size="xs" variant="light" onClick={() => navigate(`/projects/${p.id}`)}>
                          {t('common.view')}
                        </Button>
                      </Table.Td>
                    </Table.Tr>
                  ))}
                </Table.Tbody>
              </Table>
            </Table.ScrollContainer>
            {totalPages > 1 && (
              <Group justify="space-between" mt="md" px="sm">
                <Text size="sm" c="dimmed">
                  {t('project.list.total', { count: total })}
                </Text>
                <Pagination
                  value={page}
                  onChange={setPage}
                  total={totalPages}
                  size="sm"
                />
              </Group>
            )}
            {totalPages <= 1 && total > 0 && (
              <Text size="sm" c="dimmed" ta="right" mt="xs" px="sm">
                {t('project.list.total', { count: total })}
              </Text>
            )}
          </>
        )}
      </Paper>
    </Stack>
  );
}
