import { Button, Group, Paper, Stack, Text, Title } from '@mantine/core';
import type { Icon } from '@tabler/icons-react';

interface EmptyStateProps {
  icon: Icon;
  title: string;
  description: string;
  actionLabel?: string;
  onAction?: () => void;
}

export function EmptyState({ icon: Icon, title, description, actionLabel, onAction }: EmptyStateProps) {
  return (
    <Paper p="xl" style={{ textAlign: 'center' }}>
      <Stack align="center" gap="md">
        <Icon size={64} stroke={1.5} style={{ color: 'var(--mantine-color-dimmed)' }} />
        <div>
          <Title order={4} c="dimmed" mb="xs">
            {title}
          </Title>
          <Text size="sm" c="dimmed">
            {description}
          </Text>
        </div>
        {actionLabel && onAction && (
          <Group>
            <Button variant="light" onClick={onAction}>
              {actionLabel}
            </Button>
          </Group>
        )}
      </Stack>
    </Paper>
  );
}
