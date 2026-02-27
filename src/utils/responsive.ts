/**
 * Shared responsive helpers.
 * Keeps breakpoint-related constants in one place so pages stay consistent.
 */

/** Minimum touch target size in px (WCAG 2.5.5 recommended). */
export const MIN_TOUCH_TARGET = 44;

/** Standard mobile padding (xs = 8px). */
export const MOBILE_PADDING = 'xs' as const;

/** Standard desktop padding (md = 16px). */
export const DESKTOP_PADDING = 'md' as const;

/** Responsive padding value for AppShell / page containers. */
export const PAGE_PADDING = { base: MOBILE_PADDING, sm: DESKTOP_PADDING } as const;

/** Responsive cols for two-column form grids. */
export const FORM_GRID_COLS = { base: 1, sm: 2 } as const;

/** Responsive cols for three-column info grids. */
export const INFO_GRID_COLS = { base: 1, sm: 3 } as const;
