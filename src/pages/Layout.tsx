import { AppShell, Burger, Drawer, Group, NavLink, Stack, Title, Text } from '@mantine/core';
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
import { useIsMobile } from '../utils/useIsMobile';
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
  const isMobile = useIsMobile();
  const [drawerOpened, setDrawerOpened] = useState(false);
  const [syncEnabled, setSyncEnabled] = useState(false);

  useEffect(() => {
    let cancelled = false;
    syncManager
      .getConfig()
      .then((cfg) => {
        if (cancelled) return;
        setSyncEnabled(cfg.enabled);
        if (cfg.enabled) {
          syncManager.initialize();
        }
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
    // Refresh config when route changes so Settings toggles take effect.
  }, [location.pathname]);

  const handleNavClick = (to: string) => {
    navigate(to);
    setDrawerOpened(false);
  };

  const navLinks = NAV_ITEMS.map(({ to, labelKey, icon: Icon }) => (
    <NavLink
      key={to}
      label={t(labelKey)}
      leftSection={<Icon size={20} stroke={1.5} />}
      active={location.pathname === to || location.pathname.startsWith(to + '/')}
      onClick={() => handleNavClick(to)}
    />
  ));

  return (
    <>
      {/* Mobile drawer navigation */}
      {isMobile && (
        <Drawer
          opened={drawerOpened}
          onClose={() => setDrawerOpened(false)}
          size="xs"
          title={
            <Group gap="xs">
              <IconBriefcase size={20} style={{ color: '#6366f1' }} />
              <Text fw={600}>Projex</Text>
            </Group>
          }
          styles={{
            content: { maxWidth: 240 },
          }}
        >
          <Stack gap={0} mt="xs">
            {navLinks}
          </Stack>
        </Drawer>
      )}

      <AppShell
        header={{ height: 56 }}
        navbar={
          isMobile
            ? undefined
            : { width: { base: 200, md: 220 }, breakpoint: 'sm' }
        }
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
            {isMobile && (
              <Burger
                opened={drawerOpened}
                onClick={() => setDrawerOpened((o) => !o)}
                size="sm"
                aria-label="Toggle navigation"
              />
            )}
            <IconBriefcase size={24} style={{ color: '#6366f1' }} />
            <div>
              <Title order={4} style={{ lineHeight: 1.2, marginBottom: 2 }}>
                Projex
              </Title>
              {!isMobile && (
                <Text size="xs" c="dimmed" style={{ lineHeight: 1 }}>
                  Project Management
                </Text>
              )}
            </div>
          </Group>
        </AppShell.Header>

        {!isMobile && (
          <AppShell.Navbar p="xs">
            {navLinks}
          </AppShell.Navbar>
        )}

        <AppShell.Main>
          <Outlet />
        </AppShell.Main>

        {syncEnabled && (
          <AppShell.Footer>
            <SyncStatusBar />
          </AppShell.Footer>
        )}
      </AppShell>
    </>
  );
}
