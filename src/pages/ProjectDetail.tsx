import {
  Badge,
  Button,
  Flex,
  Loader,
  Modal,
  Paper,
  Select,
  SimpleGrid,
  Stack,
  Table,
  Text,
  Textarea,
  Title,
} from '@mantine/core';
import { IconArrowLeft, IconEdit, IconPlus } from '@tabler/icons-react';
import { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { assignmentApi } from '../api/assignments';
import { peopleApi } from '../api/people';
import { projectApi, type ProjectDetail as ProjectDetailType } from '../api/projects';
import { PROJECT_STATUSES } from '../constants/countries';
import { showError, showSuccess } from '../utils/errorToast';
import { getProjectStatusColor, getStatusLabel } from '../utils/statusColor';

const NOTE_REQUIRED_TRANSITIONS = [
  'ARCHIVED->BACKLOG',
  'DONE->IN_PROGRESS',
  'BACKLOG->ARCHIVED',
  'PLANNED->ARCHIVED',
];

function needsNote(from: string | null, to: string): boolean {
  if (!from) return false;
  const key = `${from}->${to}`;
  return NOTE_REQUIRED_TRANSITIONS.includes(key);
}

export function ProjectDetail() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [project, setProject] = useState<ProjectDetailType | null>(null);
  const [loading, setLoading] = useState(true);
  const [statusModal, setStatusModal] = useState<{ to: string; note: string } | null>(null);
  const [personOptions, setPersonOptions] = useState<{ value: string; label: string }[]>([]);
  const [addPersonId, setAddPersonId] = useState<string | null>(null);
  const [ownerId, setOwnerId] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    try {
      const p = await projectApi.get(id);
      setProject(p);
      setOwnerId(p.owner_person_id);
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('common.failedToLoad'));
    } finally {
      setLoading(false);
    }
  }, [id, t]);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    peopleApi.list(true).then((ps) => {
      setPersonOptions(ps.map((p) => ({ value: p.id, label: p.display_name })));
    }).catch(() => {});
  }, []);

  const handleChangeStatus = async () => {
    if (!id || !statusModal) return;
    const { to, note } = statusModal;
    if (needsNote(project?.current_status ?? null, to) && !note.trim()) {
      showError(t('project.detail.noteRequiredError'));
      return;
    }
    try {
      await projectApi.changeStatus({
        projectId: id,
        toStatus: to,
        note: note.trim() || undefined,
      });
      setStatusModal(null);
      load();
      showSuccess(t('project.detail.statusUpdated'));
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('project.detail.statusUpdateFailed'));
    }
  };

  const handleAddMember = async () => {
    if (!id || !addPersonId) return;
    try {
      await assignmentApi.addMember({ projectId: id, personId: addPersonId });
      setAddPersonId(null);
      load();
      showSuccess(t('project.detail.memberAdded'));
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('project.detail.memberAddFailed'));
    }
  };

  const handleEndMember = async (personId: string) => {
    if (!id) return;
    try {
      await assignmentApi.endMember({ projectId: id, personId });
      load();
      showSuccess(t('project.detail.memberRemoved'));
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('project.detail.operationFailed'));
    }
  };

  const handleSaveOwner = async () => {
    if (!id || !ownerId || !project) return;
    if (ownerId === project.owner_person_id) return;
    try {
      await projectApi.update({ id, ownerPersonId: ownerId });
      load();
      showSuccess(t('project.detail.ownerUpdated'));
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('project.detail.ownerUpdateFailed'));
    }
  };

  if (!id) return <Text>{t('project.detail.missingId')}</Text>;
  if (loading || !project) {
    return <Loader size="sm" />;
  }

  const activeAssignments = project.assignments.filter((a) => !a.end_at);
  const canTransitionTo = PROJECT_STATUSES.filter((s) => {
    if (s === project.current_status) return false;
    if (s === 'ARCHIVED' && project.current_status !== 'BACKLOG' && project.current_status !== 'PLANNED' && project.current_status !== 'DONE') return false;
    if (project.current_status === 'ARCHIVED' && s !== 'BACKLOG') return false;
    if (project.current_status === 'BACKLOG' && s !== 'PLANNED' && s !== 'ARCHIVED') return false;
    if (project.current_status === 'PLANNED' && s !== 'IN_PROGRESS' && s !== 'ARCHIVED') return false;
    if (project.current_status === 'IN_PROGRESS' && s !== 'BLOCKED' && s !== 'DONE') return false;
    if (project.current_status === 'BLOCKED' && s !== 'IN_PROGRESS') return false;
    if (project.current_status === 'DONE' && s !== 'ARCHIVED' && s !== 'IN_PROGRESS') return false;
    return true;
  });

  return (
    <Stack gap="md" w="100%" pb="xl" style={{ minWidth: 0 }}>
      <Flex wrap="wrap" gap="xs" justify="space-between" align="center">
        <Button variant="subtle" leftSection={<IconArrowLeft size={16} />} onClick={() => navigate('/projects')}>
          {t('common.backToList')}
        </Button>
        <Button variant="light" leftSection={<IconEdit size={16} />} onClick={() => navigate(`/projects/${id}/edit`)}>
          {t('common.edit')}
        </Button>
      </Flex>

      <Paper
        style={{
          background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
          color: 'white',
        }}
      >
        <Stack gap="md">
          <Flex wrap="wrap" align="center" gap="xs">
            <Title order={2} style={{ color: 'white' }}>{project.name}</Title>
            <Badge
              size="lg"
              color={getProjectStatusColor(project.current_status)}
              variant="filled"
              style={{ backgroundColor: 'rgba(255,255,255,0.25)' }}
            >
              {getStatusLabel(project.current_status, t)}
            </Badge>
          </Flex>
          <Text size="sm" style={{ color: 'rgba(255,255,255,0.9)' }}>{project.description || '—'}</Text>
          <SimpleGrid cols={{ base: 1, sm: 2 }} spacing="xs" verticalSpacing="xs">
            <Text size="sm" style={{ color: 'white' }}>{t('project.detail.country', { value: project.country_code })}</Text>
            <Text size="sm" style={{ color: 'white' }}>{t('project.detail.partner', { value: project.partner_name })}</Text>
          </SimpleGrid>
          <SimpleGrid cols={{ base: 1, sm: 3 }} spacing="xs" verticalSpacing="xs">
            <Text size="sm" style={{ color: 'white' }}>{t('project.detail.startDate', { value: project.start_date ?? '—' })}</Text>
            <Text size="sm" style={{ color: 'white' }}>{t('project.detail.dueDate', { value: project.due_date ?? '—' })}</Text>
            <Text size="sm" style={{ color: 'white' }}>{t('project.detail.tags', { value: project.tags?.length ? project.tags.join(', ') : '—' })}</Text>
          </SimpleGrid>
        </Stack>
      </Paper>

      <Paper>
        <Stack gap="sm">
          <Flex wrap="wrap" gap="xs" justify="space-between" align="center">
            <div>
              <Text size="sm" c="dimmed" mb={4}>{t('project.detail.owner')}</Text>
              <Flex gap="xs" align="center">
                <Select
                  size="xs"
                  style={{ minWidth: 120, flex: 1 }}
                  data={personOptions}
                  value={ownerId}
                  onChange={setOwnerId}
                  searchable
                />
                <Button size="xs" onClick={handleSaveOwner}>{t('common.save')}</Button>
              </Flex>
            </div>
            <Button
              size="sm"
              variant="light"
              onClick={() => setStatusModal({ to: '', note: '' })}
              disabled={canTransitionTo.length === 0}
            >
              {t('project.detail.changeStatus')}
            </Button>
          </Flex>
        </Stack>
      </Paper>

      <Paper>
        <Title order={5} mb="xs">{t('project.detail.members')}</Title>
        <Stack gap="xs">
          <Flex wrap="wrap" gap="xs" align="flex-end">
            <Select
              size="xs"
              style={{ minWidth: 140, flex: '1 1 140px' }}
              data={personOptions.filter((o) => !activeAssignments.some((a) => a.person_id === o.value))}
              value={addPersonId}
              onChange={setAddPersonId}
              placeholder={t('project.detail.selectMember')}
              searchable
            />
            <Button size="xs" leftSection={<IconPlus size={14} />} onClick={handleAddMember} disabled={!addPersonId}>{t('project.detail.addMember')}</Button>
          </Flex>
          <Table.ScrollContainer minWidth={400}>
          <Table>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>{t('project.detail.colName')}</Table.Th>
                <Table.Th>{t('project.detail.colRole')}</Table.Th>
                <Table.Th>{t('project.detail.colStart')}</Table.Th>
                <Table.Th>{t('project.detail.colEnd')}</Table.Th>
                <Table.Th>{t('project.detail.colActions')}</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {project.assignments.map((a) => (
                <Table.Tr key={a.id}>
                  <Table.Td>{a.person_name}</Table.Td>
                  <Table.Td>{a.role}</Table.Td>
                  <Table.Td>{a.start_at}</Table.Td>
                  <Table.Td>{a.end_at ?? '—'}</Table.Td>
                  <Table.Td>
                    {!a.end_at && (
                      <Button
                        size="xs"
                        color="red"
                        variant="light"
                        onClick={() => handleEndMember(a.person_id)}
                      >
                        {t('project.detail.removeMember')}
                      </Button>
                    )}
                  </Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
          </Table.ScrollContainer>
        </Stack>
      </Paper>

      <Paper>
        <Title order={5} mb="xs">{t('project.detail.statusTimeline')}</Title>
        <Table.ScrollContainer minWidth={500}>
        <Table>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>{t('project.detail.colTime')}</Table.Th>
              <Table.Th>{t('project.detail.colChange')}</Table.Th>
              <Table.Th>{t('project.detail.colChangedBy')}</Table.Th>
              <Table.Th>{t('project.detail.colNote')}</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {project.status_history.map((h) => (
              <Table.Tr key={h.id}>
                <Table.Td>{h.changed_at}</Table.Td>
                <Table.Td>{h.from_status ? getStatusLabel(h.from_status, t) : '—'} → {getStatusLabel(h.to_status, t)}</Table.Td>
                <Table.Td>{h.changed_by_name ?? '—'}</Table.Td>
                <Table.Td>{h.note || '—'}</Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
        </Table.ScrollContainer>
      </Paper>

      <Modal
        opened={!!statusModal}
        onClose={() => setStatusModal(null)}
        title={t('project.detail.statusModalTitle')}
      >
        <Stack>
          <Select
            label={t('project.detail.targetStatus')}
            data={canTransitionTo.map((s) => ({ value: s, label: getStatusLabel(s, t) }))}
            value={statusModal?.to ?? null}
            onChange={(v) => v && setStatusModal((m) => ({ ...(m ?? { to: '', note: '' }), to: v }))}
          />
          <Textarea
            label={t('project.detail.colNote')}
            placeholder={needsNote(project.current_status, statusModal?.to ?? '') ? t('project.detail.noteRequired') : t('common.optional')}
            value={statusModal?.note ?? ''}
            onChange={(e) => setStatusModal((m) => m ? { ...m, note: e.target.value } : null)}
          />
          <Button onClick={handleChangeStatus} disabled={!statusModal?.to}>{t('common.confirm')}</Button>
        </Stack>
      </Modal>
    </Stack>
  );
}
