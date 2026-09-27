// src/capabilities/vault/internal/vault.mjs
// Vault 能力**内部**领域逻辑（Frontend M2 Package Pilot — 物理下沉至能力包）。
//
// 语义 owner = vault 能力：noteLinks / resolveNote / searchNotes 是笔记链接图、
// 链接解析与全文搜索的**唯一实现**，不存在第二份。
//
// 变更前：本逻辑位于 src/utils/vault.mjs —— 能力域逻辑散落在能力包之外
//   （能力外部无法从图书/权限角度识别它是 vault 私有实现）。
// 变更后：下沉至 capabilities/vault/internal/，**逻辑逐字节不变**，仅归属与路径修正。
//
// PRIVATE INTERNAL（禁止外部直接 import）：
//   仅本能力内的 state/useVaultStore.ts 可引用；
//   能力外部（含 App.vue / shell / 其它 capability）必须经 public.ts 暴露的语义，
//   不得越过 public.ts 直接引用本文件。
import { marked } from 'marked';

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
  const relative = normalize(current.split('/').slice(0,-1).concat(suffix).join('/'));
  if (paths.includes(relative)) return [relative];
  const rooted = normalize(suffix);
  if (paths.includes(rooted)) return [rooted];
  if (file.includes('/')) return [];
  return paths.filter(path => path.split('/').pop() === suffix);
}

export function noteLinks(text) {
  const links = new Set();
  marked.walkTokens(marked.lexer(text), token => {
    if (token.type === 'link') links.add(token.href);
    if (token.type === 'text') {
      for (const match of token.text.matchAll(/\[\[([^\]\n]+)\]\]/g)) links.add(match[1].split('|')[0]);
    }
  });
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
      results.push({ path: note.path, line, snippet: note.text.split('\n')[line-1].slice(0,180) });
    }
    if (results.length === 100) break;
  }
  return results;
}
