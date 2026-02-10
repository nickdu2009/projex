import { PERSON_ROLES } from '../constants/countries';

/**
 * 将角色英文值转换为中文标签
 * @param role 角色英文值（如 'backend_developer'）
 * @returns 中文标签（如 '后端开发'），如果未找到则返回原值
 */
export function getRoleLabel(role: string): string {
  const found = PERSON_ROLES.find((r) => r.value === role);
  return found ? found.label : role;
}
