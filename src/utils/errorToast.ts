import { notifications } from '@mantine/notifications';
import i18n from '../i18n';

export function showError(message: string, title?: string) {
  notifications.show({
    title: title ?? i18n.t('common.error'),
    message,
    color: 'red',
    autoClose: 5000,
  });
}

export function showSuccess(message: string, title?: string) {
  notifications.show({
    title: title ?? i18n.t('common.success'),
    message,
    color: 'green',
    autoClose: 3000,
  });
}
