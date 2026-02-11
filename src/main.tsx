import { MantineProvider } from '@mantine/core';
import '@mantine/core/styles.css';
import { DatesProvider } from '@mantine/dates';
import '@mantine/dates/styles.css';
import '@mantine/tiptap/styles.css';
import 'dayjs/locale/en';
import dayjs from 'dayjs';

dayjs.locale('en');
import { Notifications } from '@mantine/notifications';
import '@mantine/notifications/styles.css';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './i18n'; // i18n must be imported before App
import './index.css';
import App from './App.tsx';
import { theme } from './theme';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <MantineProvider theme={theme} defaultColorScheme="light">
      <DatesProvider settings={{ locale: 'en', firstDayOfWeek: 1 }}>
        <Notifications position="top-right" />
        <App />
      </DatesProvider>
    </MantineProvider>
  </StrictMode>,
);
