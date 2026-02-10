import { Button, Flex, Loader, Paper, Stack, Table, Text, Title } from '@mantine/core';
import { IconBuildingCommunity, IconPlus } from '@tabler/icons-react';
import { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { partnersApi, type PartnerDto } from '../api/partners';
import { showError } from '../utils/errorToast';
import { EmptyState } from '../components/EmptyState';

export function PartnersList() {
  const { t } = useTranslation();
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
      showError((e as { message?: string })?.message ?? t('common.failedToLoad'));
    } finally {
      setLoading(false);
    }
  }, [showInactive, t]);

  useEffect(() => {
    load();
  }, [load]);

  return (
    <Stack gap="md" w="100%" pb="xl" style={{ minWidth: 0 }}>
      <Flex wrap="wrap" gap="sm" justify="space-between" align="center">
        <Title order={3}>{t('partner.list.title')}</Title>
        <Button
          variant="gradient"
          gradient={{ from: 'indigo', to: 'violet' }}
          leftSection={<IconPlus size={18} />}
          onClick={() => navigate('/partners/new')}
        >
          {t('partner.list.new')}
        </Button>
      </Flex>

      <Paper>
        <Button variant="subtle" size="xs" onClick={() => setShowInactive((v) => !v)}>
          {showInactive ? t('partner.list.activeOnly') : t('partner.list.showInactive')}
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
            title={t('partner.list.emptyTitle')}
            description={t('partner.list.emptyDesc')}
            actionLabel={t('partner.list.new')}
            onAction={() => navigate('/partners/new')}
          />
        ) : (
          <Table.ScrollContainer minWidth={400}>
            <Table>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>{t('partner.list.colName')}</Table.Th>
                  <Table.Th>{t('partner.list.colNote')}</Table.Th>
                  <Table.Th>{t('partner.list.colStatus')}</Table.Th>
                  <Table.Th>{t('partner.list.colActions')}</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {list.map((p) => (
                  <Table.Tr key={p.id}>
                    <Table.Td>{p.name}</Table.Td>
                    <Table.Td><Text size="sm" c="dimmed">{p.note || '—'}</Text></Table.Td>
                    <Table.Td>{p.is_active ? t('common.active') : t('common.inactive')}</Table.Td>
                    <Table.Td>
                      <Button variant="subtle" size="xs" onClick={() => navigate(`/partners/${p.id}`)}>
                        {t('common.view')}
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
