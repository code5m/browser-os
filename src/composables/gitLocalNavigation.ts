/** Single ephemeral navigation intent. No file paths are persisted to disk.
 * Workspace publishes a generic intent; the Git capability owns the reader UI.
 */
export type LocalGitMode = "history" | "diff";
export interface LocalGitIntent { path:string; mode:LocalGitMode; serial:number; }
let current:LocalGitIntent|null=null;
let serial=0;
const listeners=new Set<(intent:LocalGitIntent|null)=>void>();
export function currentLocalGitIntent():LocalGitIntent|null { return current; }
export function requestLocalGitIntent(path:string,mode:LocalGitMode):void {
  current={path,mode,serial:++serial};
  for(const fn of listeners)fn(current);
}
export function clearLocalGitIntent():void {
  current=null;
  for(const fn of listeners)fn(null);
}
export function subscribeLocalGitIntent(fn:(intent:LocalGitIntent|null)=>void):()=>void {
  listeners.add(fn);
  return ()=>listeners.delete(fn);
}
