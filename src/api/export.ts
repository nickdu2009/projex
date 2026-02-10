import { invokeCmd } from './invoke';

export const exportApi = {
  exportJson: (req?: { schemaVersion?: number }) =>
    invokeCmd<string>('cmd_export_json', req ? { req } : {}),
};
