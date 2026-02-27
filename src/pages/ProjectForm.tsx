import { Button, NumberInput, Paper, Select, SimpleGrid, Stack, Text, TextInput, Title } from '@mantine/core';
import { useIsMobile } from '../utils/useIsMobile';
import { DatePickerInput } from '@mantine/dates';
import { IconArrowLeft, IconDeviceFloppy } from '@tabler/icons-react';
import dayjs from 'dayjs';
import { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { projectApi } from '../api/projects';
import { getCountries } from '../constants/countries';
import { showError, showSuccess } from '../utils/errorToast';
import { usePartnerStore } from '../stores/usePartnerStore';
import { usePersonStore } from '../stores/usePersonStore';
import { useTagStore } from '../stores/useTagStore';

function parseDate(s: string | null | undefined): Date | null {
  if (!s || !s.trim()) return null;
  const d = dayjs(s.trim(), 'YYYY-MM-DD', true);
  return d.isValid() ? d.toDate() : null;
}

function formatDate(d: Date | null): string | undefined {
  return d ? dayjs(d).format('YYYY-MM-DD') : undefined;
}

function generateProjectName(args: { countryCode: string; partnerName: string; productName: string }): string {
  const parts = [
    args.countryCode.trim().toUpperCase(),
    args.partnerName.trim(),
    args.productName.trim(),
  ].filter(Boolean);
  return parts.join('-');
}

export function ProjectForm() {
  const { t, i18n } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const isMobile = useIsMobile();
  const isEdit = id && id !== 'new';
  const [loading, setLoading] = useState(false);
  const [loadProject, setLoadProject] = useState(true);
  const [name, setName] = useState('');
  const [nameEdited, setNameEdited] = useState(false);
  const [productName, setProductName] = useState('');
  const [description, setDescription] = useState('');
  const [priority, setPriority] = useState(3);
  const [countryCode, setCountryCode] = useState('CN');
  const [partnerId, setPartnerId] = useState<string | null>(null);
  const [ownerPersonId, setOwnerPersonId] = useState<string | null>(null);
  const [startDate, setStartDate] = useState<Date | null>(null);
  const [dueDate, setDueDate] = useState<Date | null>(null);
  const [tagsStr, setTagsStr] = useState('');

  // Zustand stores
  const { loaded: partnersLoaded, fetch: fetchPartners, activeOptions: partnerOptions } = usePartnerStore();
  const { loaded: personsLoaded, fetch: fetchPersons, activeOptions: personOptions } = usePersonStore();
  const { invalidate: invalidateTags } = useTagStore();

  useEffect(() => {
    if (!partnersLoaded) fetchPartners(true);
    if (!personsLoaded) fetchPersons(true);
  }, [partnersLoaded, personsLoaded, fetchPartners, fetchPersons]);

  useEffect(() => {
    if (!isEdit || !id) {
      setLoadProject(false);
      return;
    }
    projectApi.get(id).then((p) => {
      setName(p.name);
      setNameEdited(true);
      setProductName(p.product_name ?? '');
      setDescription(p.description ?? '');
      setPriority(p.priority);
      setCountryCode(p.country_code);
      setPartnerId(p.partner_id);
      setOwnerPersonId(p.owner_person_id);
      setStartDate(parseDate(p.start_date ?? undefined));
      setDueDate(parseDate(p.due_date ?? undefined));
      setTagsStr(p.tags?.length ? p.tags.join(', ') : '');
      setLoadProject(false);
    }).catch((e) => {
      showError((e as { message?: string })?.message ?? t('common.failedToLoad'));
      setLoadProject(false);
    });
  }, [id, isEdit, t]);

  useEffect(() => {
    if (isEdit) return;
    if (nameEdited) return;
    if (!partnerId) return;
    const partnerName = partnerOptions().find((o) => o.value === partnerId)?.label ?? '';
    if (!partnerName.trim()) return;
    const nextName = generateProjectName({ countryCode, partnerName, productName });
    if (!nextName.trim()) return;
    setName(nextName);
  }, [countryCode, isEdit, nameEdited, partnerId, partnerOptions, productName]);

  const handleSubmit = useCallback(async () => {
    if (!name.trim()) {
      showError(t('project.form.nameRequired'));
      return;
    }
    if (!countryCode.trim()) {
      showError(t('project.form.countryRequired'));
      return;
    }
    if (!ownerPersonId) {
      showError(t('project.form.ownerRequired'));
      return;
    }
    if (!isEdit && !partnerId) {
      showError(t('project.form.partnerRequired'));
      return;
    }
    setLoading(true);
    try {
      if (isEdit && id) {
        await projectApi.update({
          id,
          name: name.trim(),
          productName: productName.trim(),
          description: description.trim() || undefined,
          priority,
          countryCode: countryCode.trim(),
          ownerPersonId,
          startDate: formatDate(startDate),
          dueDate: formatDate(dueDate),
          tags: tagsStr.split(/[,，]/).map((tag) => tag.trim()).filter(Boolean),
        });
        showSuccess(t('common.saved'));
        invalidateTags();
        navigate(`/projects/${id}`);
      } else {
        const p = await projectApi.create({
          name: name.trim(),
          countryCode: countryCode.trim(),
          partnerId: partnerId!,
          ownerPersonId,
          productName: productName.trim() || undefined,
          description: description.trim() || undefined,
          priority,
          startDate: formatDate(startDate),
          dueDate: formatDate(dueDate),
          tags: tagsStr.split(/[,，]/).map((tag) => tag.trim()).filter(Boolean),
        });
        showSuccess(t('common.created'));
        invalidateTags();
        navigate(`/projects/${p.id}`);
      }
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? (isEdit ? t('common.failedToSave') : t('common.failedToCreate')));
    } finally {
      setLoading(false);
    }
  }, [id, isEdit, name, productName, description, priority, countryCode, partnerId, ownerPersonId, startDate, dueDate, tagsStr, navigate, t, invalidateTags]);

  if (loadProject) return <Text size="sm">{t('common.loading')}</Text>;

  return (
    <Stack gap="md" w="100%" maw={isMobile ? '100%' : 960} pb="xl" style={{ alignSelf: 'flex-start' }}>
      <Button variant="subtle" leftSection={<IconArrowLeft size={16} />} onClick={() => navigate('/projects')}>
        {t('common.backToList')}
      </Button>
      <Paper>
        <Stack gap="md">
          <Title order={3}>{isEdit ? t('project.form.editTitle') : t('project.form.newTitle')}</Title>
          <SimpleGrid cols={{ base: 1, sm: 2 }} spacing="md" verticalSpacing="md">
        <TextInput
          label={t('project.form.name')}
          required
          value={name}
          onChange={(e) => {
            setName(e.target.value);
            setNameEdited(true);
          }}
          placeholder={t('project.form.namePlaceholder')}
        />
        <TextInput
          label={t('project.form.productName')}
          value={productName}
          onChange={(e) => setProductName(e.target.value)}
          placeholder={t('project.form.productNamePlaceholder')}
        />
        <TextInput label={t('project.form.description')} value={description} onChange={(e) => setDescription(e.target.value)} placeholder={t('common.optional')} />
        <NumberInput label={t('project.form.priority')} min={1} max={5} value={priority} onChange={(v) => setPriority(Number(v) || 3)} />
        <Select
          label={t('project.form.country')}
          required
          data={getCountries(i18n.language).map((c) => ({ value: c.code, label: `${c.code} ${c.name}` }))}
          value={countryCode}
          onChange={(v) => v && setCountryCode(v)}
        />
        {!isEdit && (
          <Select
            label={t('project.form.partner')}
            required
            data={partnerOptions()}
            value={partnerId}
            onChange={setPartnerId}
            searchable
            placeholder={t('project.form.partnerPlaceholder')}
          />
        )}
        {isEdit && <Text size="sm" c="dimmed" style={{ gridColumn: '1 / -1' }}>{t('project.form.partnerImmutable')}</Text>}
        <Select
          label={t('project.form.owner')}
          required
          data={personOptions()}
          value={ownerPersonId}
          onChange={setOwnerPersonId}
          searchable
        />
        <DatePickerInput
          label={t('project.form.startDate')}
          placeholder={t('common.pickDate')}
          value={startDate}
          onChange={setStartDate}
          valueFormat="YYYY-MM-DD"
          clearable
        />
        <DatePickerInput
          label={t('project.form.dueDate')}
          placeholder={t('common.pickDate')}
          value={dueDate}
          onChange={setDueDate}
          valueFormat="YYYY-MM-DD"
          clearable
        />
          </SimpleGrid>
          <TextInput label={t('project.form.tags')} value={tagsStr} onChange={(e) => setTagsStr(e.target.value)} placeholder={t('project.form.tagsPlaceholder')} />
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
