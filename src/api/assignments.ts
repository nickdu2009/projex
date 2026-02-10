import { invokeCmd } from './invoke';

export const assignmentApi = {
  addMember: (req: {
    projectId: string;
    personId: string;
    role?: string;
    startAt?: string;
  }) => invokeCmd<unknown>('cmd_assignment_add_member', { req }),
  endMember: (req: { projectId: string; personId: string; endAt?: string }) =>
    invokeCmd<unknown>('cmd_assignment_end_member', { req }),
};
