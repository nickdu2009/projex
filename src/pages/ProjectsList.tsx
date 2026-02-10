import {
  Badge,
  Button,
  Flex,
  Loader,
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
import { projectApi, type ProjectListItem } from '../api/projects';
import { COUNTRIES, PROJECT_STATUSES } from '../constants/countries';
import { showError } from '../utils/errorToast';
import { partnersApi } from '../api/partners';
import { peopleApi } from '../api/people';
import { getProjectStatusColor } from '../utils/statusColor';
import { EmptyState } from '../components/EmptyState';

type SortBy = 'updated_at' | 'priority' | 'due_date';

export function ProjectsList() {
  const navigate = useNavigate();
  const [items, setItems] = useState<ProjectListItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [statusFilter, setStatusFilter] = useState<string | null>(null);
  const [countryFilter, setCountryFilter] = useState<string | null>(null);
  const [partnerOptions, setPartnerOptions] = useState<{ value: string; label: string }[]>([]);
  const [personOptions, setPersonOptions] = useState<{ value: string; label: string }[]>([]);
  const [partnerFilter, setPartnerFilter] = useState<string | null>(null);
  const [ownerFilter, setOwnerFilter] = useState<string | null>(null);
  const [memberFilter, setMemberFilter] = useState<string | null>(null);
  const [memberProjectIds, setMemberProjectIds] = useState<string[]>([]);
  const [showArchived, setShowArchived] = useState(false);
  const [sortBy, setSortBy] = useState<SortBy>('updated_at');

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const list = await projectApi.list({
        onlyUnarchived: !showArchived,
      });
      setItems(list);
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? '加载失败');
    } finally {
      setLoading(false);
    }
  }, [showArchived]);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    partnersApi.list(true).then((ps) => {
      setPartnerOptions(ps.map((p) => ({ value: p.id, label: p.name })));
    }).catch(() => {});
    peopleApi.list(true).then((ps) => {
      setPersonOptions(ps.map((p) => ({ value: p.id, label: p.display_name })));
    }).catch(() => {});
  }, []);

  useEffect(() => {
    if (memberFilter) {
      peopleApi.allProjects(memberFilter).then((projects) => {
        setMemberProjectIds(projects.map((p) => p.id));
      }).catch(() => {
        setMemberProjectIds([]);
      });
    } else {
      setMemberProjectIds([]);
    }
  }, [memberFilter]);

  const filtered = items
    .filter((p) => {
      if (statusFilter && p.current_status !== statusFilter) return false;
      if (countryFilter && p.country_code !== countryFilter) return false;
      if (partnerFilter && p.partner_name !== partnerOptions.find((o) => o.value === partnerFilter)?.label) return false;
      if (ownerFilter && p.owner_name !== personOptions.find((o) => o.value === ownerFilter)?.label) return false;
      if (memberFilter && memberProjectIds.length > 0 && !memberProjectIds.includes(p.id)) return false;
      return true;
    })
    .sort((a, b) => {
      if (sortBy === 'updated_at') {
        // 按更新时间降序（最新的在前）
        return new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime();
      } else if (sortBy === 'priority') {
        // 按优先级升序（数字越小优先级越高）
        return a.priority - b.priority;
      } else if (sortBy === 'due_date') {
        // 按截止日升序，null 值排在最后
        if (!a.due_date && !b.due_date) return 0;
        if (!a.due_date) return 1;
        if (!b.due_date) return -1;
        return new Date(a.due_date).getTime() - new Date(b.due_date).getTime();
      }
      return 0;
    });

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
              data={partnerOptions}
              value={partnerFilter}
              onChange={setPartnerFilter}
              searchable
              style={{ minWidth: 120, flex: '1 1 140px' }}
            />
            <Select
              placeholder="负责人"
              clearable
              data={personOptions}
              value={ownerFilter}
              onChange={setOwnerFilter}
              searchable
              style={{ minWidth: 120, flex: '1 1 140px' }}
            />
            <Select
              placeholder="参与成员"
              clearable
              data={personOptions}
              value={memberFilter}
              onChange={setMemberFilter}
              searchable
              style={{ minWidth: 120, flex: '1 1 140px' }}
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
                { value: 'updated_at', label: '更新时间（最新优先）' },
                { value: 'priority', label: '优先级（高优先级在前）' },
                { value: 'due_date', label: '截止日期（最近优先）' },
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
        ) : filtered.length === 0 ? (
          <EmptyState
            icon={IconFolder}
            title="暂无项目"
            description={items.length === 0 ? "还没有创建任何项目。请先创建合作方和成员，然后新建项目。" : "没有符合筛选条件的项目"}
            actionLabel={items.length === 0 ? "新建项目" : undefined}
            onAction={items.length === 0 ? () => navigate('/projects/new') : undefined}
          />
        ) : (
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
                {filtered.map((p) => (
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
        )}
      </Paper>
    </Stack>
  );
}
