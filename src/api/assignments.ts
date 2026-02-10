import { invokeCmd } from './invoke';

export interface AssignmentItem {
  id: string;
  project_id: string;
  person_id: string;
  person_name: string;
  role: string;
  start_at: string;
  end_at: string | null;
  created_at: string;
}

export const assignmentApi = {
  addMember: (req: {
    projectId: string;
    personId: string;
    role?: string;
    startAt?: string;
  }) => invokeCmd<unknown>('cmd_assignment_add_member', { req }),
  endMember: (req: { projectId: string; personId: string; endAt?: string }) =>
    invokeCmd<unknown>('cmd_assignment_end_member', { req }),
  listByProject: (projectId: string) =>
    invokeCmd<AssignmentItem[]>('cmd_assignment_list_by_project', { req: { projectId } }),
};
