import { invokeCmd } from './invoke';

export interface PersonDto {
  id: string;
  display_name: string;
  email: string;
  role: string;
  note: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface PersonProjectItem {
  id: string;
  name: string;
  current_status: string;
  updated_at: string;
  last_involved_at?: string | null;
}

export const peopleApi = {
  list: (onlyActive = true) =>
    invokeCmd<PersonDto[]>('cmd_person_list', { req: { onlyActive } }),
  get: (id: string) => invokeCmd<PersonDto>('cmd_person_get', { req: { id } }),
  create: (req: { displayName: string; email?: string; role?: string; note?: string }) =>
    invokeCmd<PersonDto>('cmd_person_create', { req }),
  update: (req: { id: string; displayName?: string; email?: string; role?: string; note?: string }) =>
    invokeCmd<PersonDto>('cmd_person_update', { req }),
  deactivate: (id: string) =>
    invokeCmd<PersonDto>('cmd_person_deactivate', { req: { id } }),
  currentProjects: (personId: string) =>
    invokeCmd<PersonProjectItem[]>('cmd_person_current_projects', { req: { id: personId } }),
  allProjects: (personId: string) =>
    invokeCmd<PersonProjectItem[]>('cmd_person_all_projects', { req: { id: personId } }),
};
