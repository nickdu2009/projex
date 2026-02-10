import { Badge, Button, Flex, Loader, Paper, Stack, Table, Text, Title } from '@mantine/core';
import { IconArrowLeft, IconEdit } from '@tabler/icons-react';
import { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { peopleApi, type PersonDto, type PersonProjectItem } from '../api/people';
import { showError, showSuccess } from '../utils/errorToast';
import { getRoleLabel } from '../utils/roleLabel';
import { getProjectStatusColor, getStatusLabel } from '../utils/statusColor';
import { ConfirmModal } from '../components/ConfirmModal';

export function PersonDetail() {
  const { t } = useTranslation();
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
      showError((e as { message?: string })?.message ?? t('common.failedToLoad'));
    } finally {
      setLoading(false);
    }
  }, [id, t]);

  useEffect(() => {
    load();
  }, [load]);

  const handleDeactivate = async () => {
    if (!id) return;
    setDeactivating(true);
    try {
      await peopleApi.deactivate(id);
      showSuccess(t('person.detail.deactivated'));
      setDeactivateModal(false);
      load();
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('person.detail.deactivateFailed'));
    } finally {
      setDeactivating(false);
    }
  };

  if (!id) return <Text>{t('person.detail.missingId')}</Text>;
  if (loading || !person) return <Loader size="sm" />;

  return (
    <Stack gap="md" w="100%" pb="xl" style={{ minWidth: 0 }}>
      <Flex wrap="wrap" gap="xs" justify="space-between" align="center">
        <Button variant="subtle" leftSection={<IconArrowLeft size={16} />} onClick={() => navigate('/people')}>
          {t('common.backToList')}
        </Button>
        <Flex gap="xs">
          <Button variant="light" leftSection={<IconEdit size={16} />} onClick={() => navigate(`/people/${id}/edit`)}>
            {t('common.edit')}
          </Button>
          {person.is_active && (
            <Button variant="light" color="red" onClick={() => setDeactivateModal(true)}>
              {t('common.deactivate')}
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
              {person.is_active ? t('common.active') : t('common.inactive')}
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
              {t('person.detail.email', { value: person.email })}
            </Text>
          )}
          <Text size="sm" style={{ color: 'rgba(255,255,255,0.9)' }}>{person.note || '—'}</Text>
        </Stack>
      </Paper>

      <Paper>
        <Title order={5} mb="xs">{t('person.detail.currentProjects')}</Title>
        {currentProjects.length === 0 ? (
          <Text size="sm" c="dimmed">{t('common.none')}</Text>
        ) : (
          <Table.ScrollContainer minWidth={300}>
            <Table>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>{t('person.detail.colProject')}</Table.Th>
                  <Table.Th>{t('person.detail.colStatus')}</Table.Th>
                  <Table.Th>{t('person.detail.colActions')}</Table.Th>
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
                        {getStatusLabel(proj.current_status, t)}
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
        <Title order={5} mb="xs">{t('person.detail.projectHistory')}</Title>
        {allProjects.length === 0 ? (
          <Text size="sm" c="dimmed">{t('common.none')}</Text>
        ) : (
          <Table.ScrollContainer minWidth={400}>
            <Table>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>{t('person.detail.colProject')}</Table.Th>
                  <Table.Th>{t('person.detail.colStatus')}</Table.Th>
                  <Table.Th>{t('person.detail.colLastInvolved')}</Table.Th>
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
                        {getStatusLabel(proj.current_status, t)}
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
        title={t('person.detail.deactivateTitle')}
        message={t('person.detail.deactivateMessage', { name: person.display_name })}
        confirmLabel={t('common.deactivate')}
        loading={deactivating}
      />
    </Stack>
  );
}
