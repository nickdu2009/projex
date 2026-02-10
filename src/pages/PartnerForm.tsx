import { Button, Paper, SimpleGrid, Stack, Text, TextInput, Textarea, Title } from '@mantine/core';
import { IconArrowLeft, IconDeviceFloppy } from '@tabler/icons-react';
import { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { partnersApi } from '../api/partners';
import { showError, showSuccess } from '../utils/errorToast';
import { usePartnerStore } from '../stores/usePartnerStore';

export function PartnerForm() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
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
      showError((e as { message?: string })?.message ?? '加载失败');
      setLoadPartner(false);
    });
  }, [id, isEdit]);

  const handleSubmit = useCallback(async () => {
    if (!name.trim()) {
      showError('请填写合作方名称');
      return;
    }
    setLoading(true);
    try {
      if (isEdit && id) {
        await partnersApi.update({ id, name: name.trim(), note: note.trim() || undefined });
        showSuccess('已保存');
        invalidatePartners();
        navigate(`/partners/${id}`);
      } else {
        const p = await partnersApi.create({ name: name.trim(), note: note.trim() || undefined });
        showSuccess('已创建');
        invalidatePartners();
        navigate(`/partners/${p.id}`);
      }
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? (isEdit ? '保存失败' : '创建失败'));
    } finally {
      setLoading(false);
    }
  }, [id, isEdit, name, note, navigate]);

  if (loadPartner) return <Text size="sm">加载中…</Text>;

  return (
    <Stack gap="md" w="100%" maw={640} pb="xl" style={{ alignSelf: 'flex-start' }}>
      <Button variant="subtle" leftSection={<IconArrowLeft size={16} />} onClick={() => navigate('/partners')}>
        返回列表
      </Button>
      <Paper>
        <Stack gap="md">
          <Title order={3}>{isEdit ? '编辑合作方' : '新建合作方'}</Title>
          <SimpleGrid cols={{ base: 1, sm: 2 }} spacing="md" verticalSpacing="md">
            <TextInput label="名称" required value={name} onChange={(e) => setName(e.target.value)} placeholder="合作方名称" />
            <Textarea label="备注" value={note} onChange={(e) => setNote(e.target.value)} placeholder="选填" minRows={1} />
          </SimpleGrid>
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
