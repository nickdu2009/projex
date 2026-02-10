import { invokeCmd } from './invoke';

export interface ImportResult {
  persons: number;
  partners: number;
  projects: number;
  assignments: number;
  status_history: number;
  skipped_duplicates: number;
}

export const exportApi = {
  exportJson: (req?: { schemaVersion?: number }) =>
    invokeCmd<string>('cmd_export_json', req ? { req } : {}),
  importJson: (json: string) =>
    invokeCmd<ImportResult>('cmd_import_json', { req: { json } }),
};
