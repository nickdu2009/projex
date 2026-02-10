import { invokeCmd } from './invoke';

export interface PartnerDto {
  id: string;
  name: string;
  note: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface PartnerProjectItem {
  id: string;
  name: string;
  current_status: string;
  updated_at: string;
}

export const partnersApi = {
  list: (onlyActive = true) =>
    invokeCmd<PartnerDto[]>('cmd_partner_list', { req: { onlyActive } }),
  get: (id: string) => invokeCmd<PartnerDto>('cmd_partner_get', { req: { id } }),
  create: (req: { name: string; note?: string }) =>
    invokeCmd<PartnerDto>('cmd_partner_create', { req }),
  update: (req: { id: string; name?: string; note?: string }) =>
    invokeCmd<PartnerDto>('cmd_partner_update', { req }),
  deactivate: (id: string) =>
    invokeCmd<PartnerDto>('cmd_partner_deactivate', { req: { id } }),
  projects: (partnerId: string) =>
    invokeCmd<PartnerProjectItem[]>('cmd_partner_projects', { req: { id: partnerId } }),
};
