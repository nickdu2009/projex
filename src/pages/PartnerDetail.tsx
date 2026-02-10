import { Badge, Button, Flex, Loader, Paper, Stack, Table, Text, Title } from '@mantine/core';
import { IconArrowLeft, IconEdit } from '@tabler/icons-react';
import { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { partnersApi, type PartnerDto, type PartnerProjectItem } from '../api/partners';
import { showError, showSuccess } from '../utils/errorToast';
import { getProjectStatusColor } from '../utils/statusColor';

export function PartnerDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [partner, setPartner] = useState<PartnerDto | null>(null);
  const [projects, setProjects] = useState<PartnerProjectItem[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    try {
      const [p, projs] = await Promise.all([
        partnersApi.get(id),
        partnersApi.projects(id),
      ]);
      setPartner(p);
      setProjects(projs);
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? '加载失败');
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => {
    load();
  }, [load]);

  const handleDeactivate = async () => {
    if (!id || !partner?.is_active) return;
    if (!window.confirm(`确定停用「${partner.name}」？`)) return;
    try {
      await partnersApi.deactivate(id);
      showSuccess('已停用');
      load();
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? '停用失败');
    }
  };

  if (!id) return <Text>缺少合作方 ID</Text>;
  if (loading || !partner) return <Loader size="sm" />;

  return (
    <Stack gap="md" w="100%" pb="xl" style={{ minWidth: 0 }}>
      <Flex wrap="wrap" gap="xs" justify="space-between" align="center">
        <Button variant="subtle" leftSection={<IconArrowLeft size={16} />} onClick={() => navigate('/partners')}>
          返回列表
        </Button>
        <Flex gap="xs">
          <Button variant="light" leftSection={<IconEdit size={16} />} onClick={() => navigate(`/partners/${id}/edit`)}>
            编辑
          </Button>
          {partner.is_active && (
            <Button variant="light" color="red" onClick={handleDeactivate}>
              停用
            </Button>
          )}
        </Flex>
      </Flex>

      <Paper
        style={{
          background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
          color: 'white',
        }}
      >
        <Stack gap="xs">
          <Flex wrap="wrap" align="center" gap="xs">
            <Title order={2} style={{ color: 'white' }}>{partner.name}</Title>
            <Badge
              size="lg"
              color={partner.is_active ? 'green' : 'gray'}
              variant="filled"
              style={{ backgroundColor: 'rgba(255,255,255,0.25)' }}
            >
              {partner.is_active ? '启用' : '停用'}
            </Badge>
          </Flex>
          <Text size="sm" style={{ color: 'rgba(255,255,255,0.9)' }}>{partner.note || '—'}</Text>
        </Stack>
      </Paper>

      <Paper>
        <Title order={5} mb="xs">关联项目</Title>
        {projects.length === 0 ? (
          <Text size="sm" c="dimmed">暂无</Text>
        ) : (
          <Table.ScrollContainer minWidth={300}>
            <Table>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>项目</Table.Th>
                  <Table.Th>状态</Table.Th>
                  <Table.Th>操作</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {projects.map((proj) => (
                  <Table.Tr key={proj.id}>
                    <Table.Td>
                      <Button variant="subtle" size="xs" onClick={() => navigate(`/projects/${proj.id}`)}>
                        {proj.name}
                      </Button>
                    </Table.Td>
                    <Table.Td>
                      <Badge size="sm" color={getProjectStatusColor(proj.current_status)}>
                        {proj.current_status}
                      </Badge>
                    </Table.Td>
                    <Table.Td>—</Table.Td>
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
