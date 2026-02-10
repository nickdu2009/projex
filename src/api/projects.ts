import { invokeCmd } from './invoke';

export interface ProjectListItem {
  id: string;
  name: string;
  current_status: string;
  priority: number;
  country_code: string;
  partner_name: string;
  owner_name: string;
  due_date: string | null;
  updated_at: string;
  tags: string[];
}

export interface ProjectListPage {
  items: ProjectListItem[];
  total: number;
  limit: number;
  offset: number;
}

export interface ProjectListReq {
  onlyUnarchived?: boolean;
  statuses?: string[];
  countryCodes?: string[];
  partnerIds?: string[];
  ownerPersonIds?: string[];
  participantPersonIds?: string[];
  tags?: string[];
  sortBy?: string;
  sortOrder?: string;
  limit?: number;
  offset?: number;
}

export interface ProjectDetail {
  id: string;
  name: string;
  description: string;
  priority: number;
  current_status: string;
  country_code: string;
  partner_id: string;
  owner_person_id: string;
  start_date: string | null;
  due_date: string | null;
  created_at: string;
  updated_at: string;
  archived_at: string | null;
  tags: string[];
  owner_name: string;
  partner_name: string;
  assignments: AssignmentDto[];
  status_history: StatusHistoryDto[];
}

export interface AssignmentDto {
  id: string;
  project_id: string;
  person_id: string;
  person_name: string;
  role: string;
  start_at: string;
  end_at: string | null;
  created_at: string;
}

export interface StatusHistoryDto {
  id: string;
  project_id: string;
  from_status: string | null;
  to_status: string;
  changed_at: string;
  changed_by_person_id: string | null;
  changed_by_name: string | null;
  note: string;
}

export const projectApi = {
  list: (req?: ProjectListReq) =>
    invokeCmd<ProjectListPage>('cmd_project_list', req ? { req } : {}),
  get: (id: string) => invokeCmd<ProjectDetail>('cmd_project_get', { req: { id } }),
  create: (req: {
    name: string;
    countryCode: string;
    partnerId: string;
    ownerPersonId: string;
    description?: string;
    priority?: number;
    startDate?: string;
    dueDate?: string;
    tags?: string[];
  }) => invokeCmd<ProjectDetail>('cmd_project_create', { req }),
  update: (req: {
    id: string;
    name?: string;
    description?: string;
    priority?: number;
    countryCode?: string;
    ownerPersonId?: string;
    startDate?: string | null;
    dueDate?: string | null;
    tags?: string[];
  }) => invokeCmd<ProjectDetail>('cmd_project_update', { req }),
  changeStatus: (req: {
    projectId: string;
    toStatus: string;
    note?: string;
    changedByPersonId?: string | null;
  }) => invokeCmd<ProjectDetail>('cmd_project_change_status', { req }),
};
