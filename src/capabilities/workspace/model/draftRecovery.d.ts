export interface PersistDraftAfterSaveOptions {
  path: string;
  currentSessionId: string;
  recoveredSessionId?: string;
  savedText: string;
  currentText: string;
  save: (path: string, sessionId: string, baseline: string, content: string) => Promise<void>;
  clear: (path: string, sessionId: string) => Promise<void>;
}
export function persistDraftAfterSave(options: PersistDraftAfterSaveOptions): Promise<void>;
