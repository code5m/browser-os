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
