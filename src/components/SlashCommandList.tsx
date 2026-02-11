/**
 * Slash command dropdown menu for the "/" shortcut in the rich text editor.
 *
 * Renders a Notion-style command palette with keyboard navigation (↑ ↓ Enter).
 */
import { forwardRef, useEffect, useImperativeHandle, useRef, useState, useCallback } from 'react';
import { Group, Paper, Text, UnstyledButton } from '@mantine/core';
import type { FC } from 'react';

export interface SlashCommandItem {
  id: string;
  label: string;
  description: string;
  icon: FC<{ size?: number; stroke?: number; style?: React.CSSProperties }>;
  /** Executed when the user selects this command */
  action: () => void;
}

interface SlashCommandListProps {
  items: SlashCommandItem[];
  command: (item: SlashCommandItem) => void;
}

export interface SlashCommandListRef {
  onKeyDown: (props: { event: KeyboardEvent }) => boolean;
}

export const SlashCommandList = forwardRef<SlashCommandListRef, SlashCommandListProps>(
  ({ items, command }, ref) => {
    const [selectedIndex, setSelectedIndex] = useState(0);
    const containerRef = useRef<HTMLDivElement | null>(null);
    const prevItemsRef = useRef(items);

    // Reset selection when items change
    // (Adjusting state based on props during render – React recommended pattern,
    //  see https://react.dev/learn/you-might-not-need-an-effect#adjusting-some-state-when-a-prop-changes)
    // eslint-disable-next-line react-hooks/refs
    if (prevItemsRef.current !== items) {
      prevItemsRef.current = items; // eslint-disable-line react-hooks/refs
      if (selectedIndex !== 0) {
        setSelectedIndex(0);
      }
    }

    // Keep the selected item visible when navigating with keyboard
    const scrollSelected = useCallback(() => {
      const container = containerRef.current;
      if (!container) return;
      const el = container.querySelector<HTMLElement>('[data-slash-selected="true"]');
      el?.scrollIntoView({ block: 'nearest' });
    }, []);

    useEffect(() => {
      scrollSelected();
    }, [selectedIndex, scrollSelected]);

    const selectItem = (index: number) => {
      const item = items[index];
      if (item) {
        command(item);
      }
    };

    useImperativeHandle(ref, () => ({
      onKeyDown: ({ event }: { event: KeyboardEvent }) => {
        if (event.key === 'ArrowUp') {
          setSelectedIndex((prev) => (prev + items.length - 1) % items.length);
          return true;
        }
        if (event.key === 'ArrowDown') {
          setSelectedIndex((prev) => (prev + 1) % items.length);
          return true;
        }
        if (event.key === 'Enter') {
          selectItem(selectedIndex);
          return true;
        }
        return false;
      },
    }));

    if (items.length === 0) {
      return (
        <Paper shadow="md" radius="md" p="xs" withBorder>
          <Text size="xs" c="dimmed" ta="center" py={4}>
            No matching commands
          </Text>
        </Paper>
      );
    }

    return (
      <Paper
        shadow="md"
        radius="md"
        withBorder
        // Prevent mousedown from blurring the editor, which would trigger
        // suggestion onExit and destroy the popup before click fires.
        onMouseDown={(e) => e.preventDefault()}
        style={{ minWidth: 240, maxHeight: 320, overflowY: 'auto' }}
        ref={containerRef}
      >
        {items.map((item, index) => {
          const Icon = item.icon;
          return (
            <UnstyledButton
              key={item.id}
              data-slash-selected={index === selectedIndex ? 'true' : 'false'}
              onMouseDown={(e) => {
                e.preventDefault();
                selectItem(index);
              }}
              style={(theme) => ({
                display: 'flex',
                alignItems: 'center',
                gap: 10,
                width: '100%',
                padding: '8px 12px',
                fontSize: theme.fontSizes.sm,
                backgroundColor:
                  index === selectedIndex
                    ? 'var(--mantine-color-indigo-0)'
                    : 'transparent',
                '&:hover': {
                  backgroundColor: 'var(--mantine-color-gray-0)',
                },
              })}
            >
              <Group
                style={{
                  width: 28,
                  height: 28,
                  borderRadius: 6,
                  backgroundColor: 'var(--mantine-color-indigo-0)',
                  flexShrink: 0,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                }}
              >
                <Icon size={16} stroke={1.5} style={{ color: 'var(--mantine-color-indigo-6)' }} />
              </Group>
              <div style={{ flex: 1, minWidth: 0 }}>
                <Text size="sm" fw={500} truncate>
                  {item.label}
                </Text>
                <Text size="xs" c="dimmed" truncate>
                  {item.description}
                </Text>
              </div>
            </UnstyledButton>
          );
        })}
      </Paper>
    );
  },
);

SlashCommandList.displayName = 'SlashCommandList';
