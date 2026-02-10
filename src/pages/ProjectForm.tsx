import { Button, NumberInput, Paper, Select, SimpleGrid, Stack, Text, TextInput, Title } from '@mantine/core';
import { DatePickerInput } from '@mantine/dates';
import { IconArrowLeft, IconDeviceFloppy } from '@tabler/icons-react';
import dayjs from 'dayjs';
import { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { projectApi } from '../api/projects';
import { COUNTRIES } from '../constants/countries';
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

export function ProjectForm() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const isEdit = id && id !== 'new';
  const [loading, setLoading] = useState(false);
  const [loadProject, setLoadProject] = useState(true);
  const [name, setName] = useState('');
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
      showError((e as { message?: string })?.message ?? '加载失败');
      setLoadProject(false);
    });
  }, [id, isEdit]);

  const handleSubmit = useCallback(async () => {
    if (!name.trim()) {
      showError('请填写项目名称');
      return;
    }
    if (!countryCode.trim()) {
      showError('请选择国家');
      return;
    }
    if (!ownerPersonId) {
      showError('请选择负责人');
      return;
    }
    if (!isEdit && !partnerId) {
      showError('请选择合作方');
      return;
    }
    setLoading(true);
    try {
      if (isEdit && id) {
        await projectApi.update({
          id,
          name: name.trim(),
          description: description.trim() || undefined,
          priority,
          countryCode: countryCode.trim(),
          ownerPersonId,
          startDate: formatDate(startDate),
          dueDate: formatDate(dueDate),
          tags: tagsStr.split(/[,，]/).map((t) => t.trim()).filter(Boolean),
        });
        showSuccess('已保存');
        invalidateTags();
        navigate(`/projects/${id}`);
      } else {
        const p = await projectApi.create({
          name: name.trim(),
          countryCode: countryCode.trim(),
          partnerId: partnerId!,
          ownerPersonId,
          description: description.trim() || undefined,
          priority,
          startDate: formatDate(startDate),
          dueDate: formatDate(dueDate),
          tags: tagsStr.split(/[,，]/).map((t) => t.trim()).filter(Boolean),
        });
        showSuccess('已创建');
        invalidateTags();
        navigate(`/projects/${p.id}`);
      }
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? (isEdit ? '保存失败' : '创建失败'));
    } finally {
      setLoading(false);
    }
  }, [id, isEdit, name, description, priority, countryCode, partnerId, ownerPersonId, startDate, dueDate, tagsStr, navigate]);

  if (loadProject) return <Text size="sm">加载中…</Text>;

  return (
    <Stack gap="md" w="100%" maw={960} pb="xl" style={{ alignSelf: 'flex-start' }}>
      <Button variant="subtle" leftSection={<IconArrowLeft size={16} />} onClick={() => navigate('/projects')}>
        返回列表
      </Button>
      <Paper>
        <Stack gap="md">
          <Title order={3}>{isEdit ? '编辑项目' : '新建项目'}</Title>
          <SimpleGrid cols={{ base: 1, sm: 2 }} spacing="md" verticalSpacing="md">
        <TextInput label="名称" required value={name} onChange={(e) => setName(e.target.value)} placeholder="项目名称" />
        <TextInput label="描述" value={description} onChange={(e) => setDescription(e.target.value)} placeholder="选填" />
        <NumberInput label="优先级 (1-5)" min={1} max={5} value={priority} onChange={(v) => setPriority(Number(v) || 3)} />
        <Select
          label="国家"
          required
          data={COUNTRIES.map((c) => ({ value: c.code, label: `${c.code} ${c.name}` }))}
          value={countryCode}
          onChange={(v) => v && setCountryCode(v)}
        />
        {!isEdit && (
          <Select
            label="合作方"
            required
            data={partnerOptions()}
            value={partnerId}
            onChange={setPartnerId}
            searchable
            placeholder="选择合作方（创建后不可变更）"
          />
        )}
        {isEdit && <Text size="sm" c="dimmed" style={{ gridColumn: '1 / -1' }}>合作方创建后不可变更</Text>}
        <Select
          label="负责人"
          required
          data={personOptions()}
          value={ownerPersonId}
          onChange={setOwnerPersonId}
          searchable
        />
        <DatePickerInput
          label="开始日期"
          placeholder="选择日期"
          value={startDate}
          onChange={setStartDate}
          valueFormat="YYYY-MM-DD"
          clearable
        />
        <DatePickerInput
          label="截止日期"
          placeholder="选择日期"
          value={dueDate}
          onChange={setDueDate}
          valueFormat="YYYY-MM-DD"
          clearable
        />
          </SimpleGrid>
          <TextInput label="标签" value={tagsStr} onChange={(e) => setTagsStr(e.target.value)} placeholder="逗号分隔" />
          <Button
            loading={loading}
            onClick={handleSubmit}
            variant="gradient"
            gradient={{ from: 'indigo', to: 'violet' }}
            leftSection={<IconDeviceFloppy size={18} />}
            style={{ alignSelf: 'flex-start' }}
          >
            {isEdit ? '保存' : '创建'}
          </Button>
        </Stack>
      </Paper>
    </Stack>
  );
}
