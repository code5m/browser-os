// packages/capability-vault/src/internal/vault.mjs
// Vault 能力**内部**领域逻辑（Frontend M2 —— 独立 capability package 的 PRIVATE INTERNAL）。
//
// 语义 owner = vault 能力：noteLinks / resolveNote / searchNotes 是笔记链接图、
// 链接解析与全文搜索的**唯一实现**，不存在第二份。
//
// PRIVATE INTERNAL（禁止外部直接 import）：
//   仅本包内的 state/useVaultStore.ts 可引用；
//   包外部（含 App.vue / shell / 其它 capability）必须经包 public 契约（index.ts），
//   不得越过 exports 直接引用本文件（PKG-04 / PKG-05）。
export function resolveNote(target, current, paths) {
  let decoded;
  try { decoded = decodeURIComponent(target.split('|')[0]); } catch { return []; }
  if (/^[a-z][a-z\d+.-]*:/i.test(decoded) || decoded.startsWith('/')) return [];
  const file = decoded.split('#')[0];
  if (!file) return paths.includes(current) ? [current] : [];
  const normalize = path => {
    const parts = [];
    for (const part of path.split('/')) {
      if (part === '..') { if (!parts.length) return ''; parts.pop(); }
      else if (part && part !== '.') parts.push(part);
    }
    return parts.join('/');
  };
  const suffix = /\.md$/i.test(file) ? file : file + '.md';
  const relative = normalize(current.split('/').slice(0, -1).concat(suffix).join('/'));
  if (paths.includes(relative)) return [relative];
  const rooted = normalize(suffix);
  if (paths.includes(rooted)) return [rooted];
  if (file.includes('/')) return [];
  return paths.filter(path => path.split('/').pop() === suffix);
}

export function noteLinks(text) {
  const links = new Set();
  let inCode = false;
  for (const line of text.replace(/\r\n/g, '\n').split('\n')) {
    if (line.trimStart().startsWith('```')) { inCode = !inCode; continue; }
    if (inCode) continue;
    const visible = line.replace(/`[^`]*`/g, '');
    for (const match of visible.matchAll(/!?\[[^\]]*\]\(([^)\s]+)(?:\s+['"][^'"]*['"])?\)/g)) links.add(match[1]);
    for (const match of visible.matchAll(/\[\[([^\]\n]+)\]\]/g)) links.add(match[1].split('|')[0]);
  }
  return [...links].slice(0, 256);
}

export function searchNotes(notes, query) {
  const q = query.trim().toLocaleLowerCase();
  if (!q) return notes.map(n => ({ path: n.path, line: 1, snippet: '' }));
  const results = [];
  for (const note of notes) {
    const offset = note.text.toLocaleLowerCase().indexOf(q);
    if (offset >= 0 || note.path.toLocaleLowerCase().includes(q)) {
      const line = offset < 0 ? 1 : note.text.slice(0, offset).split('\n').length;
      results.push({ path: note.path, line, snippet: note.text.split('\n')[line - 1].slice(0, 180) });
    }
    if (results.length === 100) break;
  }
  return results;
}
