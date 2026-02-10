import { Button, Paper, Select, SimpleGrid, Stack, Text, TextInput, Textarea, Title } from '@mantine/core';
import { IconArrowLeft, IconDeviceFloppy } from '@tabler/icons-react';
import { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { peopleApi } from '../api/people';
import { PERSON_ROLES } from '../constants/countries';
import { showError, showSuccess } from '../utils/errorToast';

export function PersonForm() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const isEdit = id && id !== 'new';
  const [loading, setLoading] = useState(false);
  const [loadPerson, setLoadPerson] = useState(true);
  const [displayName, setDisplayName] = useState('');
  const [email, setEmail] = useState('');
  const [role, setRole] = useState('');
  const [note, setNote] = useState('');

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
      showError((e as { message?: string })?.message ?? '加载失败');
      setLoadPerson(false);
    });
  }, [id, isEdit]);

  const handleSubmit = useCallback(async () => {
    if (!displayName.trim()) {
      showError('请填写姓名');
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
        showSuccess('已保存');
        navigate(`/people/${id}`);
      } else {
        const p = await peopleApi.create({
          displayName: displayName.trim(),
          email: email.trim() || undefined,
          role: role.trim() || undefined,
          note: note.trim() || undefined,
        });
        showSuccess('已创建');
        navigate(`/people/${p.id}`);
      }
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? (isEdit ? '保存失败' : '创建失败'));
    } finally {
      setLoading(false);
    }
  }, [id, isEdit, displayName, email, role, note, navigate]);

  if (loadPerson) return <Text size="sm">加载中…</Text>;

  return (
    <Stack gap="md" w="100%" maw={640} pb="xl" style={{ alignSelf: 'flex-start' }}>
      <Button variant="subtle" leftSection={<IconArrowLeft size={16} />} onClick={() => navigate('/people')}>
        返回列表
      </Button>
      <Paper>
        <Stack gap="md">
          <Title order={3}>{isEdit ? '编辑成员' : '新建成员'}</Title>
          <SimpleGrid cols={{ base: 1, sm: 2 }} spacing="md" verticalSpacing="md">
            <TextInput label="姓名" required value={displayName} onChange={(e) => setDisplayName(e.target.value)} placeholder="显示名称" />
            <TextInput label="邮箱" value={email} onChange={(e) => setEmail(e.target.value)} placeholder="选填" type="email" />
            <Select
              label="角色"
              placeholder="选择角色"
              data={PERSON_ROLES}
              value={role}
              onChange={(v) => setRole(v || '')}
              clearable
              searchable
            />
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
