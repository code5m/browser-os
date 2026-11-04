const formatBytes = value => {
  if (!Number.isFinite(value) || value < 0) return '-';
  if (value < 1024) return value + ' B';
  if (value < 1024 * 1024) return (value / 1024).toFixed(1) + ' KiB';
  return (value / (1024 * 1024)).toFixed(2) + ' MiB';
};

/** Decode the native DbValue ABI without applying presentation truncation. */
export function decodeDbValue(value) {
  if (value === null || value === undefined || value === 'null') return null;
  if (typeof value === 'boolean' || typeof value === 'number' || typeof value === 'string') return value;
  if (typeof value === 'object') {
    const rec=value;
    if (typeof rec.text === 'string') return rec.text;
    if (typeof rec.i64 === 'string' && /^-?(?:0|[1-9]\d*)$/.test(rec.i64)) {
      const exact=Number(rec.i64);
      return Number.isSafeInteger(exact)?exact:rec.i64;
    }
    if (typeof rec.i64 === 'number') return Number.isSafeInteger(rec.i64)?rec.i64:String(rec.i64);
    if (typeof rec.f64 === 'number') return rec.f64;
    if (rec.binary && typeof rec.binary === 'object' && typeof rec.binary.bytes === 'number') return '<binary ' + formatBytes(rec.binary.bytes) + '>';
    if (typeof rec.int === 'number') return rec.int;
    if (typeof rec.float === 'number') return rec.float;
    if (typeof rec.bool === 'boolean') return rec.bool;
    if (typeof rec.blob_len === 'number') return '<binary ' + formatBytes(rec.blob_len) + '>';
  }
  return JSON.stringify(value);
}

/** Decode raw IPC rows without display truncation; exporters use this path. */
export function decodeDbRows(rows) {
  if (!Array.isArray(rows)) return [];
  return rows.map(row => Array.isArray(row) ? row.map(decodeDbValue) : []);
}

/** One non-mutating presentation projection for all table views. */
export function projectTableRows(columns, rows, edits = {}, inserts = [], removed = []) {
  if (!Array.isArray(columns) || !Array.isArray(rows) || !Array.isArray(inserts) || !Array.isArray(removed) || !edits || typeof edits !== 'object') throw new Error('DB_TABLE_PROJECTION_INVALID');
  const removedIds=new Set(removed);
  const projected=rows.map(row=>{
    if(!row||!Array.isArray(row.values)||row.values.length!==columns.length||row.row_id===null||row.row_id===undefined) throw new Error('DB_TABLE_PROJECTION_ROW_INVALID');
    const draft=Object.prototype.hasOwnProperty.call(edits,row.row_id)&&edits[row.row_id]&&typeof edits[row.row_id]==='object'?edits[row.row_id]:{};
    const values=columns.map((column,index)=>Object.prototype.hasOwnProperty.call(draft,column.name)?draft[column.name]:row.values[index]);
    return {id:row.row_id,rowId:row.row_id,inserted:false,removed:removedIds.has(row.row_id),values,defaults:columns.map(()=>false)};
  });
  for(const row of inserts){
    if(!row||typeof row.id!=='string'||!row.values||typeof row.values!=='object') throw new Error('DB_TABLE_PROJECTION_INSERT_INVALID');
    const defaults=columns.map(column=>!Object.prototype.hasOwnProperty.call(row.values,column.name));
    const values=columns.map(column=>Object.prototype.hasOwnProperty.call(row.values,column.name)?row.values[column.name]:undefined);
    projected.push({id:row.id,rowId:null,inserted:true,removed:false,values,defaults});
  }
  return projected;
}
