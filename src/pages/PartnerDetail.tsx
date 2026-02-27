import { Badge, Button, Card, Flex, Group, Loader, Paper, Stack, Table, Text, Title } from '@mantine/core';
import { IconArrowLeft, IconEdit } from '@tabler/icons-react';
import { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useIsMobile } from '../utils/useIsMobile';
import { partnersApi, type PartnerDto, type PartnerProjectItem } from '../api/partners';
import { showError, showSuccess } from '../utils/errorToast';
import { getProjectStatusColor, getStatusLabel } from '../utils/statusColor';
import { ConfirmModal } from '../components/ConfirmModal';

export function PartnerDetail() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const isMobile = useIsMobile();
  const [partner, setPartner] = useState<PartnerDto | null>(null);
  const [projects, setProjects] = useState<PartnerProjectItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [deactivating, setDeactivating] = useState(false);

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
      await partnersApi.deactivate(id);
      showSuccess(t('partner.detail.deactivated'));
      setConfirmOpen(false);
      load();
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('partner.detail.deactivateFailed'));
    } finally {
      setDeactivating(false);
    }
  };

  if (!id) return <Text>{t('partner.detail.missingId')}</Text>;
  if (loading || !partner) return <Loader size="sm" />;

  return (
    <Stack gap="md" w="100%" pb="xl" style={{ minWidth: 0 }}>
      <Flex wrap="wrap" gap="xs" justify="space-between" align="center">
        <Button variant="subtle" leftSection={<IconArrowLeft size={16} />} onClick={() => navigate('/partners')}>
          {t('common.backToList')}
        </Button>
        <Group gap="xs" wrap="wrap">
          <Button variant="light" leftSection={<IconEdit size={16} />} onClick={() => navigate(`/partners/${id}/edit`)}>
            {t('common.edit')}
          </Button>
          {partner.is_active && (
            <Button variant="light" color="red" onClick={() => setConfirmOpen(true)}>
              {t('common.deactivate')}
            </Button>
          )}
        </Group>
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
              {partner.is_active ? t('common.active') : t('common.inactive')}
            </Badge>
          </Flex>
          <Text size="sm" style={{ color: 'rgba(255,255,255,0.9)' }}>{partner.note || '—'}</Text>
        </Stack>
      </Paper>

      <Paper>
        <Title order={5} mb="xs">{t('partner.detail.projects')}</Title>
        {projects.length === 0 ? (
          <Text size="sm" c="dimmed">{t('common.none')}</Text>
        ) : isMobile ? (
          <Stack gap="xs">
            {projects.map((proj) => (
              <Card key={proj.id} padding="xs" radius="sm" withBorder style={{ cursor: 'pointer' }} onClick={() => navigate(`/projects/${proj.id}`)}>
                <Group justify="space-between" wrap="nowrap" gap="xs">
                  <Text size="sm" fw={500} style={{ minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{proj.name}</Text>
                  <Badge size="xs" color={getProjectStatusColor(proj.current_status)} style={{ flexShrink: 0 }}>{getStatusLabel(proj.current_status, t)}</Badge>
                </Group>
              </Card>
            ))}
          </Stack>
        ) : (
          <Table.ScrollContainer minWidth={300}>
            <Table>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>{t('partner.detail.colProject')}</Table.Th>
                  <Table.Th>{t('partner.detail.colStatus')}</Table.Th>
                  <Table.Th>{t('partner.detail.colActions')}</Table.Th>
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
      <ConfirmModal
        opened={confirmOpen}
        onClose={() => setConfirmOpen(false)}
        onConfirm={handleDeactivate}
        title={t('partner.detail.deactivateTitle')}
        message={t('partner.detail.deactivateMessage', { name: partner.name })}
        confirmLabel={t('common.deactivate')}
        confirmColor="red"
        loading={deactivating}
      />
    </Stack>
  );
}
