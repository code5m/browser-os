/** Pure table editing helpers. Values stay strings/null; never round database integers in JavaScript. */
import { parseDelimited, quoteIdentifier } from './studioSql.mjs';
const size = value => new TextEncoder().encode(value).length;
export function makeTableDocument(name) {
  if (typeof name !== 'string' || !name || size(name) > 255) throw new Error('DB_TABLE_NAME_INVALID');
  return { name, offset: 0, filters: [], order: [], page: null, edits: {}, inserts: [], removed: [], ticket: null, cell: null, cellText: '', cellNull: false, notice: '' };
}
export function tableChanges(document) {
  const page = document.page;
  if (!page) return [];
  const result = [];
  const removed = new Set(document.removed);
  for (const row of page.rows) {
    if (removed.has(row.row_id)) { result.push({ action: 'delete', row_id: row.row_id, values: {} }); continue; }
    const values = {};
    for (const [column, value] of Object.entries(document.edits[row.row_id] || {})) {
      const index = page.table.columns.findIndex(c => c.name === column);
      if (index < 0 || page.table.columns[index].generated || page.table.columns[index].binary) throw new Error('DB_TABLE_COLUMN_NOT_EDITABLE');
      if (value !== null && (typeof value !== 'string' || size(value) > 65536 || value.includes('\0'))) throw new Error('DB_TABLE_VALUE_INVALID');
      if (row.values[index] !== value) values[column] = value;
    }
    if (Object.keys(values).length) result.push({ action: 'update', row_id: row.row_id, values });
  }
  for (const insert of document.inserts) result.push({ action: 'insert', row_id: null, values: { ...insert.values } });
  if (result.length > 100 || size(JSON.stringify(result)) > 4 * 1024 * 1024) throw new Error('DB_TABLE_CHANGE_LIMIT');
  return result;
}
export function dirtyTable(document) {
  return !!document && (document.cell !== null || document.ticket !== null || tableChanges(document).length !== 0);
}
export function importTableRows(text, format, columns) {
  if (size(text) > 5 * 1024 * 1024) throw new Error('DB_IMPORT_LIMIT');
  let input;
  if (format === 'json') {
    input = JSON.parse(text.replace(/^\uFEFF/, ''));
    if (!input || !Array.isArray(input.columns) || !Array.isArray(input.rows)) throw new Error('DB_IMPORT_JSON_SHAPE');
  } else input = parseDelimited(text, format === 'tsv' ? '\t' : ',');
  if (!input.columns.length || input.columns.length > 128 || new Set(input.columns).size !== input.columns.length || input.rows.length > 100) throw new Error('DB_IMPORT_LIMIT');
  for (const name of input.columns) {
    const column = columns.find(c => c.name === name);
    if (!column || column.generated || column.binary) throw new Error('DB_IMPORT_COLUMN_NOT_EDITABLE: ' + name);
  }
  return input.rows.map(row => {
    if (!Array.isArray(row) || row.length !== input.columns.length) throw new Error('DB_IMPORT_WIDTH');
    const values = {};
    row.forEach((value, index) => {
      if (value !== null && !['string', 'boolean', 'number'].includes(typeof value)) throw new Error('DB_IMPORT_SCALAR_REQUIRED');
      if (typeof value === 'number' && (!Number.isFinite(value) || Number.isInteger(value) && !Number.isSafeInteger(value))) throw new Error('DB_IMPORT_USE_STRING_FOR_PRECISE_NUMBER');
      const text = value === null ? null : String(value);
      if (text !== null && (size(text) > 65536 || text.includes('\0'))) throw new Error('DB_IMPORT_CELL_LIMIT');
      values[input.columns[index]] = text;
    });
    return { id: crypto.randomUUID(), values };
  });
}
export function exportTableSql(name, columns, rows, kind) {
  if (!['mysql', 'sqlite', 'postgres'].includes(kind) || rows.length > 1000 || columns.length > 128) throw new Error('DB_EXPORT_LIMIT');
  const table = quoteIdentifier(name, kind);
  const names = columns.map(column => quoteIdentifier(column, kind)).join(', ');
  const literal = value => {
    if (value === null) return 'NULL';
    if (typeof value !== 'string' || size(value) > 65536 || value.includes('\0')) throw new Error('DB_EXPORT_VALUE_INVALID');
    if (kind === 'mysql' && value !== '') {
      const hex = Array.from(new TextEncoder().encode(value), b => b.toString(16).padStart(2, '0')).join('');
      return `CONVERT(0x${hex} USING utf8mb4)`;
    }
    if (kind === 'postgres') return "E'" + value.replaceAll('\\', '\\\\').replaceAll("'", "''") + "'";
    return "'" + value.replaceAll("'", "''") + "'";
  };
  const result = '-- Export of the loaded rows only. Review before executing.\n' + rows.map(row => {
    if (row.length !== columns.length) throw new Error('DB_EXPORT_WIDTH');
    return `INSERT INTO ${table} (${names}) VALUES (${row.map(literal).join(', ')});`;
  }).join('\n');
  if (size(result) > 5 * 1024 * 1024) throw new Error('DB_EXPORT_LIMIT');
  return result;
}
