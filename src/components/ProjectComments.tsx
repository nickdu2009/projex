import {
  ActionIcon,
  Avatar,
  Badge,
  Box,
  Button,
  Card,
  Collapse,
  Divider,
  Flex,
  Group,
  Paper,
  Stack,
  Text,
  Title,
  Tooltip,
} from '@mantine/core';
import {
  IconChevronUp,
  IconEdit,
  IconMessageCircle,
  IconPin,
  IconPinFilled,
  IconPlus,
  IconTrash,
} from '@tabler/icons-react';
import { type JSONContent } from '@tiptap/react';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { commentApi, type CommentDto } from '../api/comments';
import { peopleApi } from '../api/people';
import { ConfirmModal } from './ConfirmModal';
import { EmptyState } from './EmptyState';
import { RichTextEditor } from './RichTextEditor';
import type { MentionItem } from './mentionSuggestion';
import { showError, showSuccess } from '../utils/errorToast';

interface ProjectCommentsProps {
  projectId: string;
}

export function ProjectComments({ projectId }: ProjectCommentsProps) {
  const { t } = useTranslation();
  const [comments, setComments] = useState<CommentDto[]>([]);
  const [loading, setLoading] = useState(true);

  // People list for @mention
  const [mentionItems, setMentionItems] = useState<MentionItem[]>([]);

  // New comment form
  const [newContent, setNewContent] = useState<JSONContent>({ type: 'doc', content: [] });
  const [showNewForm, setShowNewForm] = useState(false);

  // Edit state
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editContent, setEditContent] = useState<JSONContent>({ type: 'doc', content: [] });

  // Delete confirmation
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);

  const loadComments = useCallback(async () => {
    try {
      const data = await commentApi.list(projectId);
      setComments(data);
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('comment.loadFailed'));
    } finally {
      setLoading(false);
    }
  }, [projectId, t]);

  useEffect(() => {
    loadComments();
  }, [loadComments]);

  useEffect(() => {
    peopleApi
      .list(true)
      .then((ps) => {
        setMentionItems(ps.map((p) => ({ id: p.id, label: p.display_name })));
      })
      .catch(() => {});
  }, []);

  const handleCreate = async () => {
    const contentStr = JSON.stringify(newContent);
    if (!contentStr || contentStr === '{"type":"doc","content":[]}') {
      showError(t('comment.emptyContent'));
      return;
    }

    try {
      await commentApi.create({
        projectId,
        content: contentStr,
        isPinned: false,
      });
      setNewContent({ type: 'doc', content: [] });
      setShowNewForm(false);
      loadComments();
      showSuccess(t('comment.savedSuccess'));
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('comment.saveFailed'));
    }
  };

  const startEdit = (comment: CommentDto) => {
    setEditingId(comment.id);
    setEditContent(JSON.parse(comment.content));
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditContent({ type: 'doc', content: [] });
  };

  const handleUpdate = async (id: string) => {
    try {
      await commentApi.update({
        id,
        content: JSON.stringify(editContent),
      });
      cancelEdit();
      loadComments();
      showSuccess(t('comment.savedSuccess'));
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('comment.saveFailed'));
    }
  };

  const handleTogglePin = async (comment: CommentDto) => {
    try {
      await commentApi.update({ id: comment.id, isPinned: !comment.isPinned });
      loadComments();
      showSuccess(comment.isPinned ? t('comment.unpinSuccess') : t('comment.pinSuccess'));
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('common.operationFailed'));
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await commentApi.delete(id);
      setDeleteConfirm(null);
      loadComments();
      showSuccess(t('comment.deletedSuccess'));
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('comment.deleteFailed'));
    }
  };

  /** Generate initials from person name */
  const getInitials = (name: string | null) => {
    if (!name) return '?';
    return name
      .split(' ')
      .map((n) => n[0])
      .join('')
      .toUpperCase()
      .slice(0, 2);
  };

  /** Format relative time */
  const formatTime = (dateStr: string) => {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMin = Math.floor(diffMs / 60000);
    const diffHr = Math.floor(diffMin / 60);
    const diffDay = Math.floor(diffHr / 24);

    if (diffMin < 1) return t('comment.justNow');
    if (diffMin < 60) return t('comment.minutesAgo', { count: diffMin });
    if (diffHr < 24) return t('comment.hoursAgo', { count: diffHr });
    if (diffDay < 7) return t('comment.daysAgo', { count: diffDay });
    return date.toLocaleDateString();
  };

  if (loading) {
    return <Text>{t('common.loading')}</Text>;
  }

  return (
    <Paper>
      <Stack gap="lg">
        {/* Section Header */}
        <Flex justify="space-between" align="center">
          <Group gap="xs">
            <IconMessageCircle size={20} stroke={1.5} style={{ color: 'var(--mantine-color-indigo-6)' }} />
            <Title order={5}>{t('comment.title')}</Title>
            {comments.length > 0 && (
              <Badge size="sm" variant="light" color="indigo" circle>
                {comments.length}
              </Badge>
            )}
          </Group>
          <Button
            size="xs"
            variant={showNewForm ? 'light' : 'gradient'}
            gradient={{ from: 'indigo', to: 'violet' }}
            leftSection={showNewForm ? <IconChevronUp size={14} /> : <IconPlus size={14} />}
            onClick={() => setShowNewForm((v) => !v)}
          >
            {showNewForm ? t('common.cancel') : t('comment.add')}
          </Button>
        </Flex>

        {/* New Comment Form (collapsible) */}
        <Collapse in={showNewForm}>
          <Card withBorder radius="md" style={{ borderColor: 'var(--mantine-color-indigo-2)' }}>
            <Stack gap="sm">
              <RichTextEditor
                content={newContent}
                onChange={setNewContent}
                placeholder={t('comment.placeholder')}
                mentionItems={mentionItems}
              />

              <Flex justify="flex-end" gap="xs">
                <Button
                  size="sm"
                  variant="subtle"
                  onClick={() => {
                    setShowNewForm(false);
                    setNewContent({ type: 'doc', content: [] });
                  }}
                >
                  {t('common.cancel')}
                </Button>
                <Button
                  size="sm"
                  variant="gradient"
                  gradient={{ from: 'indigo', to: 'violet' }}
                  leftSection={<IconPlus size={16} />}
                  onClick={handleCreate}
                >
                  {t('comment.add')}
                </Button>
              </Flex>
            </Stack>
          </Card>
        </Collapse>

        {/* Comments List */}
        {comments.length === 0 ? (
          <EmptyState
            icon={IconMessageCircle}
            title={t('comment.title')}
            description={t('comment.noComments')}
          />
        ) : (
          <Stack gap="md">
            {comments.map((comment, idx) => (
              <Box key={comment.id}>
                {/* Comment Item */}
                <Flex gap="sm" align="flex-start">
                  {/* Avatar */}
                  <Avatar
                    size={36}
                    radius="xl"
                    color="indigo"
                    variant="filled"
                    style={{ flexShrink: 0, marginTop: 2 }}
                  >
                    {getInitials(comment.personName)}
                  </Avatar>

                  {/* Content */}
                  <Box style={{ flex: 1, minWidth: 0 }}>
                    {/* Header row */}
                    <Flex justify="space-between" align="center" mb={4}>
                      <Group gap={6}>
                        <Text size="sm" fw={600}>
                          {comment.personName ?? t('comment.anonymous')}
                        </Text>
                        <Text size="xs" c="dimmed">
                          {formatTime(comment.createdAt)}
                        </Text>
                        {comment.updatedAt !== comment.createdAt && (
                          <Text size="xs" c="dimmed" fs="italic">
                            ({t('comment.edited')})
                          </Text>
                        )}
                        {comment.isPinned && (
                          <Badge
                            size="xs"
                            color="yellow"
                            variant="light"
                            leftSection={<IconPinFilled size={10} />}
                          >
                            {t('comment.pinned')}
                          </Badge>
                        )}
                      </Group>

                      {/* Actions */}
                      <Group gap={4}>
                        <Tooltip
                          label={comment.isPinned ? t('comment.unpin') : t('comment.pin')}
                          withArrow
                          position="top"
                        >
                          <ActionIcon
                            size="sm"
                            variant="subtle"
                            color={comment.isPinned ? 'yellow' : 'gray'}
                            onClick={() => handleTogglePin(comment)}
                          >
                            {comment.isPinned ? <IconPinFilled size={14} /> : <IconPin size={14} />}
                          </ActionIcon>
                        </Tooltip>

                        {editingId !== comment.id && (
                          <>
                            <Tooltip label={t('comment.edit')} withArrow position="top">
                              <ActionIcon
                                size="sm"
                                variant="subtle"
                                color="gray"
                                onClick={() => startEdit(comment)}
                              >
                                <IconEdit size={14} />
                              </ActionIcon>
                            </Tooltip>
                            <Tooltip label={t('comment.delete')} withArrow position="top">
                              <ActionIcon
                                size="sm"
                                variant="subtle"
                                color="red"
                                onClick={() => setDeleteConfirm(comment.id)}
                              >
                                <IconTrash size={14} />
                              </ActionIcon>
                            </Tooltip>
                          </>
                        )}
                      </Group>
                    </Flex>

                    {/* Comment body */}
                    {editingId === comment.id ? (
                      <Card withBorder radius="md" p="sm" mt={4} style={{ borderColor: 'var(--mantine-color-indigo-2)' }}>
                        <Stack gap="sm">
                          <RichTextEditor content={editContent} onChange={setEditContent} mentionItems={mentionItems} />
                          <Flex gap="xs" justify="flex-end">
                            <Button size="xs" variant="subtle" onClick={cancelEdit}>
                              {t('common.cancel')}
                            </Button>
                            <Button
                              size="xs"
                              variant="gradient"
                              gradient={{ from: 'indigo', to: 'violet' }}
                              onClick={() => handleUpdate(comment.id)}
                            >
                              {t('common.save')}
                            </Button>
                          </Flex>
                        </Stack>
                      </Card>
                    ) : (
                      <Box
                        mt={2}
                        style={{
                          fontSize: 'var(--mantine-font-size-sm)',
                          lineHeight: 1.6,
                        }}
                      >
                        <RichTextEditor content={comment.content} editable={false} />
                      </Box>
                    )}
                  </Box>
                </Flex>

                {/* Separator between comments */}
                {idx < comments.length - 1 && <Divider mt="md" />}
              </Box>
            ))}
          </Stack>
        )}
      </Stack>

      {/* Delete Confirmation Modal */}
      <ConfirmModal
        opened={!!deleteConfirm}
        onClose={() => setDeleteConfirm(null)}
        onConfirm={() => deleteConfirm && handleDelete(deleteConfirm)}
        title={t('comment.delete')}
        message={t('comment.confirmDelete')}
        confirmLabel={t('common.delete')}
      />
    </Paper>
  );
}
