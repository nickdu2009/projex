import { notifications } from '@mantine/notifications';

export function showError(message: string, title = '错误') {
  notifications.show({
    title,
    message,
    color: 'red',
    autoClose: 5000,
  });
}

export function showSuccess(message: string, title = '成功') {
  notifications.show({
    title,
    message,
    color: 'green',
    autoClose: 3000,
  });
}
