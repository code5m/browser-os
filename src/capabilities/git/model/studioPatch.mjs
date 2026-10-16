/** Exact single-file patch selection. Paths never become shell arguments here. */
export function patchHunks(patch) {
  if (new TextEncoder().encode(patch).length > 262144) throw new Error('GIT_PATCH_LIMIT');
  const lines = patch.split('\n'), hunks = []; let current = null, files = 0;
  for (let lineNumber = 0; lineNumber < lines.length; lineNumber++) {
    const line = lines[lineNumber];
    if (line.startsWith('diff --git ') && ++files > 1) throw new Error('GIT_SELECT_ONE_FILE');
    const match = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/.exec(line);
    if (match) { current = { id: hunks.length, oldStart: Number(match[1]), oldCount: match[2] == null ? 1 : Number(match[2]), newStart: Number(match[3]), lines: [] }; hunks.push(current); continue; }
    if (!current) continue;
    if (line.startsWith('\\ No newline')) { if (current.lines.length) current.lines.at(-1).eol = ''; continue; }
    if (/^[ +\-]/.test(line)) current.lines.push({ id: lineNumber, sign: line[0], text: line.slice(1), eol: '\n' });
  }
  return hunks;
}
const textLines = text => text.match(/[^\n]*\n|[^\n]+$/g) ?? [];
export function applyPatchSelection(original, patch, selectedLineIds) {
  const selected = new Set(selectedLineIds), source = textLines(original), output = [];
  let cursor = 0;
  for (const hunk of patchHunks(patch)) {
    const start = hunk.oldStart === 0 ? 0 : hunk.oldStart - 1;
    if (start < cursor || start > source.length) throw new Error('GIT_PATCH_POSITION_MISMATCH');
    output.push(...source.slice(cursor, start)); cursor = start; let consumed = 0;
    for (const line of hunk.lines) {
      const text = line.text + line.eol;
      if (line.sign !== '+') { if (source[cursor] !== text) throw new Error('GIT_PATCH_BASE_CHANGED'); cursor++; consumed++; }
      if (line.sign === ' ' || (line.sign === '-' && !selected.has(line.id)) || (line.sign === '+' && selected.has(line.id))) output.push(text);
    }
    if (consumed !== hunk.oldCount) throw new Error('GIT_PATCH_TRUNCATED');
  }
  output.push(...source.slice(cursor)); return output.join('');
}
export function blameLines(text) {
  const lines = text.split('\n'), result = []; let item = null;
  for (const line of lines) {
    const match = /^([0-9a-f]{40,64})\s+(\d+)\s+(\d+)(?:\s+\d+)?$/.exec(line);
    if (match) item = { oid:match[1], original:Number(match[2]), line:Number(match[3]), author:'', summary:'', text:'' };
    else if (item && line.startsWith('author ')) item.author = line.slice(7);
    else if (item && line.startsWith('summary ')) item.summary = line.slice(8);
    else if (item && line.startsWith('\t')) { result.push({...item,text:line.slice(1)}); item = null; }
    if (result.length >= 4000) break;
  }
  return result;
}
export function sourceIdentity(source) {
  if (!!source?.path === !!source?.repo_id) throw new Error('GIT_STUDIO_EXACTLY_ONE_SOURCE');
  return JSON.stringify(source.path ? {path:source.path} : {repo_id:source.repo_id});
}
export function safeGithubLink(value) {
  try { const url = new URL(String(value)); return url.protocol === 'https:' && url.hostname === 'github.com' && !url.username && !url.password ? url.href : ''; } catch { return ''; }
}
