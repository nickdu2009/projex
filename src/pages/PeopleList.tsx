import { Button, Flex, Loader, Paper, Stack, Table, Text, Title } from '@mantine/core';
import { IconPlus, IconUsers } from '@tabler/icons-react';
import { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { peopleApi, type PersonDto } from '../api/people';
import { showError } from '../utils/errorToast';
import { getRoleLabel } from '../utils/roleLabel';
import { EmptyState } from '../components/EmptyState';

export function PeopleList() {
  const navigate = useNavigate();
  const [list, setList] = useState<PersonDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [showInactive, setShowInactive] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await peopleApi.list(!showInactive);
      setList(data);
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? '加载失败');
    } finally {
      setLoading(false);
    }
  }, [showInactive]);

  useEffect(() => {
    load();
  }, [load]);

  return (
    <Stack gap="md" w="100%" pb="xl" style={{ minWidth: 0 }}>
      <Flex wrap="wrap" gap="sm" justify="space-between" align="center">
        <Title order={3}>成员列表</Title>
        <Button
          variant="gradient"
          gradient={{ from: 'indigo', to: 'violet' }}
          leftSection={<IconPlus size={18} />}
          onClick={() => navigate('/people/new')}
        >
          新建成员
        </Button>
      </Flex>

      <Paper>
        <Button variant="subtle" size="xs" onClick={() => setShowInactive((v) => !v)}>
          {showInactive ? '仅显示启用' : '显示已停用'}
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
            title="暂无成员"
            description="还没有添加任何团队成员。创建成员后可以将其分配到项目中。"
            actionLabel="新建成员"
            onAction={() => navigate('/people/new')}
          />
        ) : (
          <Table.ScrollContainer minWidth={600}>
            <Table>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>姓名</Table.Th>
                  <Table.Th>邮箱</Table.Th>
                  <Table.Th>角色</Table.Th>
                  <Table.Th>备注</Table.Th>
                  <Table.Th>状态</Table.Th>
                  <Table.Th>操作</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {list.map((p) => (
                  <Table.Tr key={p.id}>
                    <Table.Td>{p.display_name}</Table.Td>
                    <Table.Td><Text size="sm" c="dimmed">{p.email || '—'}</Text></Table.Td>
                    <Table.Td><Text size="sm">{p.role ? getRoleLabel(p.role) : '—'}</Text></Table.Td>
                    <Table.Td><Text size="sm" c="dimmed">{p.note || '—'}</Text></Table.Td>
                    <Table.Td>{p.is_active ? '启用' : '停用'}</Table.Td>
                    <Table.Td>
                      <Button variant="subtle" size="xs" onClick={() => navigate(`/people/${p.id}`)}>
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
