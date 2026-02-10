import { Badge, Button, Flex, Loader, Paper, Stack, Table, Text, Title } from '@mantine/core';
import { IconArrowLeft, IconEdit } from '@tabler/icons-react';
import { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { peopleApi, type PersonDto, type PersonProjectItem } from '../api/people';
import { showError, showSuccess } from '../utils/errorToast';
import { getRoleLabel } from '../utils/roleLabel';
import { getProjectStatusColor } from '../utils/statusColor';
import { ConfirmModal } from '../components/ConfirmModal';

export function PersonDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [person, setPerson] = useState<PersonDto | null>(null);
  const [currentProjects, setCurrentProjects] = useState<PersonProjectItem[]>([]);
  const [allProjects, setAllProjects] = useState<PersonProjectItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [deactivateModal, setDeactivateModal] = useState(false);
  const [deactivating, setDeactivating] = useState(false);

  const load = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    try {
      const [p, current, all] = await Promise.all([
        peopleApi.get(id),
        peopleApi.currentProjects(id),
        peopleApi.allProjects(id),
      ]);
      setPerson(p);
      setCurrentProjects(current);
      setAllProjects(all);
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
    if (!id) return;
    setDeactivating(true);
    try {
      await peopleApi.deactivate(id);
      showSuccess('已停用');
      setDeactivateModal(false);
      load();
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? '停用失败');
    } finally {
      setDeactivating(false);
    }
  };

  if (!id) return <Text>缺少成员 ID</Text>;
  if (loading || !person) return <Loader size="sm" />;

  return (
    <Stack gap="md" w="100%" pb="xl" style={{ minWidth: 0 }}>
      <Flex wrap="wrap" gap="xs" justify="space-between" align="center">
        <Button variant="subtle" leftSection={<IconArrowLeft size={16} />} onClick={() => navigate('/people')}>
          返回列表
        </Button>
        <Flex gap="xs">
          <Button variant="light" leftSection={<IconEdit size={16} />} onClick={() => navigate(`/people/${id}/edit`)}>
            编辑
          </Button>
          {person.is_active && (
            <Button variant="light" color="red" onClick={() => setDeactivateModal(true)}>
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
            <Title order={2} style={{ color: 'white' }}>{person.display_name}</Title>
            <Badge
              size="lg"
              color={person.is_active ? 'green' : 'gray'}
              variant="filled"
              style={{ backgroundColor: 'rgba(255,255,255,0.25)' }}
            >
              {person.is_active ? '启用' : '停用'}
            </Badge>
            {person.role && (
              <Badge
                size="lg"
                variant="filled"
                style={{ backgroundColor: 'rgba(255,255,255,0.2)' }}
              >
                {getRoleLabel(person.role)}
              </Badge>
            )}
          </Flex>
          {person.email && (
            <Text size="sm" style={{ color: 'rgba(255,255,255,0.9)' }}>
              邮箱：{person.email}
            </Text>
          )}
          <Text size="sm" style={{ color: 'rgba(255,255,255,0.9)' }}>{person.note || '—'}</Text>
        </Stack>
      </Paper>

      <Paper>
        <Title order={5} mb="xs">当前参与项目</Title>
        {currentProjects.length === 0 ? (
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
                {currentProjects.map((proj) => (
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

      <Paper>
        <Title order={5} mb="xs">参与过的项目</Title>
        {allProjects.length === 0 ? (
          <Text size="sm" c="dimmed">暂无</Text>
        ) : (
          <Table.ScrollContainer minWidth={400}>
            <Table>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>项目</Table.Th>
                  <Table.Th>状态</Table.Th>
                  <Table.Th>最近参与</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {allProjects.map((proj) => (
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
                    <Table.Td>{proj.last_involved_at ?? '—'}</Table.Td>
                  </Table.Tr>
                ))}
              </Table.Tbody>
            </Table>
          </Table.ScrollContainer>
        )}
      </Paper>

      <ConfirmModal
        opened={deactivateModal}
        onClose={() => setDeactivateModal(false)}
        onConfirm={handleDeactivate}
        title="停用成员"
        message={`确定停用「${person.display_name}」？停用后该成员将无法被分配到新项目。`}
        confirmLabel="停用"
        loading={deactivating}
      />
    </Stack>
  );
}
