import { ActionIcon, Flex, Group, Title } from '@mantine/core';
import { IconArrowLeft } from '@tabler/icons-react';

interface MobilePageHeaderProps {
  title: string;
  onBack?: () => void;
  actions?: React.ReactNode;
}

/**
 * Unified page header for detail / form pages.
 * On mobile: full-width row with back button, title and optional action slot.
 * On desktop: degrades to a simple flex row (same visual as before).
 */
export function MobilePageHeader({ title, onBack, actions }: MobilePageHeaderProps) {
  return (
    <Flex
      wrap="wrap"
      gap="xs"
      justify="space-between"
      align="center"
      w="100%"
      style={{ minWidth: 0 }}
    >
      <Group gap="xs" style={{ minWidth: 0, flex: 1 }}>
        {onBack && (
          <ActionIcon
            variant="subtle"
            size="lg"
            onClick={onBack}
            aria-label="back"
            style={{ flexShrink: 0 }}
          >
            <IconArrowLeft size={20} />
          </ActionIcon>
        )}
        <Title
          order={3}
          style={{
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            minWidth: 0,
          }}
        >
          {title}
        </Title>
      </Group>
      {actions && (
        <Group gap="xs" wrap="wrap" style={{ flexShrink: 0 }}>
          {actions}
        </Group>
      )}
    </Flex>
  );
}
