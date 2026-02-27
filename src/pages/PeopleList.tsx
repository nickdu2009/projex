import { Badge, Button, Card, Flex, Group, Loader, Paper, Stack, Table, Text, Title } from '@mantine/core';
import { IconPlus, IconUsers } from '@tabler/icons-react';
import { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { peopleApi, type PersonDto } from '../api/people';
import { showError } from '../utils/errorToast';
import { getRoleLabel } from '../utils/roleLabel';
import { EmptyState } from '../components/EmptyState';
import { useIsMobile } from '../utils/useIsMobile';

export function PeopleList() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const isMobile = useIsMobile();
  const [list, setList] = useState<PersonDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [showInactive, setShowInactive] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await peopleApi.list(!showInactive);
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
        <Title order={3}>{t('person.list.title')}</Title>
        <Button
          variant="gradient"
          gradient={{ from: 'indigo', to: 'violet' }}
          leftSection={<IconPlus size={18} />}
          onClick={() => navigate('/people/new')}
        >
          {t('person.list.new')}
        </Button>
      </Flex>

      <Paper>
        <Button variant="subtle" size="xs" onClick={() => setShowInactive((v) => !v)}>
          {showInactive ? t('person.list.activeOnly') : t('person.list.showInactive')}
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
            title={t('person.list.emptyTitle')}
            description={t('person.list.emptyDesc')}
            actionLabel={t('person.list.new')}
            onAction={() => navigate('/people/new')}
          />
        ) : isMobile ? (
          /* Mobile card view */
          <Stack gap="xs" p="xs">
            {list.map((p) => (
              <Card
                key={p.id}
                padding="sm"
                radius="md"
                withBorder
                style={{ cursor: 'pointer' }}
                onClick={() => navigate(`/people/${p.id}`)}
              >
                <Group justify="space-between" wrap="nowrap" gap="xs">
                  <Stack gap={4} style={{ minWidth: 0, flex: 1 }}>
                    <Text fw={600} size="sm" style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                      {p.display_name}
                    </Text>
                    {p.email && <Text size="xs" c="dimmed">{p.email}</Text>}
                    {p.role && <Text size="xs" c="dimmed">{getRoleLabel(p.role)}</Text>}
                  </Stack>
                  <Badge size="xs" color={p.is_active ? 'teal' : 'gray'} style={{ flexShrink: 0 }}>
                    {p.is_active ? t('common.active') : t('common.inactive')}
                  </Badge>
                </Group>
              </Card>
            ))}
          </Stack>
        ) : (
          /* Desktop table view */
          <Table.ScrollContainer minWidth={600}>
            <Table>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>{t('person.list.colName')}</Table.Th>
                  <Table.Th>{t('person.list.colEmail')}</Table.Th>
                  <Table.Th>{t('person.list.colRole')}</Table.Th>
                  <Table.Th>{t('person.list.colNote')}</Table.Th>
                  <Table.Th>{t('person.list.colStatus')}</Table.Th>
                  <Table.Th>{t('person.list.colActions')}</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {list.map((p) => (
                  <Table.Tr key={p.id}>
                    <Table.Td>{p.display_name}</Table.Td>
                    <Table.Td><Text size="sm" c="dimmed">{p.email || '—'}</Text></Table.Td>
                    <Table.Td><Text size="sm">{p.role ? getRoleLabel(p.role) : '—'}</Text></Table.Td>
                    <Table.Td><Text size="sm" c="dimmed">{p.note || '—'}</Text></Table.Td>
                    <Table.Td>{p.is_active ? t('common.active') : t('common.inactive')}</Table.Td>
                    <Table.Td>
                      <Button variant="subtle" size="xs" onClick={() => navigate(`/people/${p.id}`)}>
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
