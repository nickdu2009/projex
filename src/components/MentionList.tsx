/**
 * Suggestion dropdown for @mention in the rich text editor.
 *
 * Renders a Mantine-styled popup with keyboard navigation (↑ ↓ Enter).
 * Used by the Tiptap Mention extension's `suggestion.render()` hook.
 */
import { forwardRef, useEffect, useImperativeHandle, useRef, useState, useCallback } from 'react';
import { Paper, Text, UnstyledButton } from '@mantine/core';
import { IconUser } from '@tabler/icons-react';

export interface MentionItem {
  id: string;
  label: string;
}

interface MentionListProps {
  items: MentionItem[];
  command: (item: MentionItem) => void;
}

export interface MentionListRef {
  onKeyDown: (props: { event: KeyboardEvent }) => boolean;
}

export const MentionList = forwardRef<MentionListRef, MentionListProps>(
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
      const el = container.querySelector<HTMLElement>('[data-mention-selected="true"]');
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
            No results
          </Text>
        </Paper>
      );
    }

    return (
      <Paper
        shadow="md"
        radius="md"
        withBorder
        onMouseDown={(e) => e.preventDefault()}
        style={{ overflow: 'hidden', minWidth: 180 }}
        ref={containerRef}
      >
        {items.map((item, index) => (
          <UnstyledButton
            key={item.id}
            data-mention-selected={index === selectedIndex ? 'true' : 'false'}
            onMouseDown={(e) => {
              e.preventDefault();
              selectItem(index);
            }}
            style={(theme) => ({
              display: 'flex',
              alignItems: 'center',
              gap: 8,
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
            <IconUser size={14} stroke={1.5} style={{ color: 'var(--mantine-color-indigo-5)', flexShrink: 0 }} />
            <Text size="sm" truncate>
              {item.label}
            </Text>
          </UnstyledButton>
        ))}
      </Paper>
    );
  },
);

MentionList.displayName = 'MentionList';
