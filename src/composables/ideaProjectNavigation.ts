/** User-selected IDEA project navigation, valid for this application session only.
 * Does not read files, passwords or maintain another copy of database settings.
 */
let projectPath:string|null=null;
const subscribers=new Set<(path:string)=>void>();
export function requestIdeaProjectScan(path:string):void{
  projectPath=path;
  for(const subscriber of subscribers)subscriber(path);
}
export function takeIdeaProjectPath():string|null{const next=projectPath;projectPath=null;return next;}
export function subscribeIdeaProjectScan(fn:(path:string)=>void):()=>void{
  subscribers.add(fn);
  return ()=>subscribers.delete(fn);
}
