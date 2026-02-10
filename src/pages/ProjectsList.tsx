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
import { projectApi, type ProjectListItem, type ProjectListReq } from '../api/projects';
import { COUNTRIES, PROJECT_STATUSES } from '../constants/countries';
import { showError } from '../utils/errorToast';
import { usePartnerStore } from '../stores/usePartnerStore';
import { usePersonStore } from '../stores/usePersonStore';
import { useTagStore } from '../stores/useTagStore';
import { getProjectStatusColor } from '../utils/statusColor';
import { EmptyState } from '../components/EmptyState';

type SortBy = 'updatedAt' | 'priority' | 'dueDate';

const PAGE_SIZE = 50;

export function ProjectsList() {
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
      showError((e as { message?: string })?.message ?? '加载失败');
    } finally {
      setLoading(false);
    }
  }, [showArchived, page, sortBy, statusFilter, countryFilter, partnerFilter, ownerFilter, memberFilter, tagFilter]);

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
        <Title order={3}>项目列表</Title>
        <Button
          variant="gradient"
          gradient={{ from: 'indigo', to: 'violet' }}
          leftSection={<IconPlus size={18} />}
          onClick={() => navigate('/projects/new')}
        >
          新建项目
        </Button>
      </Flex>

      <Paper>
        <Stack gap="xs">
          <Text size="xs" c="dimmed" fw={500}>筛选条件</Text>
          <Flex wrap="wrap" gap="xs" align="flex-end">
            <Select
              placeholder="状态"
              clearable
              data={PROJECT_STATUSES.map((s) => ({ value: s, label: s }))}
              value={statusFilter}
              onChange={setStatusFilter}
              style={{ minWidth: 120, flex: '1 1 120px' }}
            />
            <Select
              placeholder="国家"
              clearable
              data={COUNTRIES.map((c) => ({ value: c.code, label: `${c.code} ${c.name}` }))}
              value={countryFilter}
              onChange={setCountryFilter}
              style={{ minWidth: 120, flex: '1 1 140px' }}
            />
            <Select
              placeholder="合作方"
              clearable
              data={partnerOptions()}
              value={partnerFilter}
              onChange={setPartnerFilter}
              searchable
              style={{ minWidth: 120, flex: '1 1 140px' }}
            />
            <Select
              placeholder="负责人"
              clearable
              data={personOptions()}
              value={ownerFilter}
              onChange={setOwnerFilter}
              searchable
              style={{ minWidth: 120, flex: '1 1 140px' }}
            />
            <Select
              placeholder="参与成员"
              clearable
              data={personOptions()}
              value={memberFilter}
              onChange={setMemberFilter}
              searchable
              style={{ minWidth: 120, flex: '1 1 140px' }}
            />
            <MultiSelect
              placeholder="标签"
              clearable
              data={allTags.map((t) => ({ value: t, label: t }))}
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
              {showArchived ? '隐藏已归档' : '显示已归档'}
            </Button>
          </Flex>
          <Flex gap="xs" align="center">
            <Text size="xs" c="dimmed">排序方式：</Text>
            <Select
              size="xs"
              data={[
                { value: 'updatedAt', label: '更新时间（最新优先）' },
                { value: 'priority', label: '优先级（高优先级在前）' },
                { value: 'dueDate', label: '截止日期（最近优先）' },
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
            title="暂无项目"
            description={total === 0 && !statusFilter && !countryFilter && !partnerFilter && !ownerFilter && !memberFilter && tagFilter.length === 0 ? "还没有创建任何项目。请先创建合作方和成员，然后新建项目。" : "没有符合筛选条件的项目"}
            actionLabel={total === 0 && !statusFilter ? "新建项目" : undefined}
            onAction={total === 0 && !statusFilter ? () => navigate('/projects/new') : undefined}
          />
        ) : (
          <>
            <Table.ScrollContainer minWidth={700}>
              <Table striped highlightOnHover>
                <Table.Thead>
                  <Table.Tr>
                    <Table.Th>名称</Table.Th>
                    <Table.Th>状态</Table.Th>
                    <Table.Th>国家</Table.Th>
                    <Table.Th>合作方</Table.Th>
                    <Table.Th>负责人</Table.Th>
                    <Table.Th>截止日</Table.Th>
                    <Table.Th>标签</Table.Th>
                    <Table.Th>操作</Table.Th>
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
                          {p.current_status}
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
                          详情
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
                  共 {total} 个项目
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
                共 {total} 个项目
              </Text>
            )}
          </>
        )}
      </Paper>
    </Stack>
  );
}
