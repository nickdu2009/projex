import { invokeCmd } from './invoke';

export interface CommentDto {
  id: string;
  projectId: string;
  personId: string | null;
  personName: string | null;
  content: string;
  isPinned: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CommentCreateReq {
  projectId: string;
  personId?: string | null;
  content: string;
  isPinned?: boolean;
}

export interface CommentUpdateReq {
  id: string;
  content?: string;
  personId?: string | null;
  isPinned?: boolean;
}

export const commentApi = {
  list: (projectId: string) =>
    invokeCmd<CommentDto[]>('cmd_comment_list', { req: { projectId } }),

  create: (req: CommentCreateReq) =>
    invokeCmd<CommentDto>('cmd_comment_create', { req }),

  update: (req: CommentUpdateReq) =>
    invokeCmd<CommentDto>('cmd_comment_update', { req }),

  delete: (id: string) =>
    invokeCmd<void>('cmd_comment_delete', { req: { id } }),
};
