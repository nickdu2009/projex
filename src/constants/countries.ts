import countries from 'i18n-iso-countries';
import enLocale from 'i18n-iso-countries/langs/en.json';
import zhLocale from 'i18n-iso-countries/langs/zh.json';

countries.registerLocale(enLocale as countries.LocaleData);
countries.registerLocale(zhLocale as countries.LocaleData);

/** East & Southeast Asia ISO 3166-1 alpha-2, order: East Asia first, then Southeast Asia */
const EAST_AND_SOUTHEAST_ASIA_CODES = [
  'CN', 'HK', 'MO', 'TW', 'JP', 'KR', 'KP', 'MN', // East Asia
  'BN', 'KH', 'ID', 'LA', 'MY', 'MM', 'PH', 'SG', 'TH', 'TL', 'VN', // Southeast Asia
] as const;

/**
 * Get country list in the specified language.
 * @param lng Language code ('en' | 'zh')
 * @returns Country list with code and localized name
 */
export function getCountries(lng: string): { code: string; name: string }[] {
  const locale = lng.startsWith('zh') ? 'zh' : 'en';
  const names = countries.getNames(locale) as Record<string, string>;
  return EAST_AND_SOUTHEAST_ASIA_CODES
    .filter((code) => names[code])
    .map((code) => ({ code, name: names[code]! }));
}

export const PROJECT_STATUSES = [
  'BACKLOG',
  'PLANNED',
  'IN_PROGRESS',
  'BLOCKED',
  'DONE',
  'ARCHIVED',
] as const;

/** Person role presets — labels are i18n keys, resolved at render time via t() */
export const PERSON_ROLES = [
  { value: 'tester', label: 'role.tester' },
  { value: 'product_manager', label: 'role.product_manager' },
  { value: 'backend_developer', label: 'role.backend_developer' },
  { value: 'frontend_developer', label: 'role.frontend_developer' },
] as const;
