import type { MantineColor } from '@mantine/core';
import type { TFunction } from 'i18next';

/**
 * Project status color mapping
 * Uses the actual status values from the backend
 */
export const PROJECT_STATUS_COLORS: Record<string, MantineColor> = {
  BACKLOG: 'gray',
  PLANNED: 'indigo',
  IN_PROGRESS: 'green',
  BLOCKED: 'yellow',
  DONE: 'blue',
  ARCHIVED: 'dimmed',
};

/**
 * Get the display color for a project status
 */
export function getProjectStatusColor(status: string): MantineColor {
  return PROJECT_STATUS_COLORS[status] || 'gray';
}

/**
 * Get the translated label for a project status
 * @param status Raw status value (e.g. 'IN_PROGRESS')
 * @param t i18next translation function
 * @returns Translated label (e.g. 'In Progress' or '进行中')
 */
export function getStatusLabel(status: string, t: TFunction): string {
  return t(`status.${status}`, status);
}
