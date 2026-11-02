/** IDEA-like Database Studio geometry. Persist UI state only, never SQL, credentials or URLs. */
export const KEY = 'browseros.database.idea-layout.v1';
export const DEFAULT_DOCK = Object.freeze({ side: 'right', navigator: 268, inspector: 258, output: 270, resultOpen: true, resultMaximized: false });
const finite = (value, fallback) => typeof value === 'number' && Number.isFinite(value) ? value : fallback;
export const clamp = (value, lower, upper) => Math.max(lower, Math.min(upper, value));
export function normalizeDock(input) {
  const x = input && typeof input === 'object' && !Array.isArray(input) ? input : {};
  return {
    side: x.side === 'left' ? 'left' : 'right',
    navigator: Math.round(clamp(finite(x.navigator, DEFAULT_DOCK.navigator), 200, 560)),
    inspector: Math.round(clamp(finite(x.inspector, DEFAULT_DOCK.inspector), 210, 500)),
    output: Math.round(clamp(finite(x.output, DEFAULT_DOCK.output), 130, 750)),
    resultOpen: typeof x.resultOpen === 'boolean' ? x.resultOpen : true,
    resultMaximized: typeof x.resultMaximized === 'boolean' ? x.resultMaximized : false,
  };
}
export function readDock(storage = typeof localStorage === 'undefined' ? null : localStorage) {
  try { const stored = storage?.getItem(KEY); return normalizeDock(stored ? JSON.parse(stored) : null); }
  catch { return normalizeDock(); }
}
export function writeDock(dock, storage = typeof localStorage === 'undefined' ? null : localStorage) {
  try { storage?.setItem(KEY, JSON.stringify(normalizeDock(dock))); return true; }
  catch { return false; }
}
export function resizedWidth(original, movement, side, containerWidth, otherWidth=0) {
  const direction = side === 'left' ? 1 : -1;
  const max = Math.max(200, Math.min(560, finite(containerWidth, 1024) - finite(otherWidth, 0) - 300));
  return Math.round(clamp(original + direction * movement, 200, max));
}
export function resizedOutput(original, movement, viewportHeight) {
  const max = Math.max(130, Math.min(750, finite(viewportHeight, 650) - 145));
  return Math.round(clamp(original - movement, 130, max));
}
/** UI-only result data representation, independent of SQL execution and transactions. */
export function markdownResult(columns, rows) {
  const escape = value => String(value ?? 'NULL').replaceAll('|', '\\|').replaceAll('\r', ' ').replaceAll('\n', ' ');
  return [ '| ' + columns.map(escape).join(' | ') + ' |', '| ' + columns.map(()=>'---').join(' | ') + ' |', ...rows.map(row => '| ' + row.map(escape).join(' | ') + ' |') ].join('\n');
}
