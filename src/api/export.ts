import { invokeCmd } from './invoke';

export interface ImportResult {
  persons: number;
  partners: number;
  projects: number;
  assignments: number;
  status_history: number;
  skipped_duplicates: number;
}

export interface WipeResult {
  wipe_id: string;
  deleted_project_comments: number;
  deleted_status_history: number;
  deleted_assignments: number;
  deleted_project_tags: number;
  deleted_projects: number;
  deleted_persons: number;
  deleted_partners: number;
}

export const exportApi = {
  exportJson: (req?: { schemaVersion?: number }) =>
    invokeCmd<string>('cmd_export_json', req ? { req } : {}),
  importJson: (json: string) =>
    invokeCmd<ImportResult>('cmd_import_json', { req: { json } }),
  wipeBusinessData: () => invokeCmd<WipeResult>('cmd_wipe_business_data'),
};
