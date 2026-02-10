import { notifications } from '@mantine/notifications';

export function showError(message: string, title = 'Error') {
  notifications.show({
    title,
    message,
    color: 'red',
    autoClose: 5000,
  });
}

export function showSuccess(message: string, title = 'Success') {
  notifications.show({
    title,
    message,
    color: 'green',
    autoClose: 3000,
  });
}
