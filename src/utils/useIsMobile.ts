import { useMediaQuery } from '@mantine/hooks';

/**
 * Returns true when the viewport width is below the 'sm' breakpoint (768px).
 * Use this to switch between mobile card view and desktop table view.
 */
export function useIsMobile(): boolean {
  return useMediaQuery('(max-width: 768px)') ?? false;
}
