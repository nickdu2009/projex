import { AppShell, Group, NavLink, Title, Text } from '@mantine/core';
import {
  IconFolder,
  IconUsers,
  IconBuildingCommunity,
  IconSettings,
  IconBriefcase,
} from '@tabler/icons-react';
import { Outlet, useLocation, useNavigate } from 'react-router-dom';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { SyncStatusBar } from '../components/SyncStatusBar';
import { syncManager } from '../sync/SyncManager';

const NAV_ITEMS = [
  { to: '/projects', labelKey: 'nav.projects', icon: IconFolder },
  { to: '/people', labelKey: 'nav.people', icon: IconUsers },
  { to: '/partners', labelKey: 'nav.partners', icon: IconBuildingCommunity },
  { to: '/settings', labelKey: 'nav.settings', icon: IconSettings },
];

export function Layout() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const [syncEnabled, setSyncEnabled] = useState(false);

  useEffect(() => {
    syncManager.getConfig().then((cfg) => {
      setSyncEnabled(cfg.enabled);
      if (cfg.enabled) {
        syncManager.initialize();
      }
    }).catch(() => {});
  }, []);

  return (
    <AppShell
      header={{ height: 56 }}
      navbar={{ width: { base: 200, md: 220 }, breakpoint: 'sm' }}
      footer={syncEnabled ? { height: 40 } : undefined}
      padding={{ base: 'xs', sm: 'md' }}
      styles={{
        root: { height: '100vh', width: '100vw' },
        header: {
          backgroundColor: 'rgba(255, 255, 255, 0.7)',
          backdropFilter: 'blur(12px)',
          borderBottom: '1px solid rgba(0, 0, 0, 0.06)',
        },
        navbar: {
          backgroundColor: 'rgba(255, 255, 255, 0.7)',
          backdropFilter: 'blur(12px)',
          borderRight: '1px solid rgba(0, 0, 0, 0.06)',
        },
        main: {
          display: 'flex',
          flexDirection: 'column',
          height: '100%',
          overflow: 'auto',
        },
      }}
    >
      <AppShell.Header>
        <Group h="100%" px={{ base: 'xs', sm: 'md' }} align="center" gap="xs">
          <IconBriefcase size={24} style={{ color: '#6366f1' }} />
          <div>
            <Title order={4} style={{ lineHeight: 1.2, marginBottom: 2 }}>
              Projex
            </Title>
            <Text size="xs" c="dimmed" style={{ lineHeight: 1 }}>
              Project Management
            </Text>
          </div>
        </Group>
      </AppShell.Header>
      <AppShell.Navbar p="xs">
        {NAV_ITEMS.map(({ to, labelKey, icon: Icon }) => (
          <NavLink
            key={to}
            label={t(labelKey)}
            leftSection={<Icon size={20} stroke={1.5} />}
            active={location.pathname === to || location.pathname.startsWith(to + '/')}
            onClick={() => navigate(to)}
          />
        ))}
      </AppShell.Navbar>
      <AppShell.Main>
        <Outlet />
      </AppShell.Main>
      {syncEnabled && (
        <AppShell.Footer>
          <SyncStatusBar />
        </AppShell.Footer>
      )}
    </AppShell>
  );
}
