import i18n from '../i18n';
import { PERSON_ROLES } from '../constants/countries';

/**
 * Resolve a role value to its translated label.
 * @param role Role value (e.g. 'backend_developer')
 * @returns Translated label (e.g. 'Backend Developer'), or the raw value if not found
 */
export function getRoleLabel(role: string): string {
  const found = PERSON_ROLES.find((r) => r.value === role);
  return found ? i18n.t(found.label) : role;
}
