/** Bounded SQL editing helpers. Native policy remains the sole execution gate. */
export const KEYWORDS = new Set(('SELECT FROM WHERE JOIN LEFT RIGHT INNER OUTER FULL ON AS AND OR NOT NULL IS IN EXISTS BETWEEN LIKE ILIKE ORDER BY GROUP HAVING LIMIT OFFSET ASC DESC DISTINCT UNION ALL WITH RECURSIVE INSERT INTO VALUES UPDATE SET DELETE RETURNING CREATE ALTER DROP TABLE VIEW INDEX PRIMARY KEY FOREIGN REFERENCES DEFAULT CASE WHEN THEN ELSE END EXPLAIN BEGIN COMMIT ROLLBACK TRUE FALSE COUNT SUM AVG MIN MAX OVER PARTITION').split(' '));
const bytes = text => new TextEncoder().encode(text).length;
export function lexSql(text) {
  if (bytes(text) > 65536) throw new Error('SQL_TEXT_TOO_LARGE');
  const tokens = [], issues = [], stack = []; let i = 0;
  const add = (start, kind) => tokens.push({ start, end: i, kind, text: text.slice(start, i) });
  while (i < text.length) {
    const start = i, c = text[i];
    if (/\s/.test(c)) { while (i < text.length && /\s/.test(text[i])) i++; add(start, 'space'); continue; }
    if (text.startsWith('--', i)) { while (i < text.length && text[i] !== '\n') i++; add(start, 'comment'); continue; }
    if (text.startsWith('/*', i)) {
      let depth = 1; i += 2;
      while (i < text.length && depth) {
        if (text.startsWith('/*', i)) { depth++; i += 2; }
        else if (text.startsWith('*/', i)) { depth--; i += 2; }
        else i++;
      }
      if (depth) issues.push({ offset: start, code: 'UNCLOSED_COMMENT' }); add(start, 'comment'); continue;
    }
    const dollar = c === '$' ? /^\$(?:[A-Za-z_][A-Za-z_0-9]*)?\$/.exec(text.slice(i)) : null;
    if (dollar) {
      i += dollar[0].length; const end = text.indexOf(dollar[0], i);
      if (end < 0) { i = text.length; issues.push({ offset: start, code: 'UNCLOSED_STRING' }); }
      else i = end + dollar[0].length;
      add(start, 'string'); continue;
    }
    if (c === "'" || c === '"' || c === '`' || c === '[') {
      const close = c === '[' ? ']' : c; let closed = false; i++;
      while (i < text.length) {
        if (text[i] === close) { if (text[i + 1] === close) { i += 2; continue; } i++; closed = true; break; }
        if (text[i] === '\\' && i + 1 < text.length) i += 2; else i++;
      }
      if (!closed) issues.push({ offset: start, code: 'UNCLOSED_STRING' }); add(start, c === "'" ? 'string' : 'identifier'); continue;
    }
    if (/[A-Za-z_\u0080-\uffff]/.test(c)) {
      i++; while (i < text.length && /[A-Za-z_0-9$\u0080-\uffff]/.test(text[i])) i++;
      add(start, KEYWORDS.has(text.slice(start, i).toUpperCase()) ? 'keyword' : 'identifier'); continue;
    }
    if (/[0-9]/.test(c)) {
      const match = /^(?:0x[0-9a-f]+|\d+(?:\.\d*)?(?:e[+-]?\d+)?)/i.exec(text.slice(i));
      i += match ? match[0].length : 1; add(start, 'number'); continue;
    }
    if (c === '(') stack.push(i);
    if (c === ')') { if (stack.length) stack.pop(); else issues.push({ offset: i, code: 'UNMATCHED_CLOSE' }); }
    i++; add(start, c === ';' ? 'separator' : 'punctuation');
  }
  for (const offset of stack) issues.push({ offset, code: 'UNCLOSED_PAREN' });
  return { tokens, issues };
}
export function sqlStatements(text) {
  const { tokens, issues } = lexSql(text); if (issues.length) throw new Error(issues[0].code);
  const result = []; let start = 0, meaningful = false;
  for (const token of tokens) {
    if (token.kind === 'separator') {
      if (meaningful) result.push({ start, end: token.end, text: text.slice(start, token.end).trim() });
      start = token.end; meaningful = false;
    } else if (token.kind !== 'comment' && token.kind !== 'space') meaningful = true;
  }
  if (meaningful) result.push({ start, end: text.length, text: text.slice(start).trim() });
  if (result.length > 32) throw new Error('SQL_STATEMENT_LIMIT');
  return result;
}
export function selectedSql(text, start, end = start) {
  if (!Number.isInteger(start) || !Number.isInteger(end) || start < 0 || end < start || end > text.length) throw new Error('SQL_SELECTION_INVALID');
  if (end > start) { const selected = text.slice(start, end).trim(); sqlStatements(selected); return selected; }
  const statements = sqlStatements(text);
  return (statements.find(s => start >= s.start && start < s.end) ?? statements.at(-1))?.text ?? '';
}
/** Preserve all token bytes AND original adjacency; never split ::, ->>, E'', etc. */
export function formatSql(text) {
  const { tokens, issues } = lexSql(text); if (issues.length) throw new Error(issues[0].code);
  if (tokens.some(t => t.kind === 'comment') || /\b(?:DELIMITER|CREATE\s+(?:FUNCTION|PROCEDURE|TRIGGER))\b/i.test(text)) return text;
  let out = '', depth = 0, previousEnd = 0;
  const clauses = new Set(['SELECT','FROM','WHERE','HAVING','LIMIT','OFFSET','RETURNING','UNION','LEFT','RIGHT','INNER','JOIN','ORDER','GROUP','SET','VALUES']);
  for (const token of tokens.filter(t => t.kind !== 'space')) {
    if (token.text === ')') depth = Math.max(0, depth - 1);
    const gap = text.slice(previousEnd, token.start);
    const newline = token.kind === 'keyword' && depth === 0 && clauses.has(token.text.toUpperCase()) && /\s/.test(gap);
    if (newline && out && !out.endsWith('\n')) out = out.trimEnd() + '\n';
    else if (out && !out.endsWith('\n') && /\s/.test(gap)) out += ' ';
    out += token.text; previousEnd = token.end;
    if (token.text === '(') depth++;
    if (token.text === ';') out += '\n';
  }
  const formatted = out.trim();
  const signature = s => lexSql(s).tokens.filter(t => t.kind !== 'space').map(t => [t.kind, t.text]);
  return JSON.stringify(signature(text)) === JSON.stringify(signature(formatted)) ? formatted : text;
}
export function sqlCompletions(text, cursor, objects = [], columns = []) {
  const prefix = /[A-Za-z_\u0080-\uffff][A-Za-z_0-9\u0080-\uffff]*$/.exec(text.slice(0, cursor))?.[0] ?? '';
  const token = lexSql(text).tokens.find(t => t.start < cursor && t.end >= cursor);
  if (token && ['string', 'comment'].includes(token.kind)) return [];
  return [...new Set([...columns, ...objects, ...KEYWORDS])].filter(x => typeof x === 'string' && x.length <= 128 && x.toLowerCase().startsWith(prefix.toLowerCase()))
    .slice(0, 40).map(label => ({ label, from: cursor - prefix.length, to: cursor }));
}
export function quoteIdentifier(name, kind) {
  if (typeof name !== 'string' || !name || bytes(name) > 255 || /[\0\r\n]/.test(name)) throw new Error('DB_IDENTIFIER_INVALID');
  const quote = kind === 'mysql' ? '`' : '"'; return quote + name.replaceAll(quote, quote + quote) + quote;
}
export function exportRows(columns, rows, format = 'csv') {
  if (!Array.isArray(columns) || columns.length > 512 || rows.length > 1000 || !['csv','tsv','json'].includes(format)) throw new Error('DB_EXPORT_LIMIT');
  const delimiter = format === 'tsv' ? '\t' : ',';
  const escape = value => {
    let text = value == null ? '' : typeof value === 'object' ? JSON.stringify(value) : String(value);
    if (/^[\s]*[=+@-]/.test(text) && !/^-?\d+(?:\.\d+)?$/.test(text)) text = "'" + text;
    return text.includes(delimiter) || /["\r\n]/.test(text) ? '"' + text.replaceAll('"','""') + '"' : text;
  };
  const output = format === 'json'
    ? JSON.stringify({ columns, rows }, null, 2)
    : [columns,...rows].map(row => row.map(escape).join(delimiter)).join('\r\n');
  if (bytes(output) > 5 * 1024 * 1024) throw new Error('DB_EXPORT_LIMIT'); return output;
}
export function parseDelimited(text, delimiter = ',') {
  if (bytes(text) > 5 * 1024 * 1024 || ![',','\t'].includes(delimiter)) throw new Error('DB_IMPORT_LIMIT');
  text = text.replace(/^\uFEFF/, '');
  const rows = []; let row = [], field = '', quoted = false, afterQuote = false;
  const push = () => { row.push(field); field = ''; afterQuote = false; if (row.length > 512) throw new Error('DB_IMPORT_COLUMN_LIMIT'); };
  const finish = () => { push(); rows.push(row); row = []; if (rows.length > 1001) throw new Error('DB_IMPORT_ROW_LIMIT'); };
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (quoted) { if (c === '"') { if (text[i+1] === '"') { field += '"'; i++; } else { quoted = false; afterQuote = true; } } else field += c; }
    else if (c === delimiter) push();
    else if (c === '\n' || c === '\r') { if (c === '\r' && text[i+1] === '\n') i++; finish(); }
    else if (c === '"' && field === '' && !afterQuote) quoted = true;
    else { if (afterQuote || c === '"') throw new Error('DB_IMPORT_QUOTING'); field += c; }
  }
  if (quoted) throw new Error('DB_IMPORT_UNCLOSED_QUOTE');
  if (field || row.length || afterQuote) finish();
  const columns = rows.shift() ?? [];
  if (columns.some(c => !c) || new Set(columns).size !== columns.length) throw new Error('DB_IMPORT_HEADERS');
  if (rows.some(r => r.length !== columns.length)) throw new Error('DB_IMPORT_WIDTH');
  return { columns, rows };
}
