import type { MantineColor } from '@mantine/core';

/**
 * 项目状态色彩映射
 * 统一管理项目状态的Badge颜色
 */
export const PROJECT_STATUS_COLORS: Record<string, MantineColor> = {
  Planning: 'gray',
  Active: 'green',
  'On Hold': 'yellow',
  Completed: 'blue',
  Cancelled: 'red',
};

/**
 * 根据项目状态获取对应颜色
 */
export function getProjectStatusColor(status: string): MantineColor {
  return PROJECT_STATUS_COLORS[status] || 'gray';
}
