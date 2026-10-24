/** Pure, bounded renderer for libgit2's unified patch: changed fragments, not full files. */
export interface DiffRow {
  kind: "header" | "meta" | "context" | "change";
  beforeLine: number | null;
  afterLine: number | null;
  before: string;
  after: string;
  removed: boolean;
  added: boolean;
}
export interface ParsedPatch { rows: DiffRow[]; additions: number; deletions: number; truncated: boolean; }
/** Restore unchanged gaps from the actual after-revision blob, not invented lines. */
export function historyContextRows(patch:string, afterContent:string): DiffRow[] {
  const parsed = parseUnifiedPatch(patch);
  const after = afterContent.split('\n');
  if (after[after.length-1] === '') after.pop();
  const rows:DiffRow[] = [];
  let oldLine=1, newLine=1;
  function contextUntil(end:number) {
    while (newLine < end && newLine <= after.length && rows.length < 12000) {
      const text=after[newLine-1];
      rows.push({kind:'context',beforeLine:oldLine++,afterLine:newLine++,before:text,after:text,removed:false,added:false});
    }
  }
  for (const row of parsed.rows) {
    if (row.kind==='meta') continue;
    if (row.kind==='header') {
      const start=/^@@ -(\d+)(?:,\d+)? \+(\d+)/.exec(row.before);
      if (start) { contextUntil(Number(start[2])); oldLine=Math.max(1,Number(start[1])); newLine=Math.max(1,Number(start[2])); }
      continue;
    }
    rows.push(row);
    if (row.beforeLine!==null) oldLine=row.beforeLine+1;
    if (row.afterLine!==null) newLine=row.afterLine+1;
  }
  if (!parsed.truncated) contextUntil(after.length+1);
  return rows;
}
export function parseUnifiedPatch(patch: string, maxRows = 4000): ParsedPatch {
  const lines = patch.split(/\r?\n/);
  const rows: DiffRow[] = [];
  const minus: {line: number; text: string}[] = [];
  const plus: {line: number; text: string}[] = [];
  let beforeLine = 0, afterLine = 0, additions = 0, deletions = 0, truncated = false;
  function push(row: DiffRow) {
    if (rows.length >= maxRows) { truncated = true; return; }
    rows.push(row);
  }
  function flush() {
    for (let i = 0; i < Math.max(minus.length, plus.length); i++) {
      push({kind:"change",beforeLine:minus[i]?.line??null,afterLine:plus[i]?.line??null,
        before:minus[i]?.text??"",after:plus[i]?.text??"",removed:!!minus[i],added:!!plus[i]});
    }
    minus.length = 0; plus.length = 0;
  }
  for (const line of lines) {
    if (rows.length >= maxRows) { truncated = true; break; }
    const match = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(line);
    if (match) {
      flush(); beforeLine = Number(match[1]); afterLine = Number(match[2]);
      push({kind:"header",beforeLine:null,afterLine:null,before:line,after:line,removed:false,added:false});
    } else if (/^(--- |\+\+\+ |diff --git |index |new file |deleted file |rename |similarity )/.test(line)) {
      flush();
      push({kind:"meta",beforeLine:null,afterLine:null,before:line,after:"",removed:false,added:false});
    } else if (line.startsWith("-")) {
      minus.push({line:beforeLine++,text:line.slice(1)}); deletions++;
    } else if (line.startsWith("+")) {
      plus.push({line:afterLine++,text:line.slice(1)}); additions++;
    } else if (line.startsWith(" ")) {
      flush();
      push({kind:"context",beforeLine:beforeLine++,afterLine:afterLine++,before:line.slice(1),after:line.slice(1),removed:false,added:false});
    } else if (line.startsWith("\\ No newline") || line.trim()) {
      flush();
      push({kind:"meta",beforeLine:null,afterLine:null,before:line,after:"",removed:false,added:false});
    }
  }
  flush();
  return {rows,additions,deletions,truncated};
}
