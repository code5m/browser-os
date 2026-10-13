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
