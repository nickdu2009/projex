import { Drawer } from '@mantine/core';

interface MobileBottomSheetProps {
  opened: boolean;
  onClose: () => void;
  title?: string;
  children: React.ReactNode;
}

/**
 * Bottom-sheet drawer for mobile filter panels and action menus.
 * Opens from the bottom on small screens; on larger screens it behaves
 * like a standard right-side drawer.
 */
export function MobileBottomSheet({ opened, onClose, title, children }: MobileBottomSheetProps) {
  return (
    <Drawer
      opened={opened}
      onClose={onClose}
      title={title}
      position="bottom"
      size="auto"
      styles={{
        content: {
          borderTopLeftRadius: 'var(--mantine-radius-lg)',
          borderTopRightRadius: 'var(--mantine-radius-lg)',
          maxHeight: '85dvh',
          overflowY: 'auto',
        },
      }}
    >
      {children}
    </Drawer>
  );
}
