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
