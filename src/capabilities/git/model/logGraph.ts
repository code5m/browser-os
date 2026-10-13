/** Deterministic read-only lanes for bounded Git history; parents come from libgit2. */
export interface HistoryNode {
  oid: string;
  parents: string[];
}
export interface CommitGraphRow {
  oid: string;
  lane: number;
  through: number[];
  parentLanes: number[];
  hasMerge: boolean;
}
export function layoutCommitGraph<T extends HistoryNode>(
  commits: readonly T[],
  maxLanes = 10,
): CommitGraphRow[] {
  const waiting: string[] = [];
  const rows: CommitGraphRow[] = [];
  for (const commit of commits.slice(0,200)) {
    let lane = waiting.indexOf(commit.oid);
    if (lane < 0) {
      lane = waiting.length;
      waiting.push(commit.oid);
    }
    const through = waiting.map((oid,i)=>oid && i!==lane ? i : -1)
      .filter(i => i >= 0 && i < maxLanes);
    const parents = [...new Set(commit.parents.filter(Boolean))];
    const next = waiting.slice();
    if (parents.length) next[lane] = parents[0];
    else next.splice(lane, 1);
    const parentLanes: number[] = [];
    for (const parent of parents) {
      let target = next.indexOf(parent);
      if (target < 0) {
        target = Math.min(lane + 1, next.length);
        next.splice(target,0,parent);
      }
      parentLanes.push(target);
    }
    rows.push({oid:commit.oid,lane:Math.min(lane,maxLanes-1),
      through,parentLanes:parentLanes.filter(i=>i<maxLanes),hasMerge:parents.length>1});
    waiting.splice(0,waiting.length,...next.slice(0,maxLanes));
  }
  return rows;
}
