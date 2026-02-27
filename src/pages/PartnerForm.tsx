import { Button, Paper, SimpleGrid, Stack, Text, TextInput, Textarea, Title } from '@mantine/core';
import { useIsMobile } from '../utils/useIsMobile';
import { IconArrowLeft, IconDeviceFloppy } from '@tabler/icons-react';
import { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { partnersApi } from '../api/partners';
import { showError, showSuccess } from '../utils/errorToast';
import { usePartnerStore } from '../stores/usePartnerStore';

export function PartnerForm() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const isMobile = useIsMobile();
  const isEdit = id && id !== 'new';
  const [loading, setLoading] = useState(false);
  const [loadPartner, setLoadPartner] = useState(true);
  const [name, setName] = useState('');
  const [note, setNote] = useState('');
  const invalidatePartners = usePartnerStore((s) => s.invalidate);

  useEffect(() => {
    if (!isEdit || !id) {
      setLoadPartner(false);
      return;
    }
    partnersApi.get(id).then((p) => {
      setName(p.name);
      setNote(p.note ?? '');
      setLoadPartner(false);
    }).catch((e) => {
      showError((e as { message?: string })?.message ?? t('common.failedToLoad'));
      setLoadPartner(false);
    });
  }, [id, isEdit, t]);

  const handleSubmit = useCallback(async () => {
    if (!name.trim()) {
      showError(t('partner.form.nameRequired'));
      return;
    }
    setLoading(true);
    try {
      if (isEdit && id) {
        await partnersApi.update({ id, name: name.trim(), note: note.trim() || undefined });
        showSuccess(t('common.saved'));
        invalidatePartners();
        navigate(`/partners/${id}`);
      } else {
        const p = await partnersApi.create({ name: name.trim(), note: note.trim() || undefined });
        showSuccess(t('common.created'));
        invalidatePartners();
        navigate(`/partners/${p.id}`);
      }
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? (isEdit ? t('common.failedToSave') : t('common.failedToCreate')));
    } finally {
      setLoading(false);
    }
  }, [id, isEdit, name, note, navigate, t, invalidatePartners]);

  if (loadPartner) return <Text size="sm">{t('common.loading')}</Text>;

  return (
    <Stack gap="md" w="100%" maw={isMobile ? '100%' : 640} pb="xl" style={{ alignSelf: 'flex-start' }}>
      <Button variant="subtle" leftSection={<IconArrowLeft size={16} />} onClick={() => navigate('/partners')}>
        {t('common.backToList')}
      </Button>
      <Paper>
        <Stack gap="md">
          <Title order={3}>{isEdit ? t('partner.form.editTitle') : t('partner.form.newTitle')}</Title>
          <SimpleGrid cols={{ base: 1, sm: 2 }} spacing="md" verticalSpacing="md">
            <TextInput label={t('partner.form.name')} required value={name} onChange={(e) => setName(e.target.value)} placeholder={t('partner.form.namePlaceholder')} />
            <Textarea label={t('partner.form.note')} value={note} onChange={(e) => setNote(e.target.value)} placeholder={t('common.optional')} minRows={1} />
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
