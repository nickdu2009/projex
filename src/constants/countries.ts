import countries from 'i18n-iso-countries';
import zhLocale from 'i18n-iso-countries/langs/zh.json';

countries.registerLocale(zhLocale as countries.LocaleData);

/** 东亚 + 东南亚 ISO 3166-1 alpha-2，顺序：东亚优先，再东南亚 */
const EAST_AND_SOUTHEAST_ASIA_CODES = [
  'CN', 'HK', 'MO', 'TW', 'JP', 'KR', 'KP', 'MN', // 东亚
  'BN', 'KH', 'ID', 'LA', 'MY', 'MM', 'PH', 'SG', 'TH', 'TL', 'VN', // 东南亚
] as const;

const names = countries.getNames('zh') as Record<string, string>;

/** 国家列表：仅东亚与东南亚，中文名来自 i18n-iso-countries */
export const COUNTRIES: { code: string; name: string }[] = EAST_AND_SOUTHEAST_ASIA_CODES
  .filter((code) => names[code])
  .map((code) => ({ code, name: names[code]! }));

export const PROJECT_STATUSES = [
  'BACKLOG',
  'PLANNED',
  'IN_PROGRESS',
  'BLOCKED',
  'DONE',
  'ARCHIVED',
] as const;

/** 人员角色预制选项 */
export const PERSON_ROLES = [
  { value: 'tester', label: '测试' },
  { value: 'product_manager', label: '产品经理' },
  { value: 'backend_developer', label: '后端开发' },
  { value: 'frontend_developer', label: '前端开发' },
] as const;
