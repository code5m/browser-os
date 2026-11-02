export type GitPreferences = {
  logPlacement: 'editor' | 'toolwindow';
  writeIdeaMetadata: boolean;
  indexEnabled: boolean;
};

const key = 'browseros.git-studio.preferences';

export function readGitPreferences(): Partial<GitPreferences> {
  if (typeof localStorage === 'undefined') return {};
  try { return JSON.parse(localStorage.getItem(key) || '{}'); }
  catch { return {}; }
}

export function writeGitPreferences(value: GitPreferences): void {
  if (typeof localStorage !== 'undefined') localStorage.setItem(key, JSON.stringify(value));
}

// UI-only fold flags share the existing Git preferences adapter. No repo data
// or model credentials are persisted, and the Git component never accesses storage.
export type GitDisclosure = 'toolbar' | 'files' | 'inspector';
export function readGitDisclosure(view: GitDisclosure): boolean {
  if (typeof localStorage === 'undefined') return true;
  try { return localStorage.getItem(`browseros.git.ui.${view}`) !== 'false'; }
  catch { return true; }
}
export function writeGitDisclosure(view: GitDisclosure, expanded: boolean): void {
  if (typeof localStorage === 'undefined') return;
  try { localStorage.setItem(`browseros.git.ui.${view}`,String(expanded)); }
  catch { /* restricted storage: keep current in-memory visibility */ }
}
