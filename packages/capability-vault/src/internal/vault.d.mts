export function resolveNote(target: string, current: string, paths: string[]): string[];
export function noteLinks(text: string): string[];
export function searchNotes(notes: { path: string; text: string }[], query: string): { path: string; line: number; snippet: string }[];
