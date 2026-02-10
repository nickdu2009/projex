import { AppShell, Group, NavLink, Title, Text } from '@mantine/core';
import {
  IconFolder,
  IconUsers,
  IconBuildingCommunity,
  IconSettings,
  IconBriefcase,
} from '@tabler/icons-react';
import { Outlet, useLocation, useNavigate } from 'react-router-dom';

const NAV = [
  { to: '/projects', label: '项目', icon: IconFolder },
  { to: '/people', label: '成员', icon: IconUsers },
  { to: '/partners', label: '合作方', icon: IconBuildingCommunity },
  { to: '/settings', label: '设置', icon: IconSettings },
];

export function Layout() {
  const navigate = useNavigate();
  const location = useLocation();

  return (
    <AppShell
      header={{ height: 56 }}
      navbar={{ width: { base: 200, md: 220 }, breakpoint: 'sm' }}
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
              项目管理
            </Title>
            <Text size="xs" c="dimmed" style={{ lineHeight: 1 }}>
              Project Manager
            </Text>
          </div>
        </Group>
      </AppShell.Header>
      <AppShell.Navbar p="xs">
        {NAV.map(({ to, label, icon: Icon }) => (
          <NavLink
            key={to}
            label={label}
            leftSection={<Icon size={20} stroke={1.5} />}
            active={location.pathname === to || location.pathname.startsWith(to + '/')}
            onClick={() => navigate(to)}
          />
        ))}
      </AppShell.Navbar>
      <AppShell.Main>
        <Outlet />
      </AppShell.Main>
    </AppShell>
  );
}
