import { Button, Group, Modal, Stack, Text } from '@mantine/core';
import { useTranslation } from 'react-i18next';
import { useIsMobile } from '../utils/useIsMobile';

interface ConfirmModalProps {
  opened: boolean;
  onClose: () => void;
  onConfirm: () => void;
  title: string;
  message: string;
  confirmLabel?: string;
  confirmColor?: string;
  loading?: boolean;
}

export function ConfirmModal({
  opened,
  onClose,
  onConfirm,
  title,
  message,
  confirmLabel,
  confirmColor = 'red',
  loading = false,
}: ConfirmModalProps) {
  const { t } = useTranslation();
  const isMobile = useIsMobile();
  return (
    <Modal
      opened={opened}
      onClose={onClose}
      title={title}
      centered
      size={isMobile ? '100%' : 'sm'}
      fullScreen={isMobile}
    >
      <Stack>
        <Text size="sm">{message}</Text>
        <Group justify={isMobile ? 'stretch' : 'flex-end'} wrap="wrap">
          <Button
            variant="subtle"
            onClick={onClose}
            disabled={loading}
            fullWidth={isMobile}
          >
            {t('common.cancel')}
          </Button>
          <Button
            color={confirmColor}
            onClick={onConfirm}
            loading={loading}
            fullWidth={isMobile}
          >
            {confirmLabel ?? t('common.confirm')}
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
}
