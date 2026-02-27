import { Button, Paper, Select, SimpleGrid, Stack, Text, TextInput, Textarea, Title } from '@mantine/core';
import { useIsMobile } from '../utils/useIsMobile';
import { IconArrowLeft, IconDeviceFloppy } from '@tabler/icons-react';
import { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { peopleApi } from '../api/people';
import { PERSON_ROLES } from '../constants/countries';
import { showError, showSuccess } from '../utils/errorToast';
import { usePersonStore } from '../stores/usePersonStore';

export function PersonForm() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const isMobile = useIsMobile();
  const isEdit = id && id !== 'new';
  const [loading, setLoading] = useState(false);
  const [loadPerson, setLoadPerson] = useState(true);
  const [displayName, setDisplayName] = useState('');
  const [email, setEmail] = useState('');
  const [role, setRole] = useState('');
  const [note, setNote] = useState('');
  const invalidatePersons = usePersonStore((s) => s.invalidate);

  // Resolve role labels via i18n
  const roleOptions = PERSON_ROLES.map((r) => ({ value: r.value, label: t(r.label) }));

  useEffect(() => {
    if (!isEdit || !id) {
      setLoadPerson(false);
      return;
    }
    peopleApi.get(id).then((p) => {
      setDisplayName(p.display_name);
      setEmail(p.email ?? '');
      setRole(p.role ?? '');
      setNote(p.note ?? '');
      setLoadPerson(false);
    }).catch((e) => {
      showError((e as { message?: string })?.message ?? t('common.failedToLoad'));
      setLoadPerson(false);
    });
  }, [id, isEdit, t]);

  const handleSubmit = useCallback(async () => {
    if (!displayName.trim()) {
      showError(t('person.form.nameRequired'));
      return;
    }
    setLoading(true);
    try {
      if (isEdit && id) {
        await peopleApi.update({
          id,
          displayName: displayName.trim(),
          email: email.trim() || undefined,
          role: role.trim() || undefined,
          note: note.trim() || undefined,
        });
        showSuccess(t('common.saved'));
        invalidatePersons();
        navigate(`/people/${id}`);
      } else {
        const p = await peopleApi.create({
          displayName: displayName.trim(),
          email: email.trim() || undefined,
          role: role.trim() || undefined,
          note: note.trim() || undefined,
        });
        showSuccess(t('common.created'));
        invalidatePersons();
        navigate(`/people/${p.id}`);
      }
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? (isEdit ? t('common.failedToSave') : t('common.failedToCreate')));
    } finally {
      setLoading(false);
    }
  }, [id, isEdit, displayName, email, role, note, navigate, t, invalidatePersons]);

  if (loadPerson) return <Text size="sm">{t('common.loading')}</Text>;

  return (
    <Stack gap="md" w="100%" maw={isMobile ? '100%' : 640} pb="xl" style={{ alignSelf: 'flex-start' }}>
      <Button variant="subtle" leftSection={<IconArrowLeft size={16} />} onClick={() => navigate('/people')}>
        {t('common.backToList')}
      </Button>
      <Paper>
        <Stack gap="md">
          <Title order={3}>{isEdit ? t('person.form.editTitle') : t('person.form.newTitle')}</Title>
          <SimpleGrid cols={{ base: 1, sm: 2 }} spacing="md" verticalSpacing="md">
            <TextInput label={t('person.form.name')} required value={displayName} onChange={(e) => setDisplayName(e.target.value)} placeholder={t('person.form.namePlaceholder')} />
            <TextInput label={t('person.form.email')} value={email} onChange={(e) => setEmail(e.target.value)} placeholder={t('common.optional')} type="email" />
            <Select
              label={t('person.form.role')}
              placeholder={t('person.form.rolePlaceholder')}
              data={roleOptions}
              value={role}
              onChange={(v) => setRole(v || '')}
              clearable
              searchable
            />
            <Textarea label={t('person.form.note')} value={note} onChange={(e) => setNote(e.target.value)} placeholder={t('common.optional')} minRows={1} />
          </SimpleGrid>
          <Button
            loading={loading}
            onClick={handleSubmit}
            variant="gradient"
            gradient={{ from: 'indigo', to: 'violet' }}
            leftSection={<IconDeviceFloppy size={18} />}
            fullWidth={isMobile}
            style={isMobile ? undefined : { alignSelf: 'flex-start' }}
          >
            {isEdit ? t('common.save') : t('common.create')}
          </Button>
        </Stack>
      </Paper>
    </Stack>
  );
}
