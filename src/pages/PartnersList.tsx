import { Button, Flex, Loader, Paper, Stack, Table, Text, Title } from '@mantine/core';
import { IconBuildingCommunity, IconPlus } from '@tabler/icons-react';
import { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { partnersApi, type PartnerDto } from '../api/partners';
import { showError } from '../utils/errorToast';
import { EmptyState } from '../components/EmptyState';

export function PartnersList() {
  const navigate = useNavigate();
  const [list, setList] = useState<PartnerDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [showInactive, setShowInactive] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await partnersApi.list(!showInactive);
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
        <Title order={3}>合作方列表</Title>
        <Button
          variant="gradient"
          gradient={{ from: 'indigo', to: 'violet' }}
          leftSection={<IconPlus size={18} />}
          onClick={() => navigate('/partners/new')}
        >
          新建合作方
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
            icon={IconBuildingCommunity}
            title="暂无合作方"
            description="还没有添加任何合作方。创建合作方后才能新建项目。"
            actionLabel="新建合作方"
            onAction={() => navigate('/partners/new')}
          />
        ) : (
          <Table.ScrollContainer minWidth={400}>
            <Table>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>名称</Table.Th>
                  <Table.Th>备注</Table.Th>
                  <Table.Th>状态</Table.Th>
                  <Table.Th>操作</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {list.map((p) => (
                  <Table.Tr key={p.id}>
                    <Table.Td>{p.name}</Table.Td>
                    <Table.Td><Text size="sm" c="dimmed">{p.note || '—'}</Text></Table.Td>
                    <Table.Td>{p.is_active ? '启用' : '停用'}</Table.Td>
                    <Table.Td>
                      <Button variant="subtle" size="xs" onClick={() => navigate(`/partners/${p.id}`)}>
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
