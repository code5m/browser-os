// Canonical in-memory text/document state shared across Workspace and Coding UI.
// Native filesystem access, permission checks and save approval remain with each
// capability's existing Public API. No hidden disk reads, writes or model calls.
export function normalizeDocumentPath(root,path){
 if(typeof path!=='string'||!path||path.includes('\0'))throw Error('DOCUMENT_PATH_REQUIRED');
 if(path.startsWith('/'))return path;
 if(typeof root!=='string'||!root.startsWith('/'))throw Error('DOCUMENT_ROOT_REQUIRED');
 if(path.split('/').some(x=>x==='..'||x==='.'||x===''))throw Error('DOCUMENT_PATH_INVALID');
 return root.replace(/\/+$/,'')+'/'+path;
}
export function createDocumentManager({limit=128}={}){
 const docs=new Map(),watchers=new Set();
 let clock=0;
 const snapshot=d=>Object.freeze({path:d.path,content:d.content,saved:d.saved,hash:d.hash,version:d.version,dirty:d.content!==d.saved,owners:[...d.owners]});
 function notify(d){const item=snapshot(d);for(const fn of watchers){try{fn(item);}catch{/* observers cannot break edits */}}}
 function get(path){return docs.get(path);}
 function open({path,content,hash=null,owner}){
  if(typeof content!=='string'||typeof owner!=='string'||!owner)throw Error('DOCUMENT_ARGUMENT_INVALID');
  let d=get(path);
  if(!d){
   if(docs.size>=limit){
    for(const [k,v] of docs){if(!v.owners.size&&v.content===v.saved){docs.delete(k);break;}}
    if(docs.size>=limit)throw Error('DOCUMENT_CAPACITY_REACHED');
   }
   d={path,content,saved:content,hash,version:1,owners:new Set(),access:++clock};
   docs.set(path,d);
  }else if(d.content===d.saved && d.saved!==content){
   // Refresh only clean documents, never overwrite another editor's draft.
   d.content=content;d.saved=content;d.hash=hash;d.version++;
  }
  d.owners.add(owner);d.access=++clock;notify(d);return snapshot(d);
 }
 function edit(path,content,expectedVersion){
  const d=get(path);if(!d)throw Error('DOCUMENT_NOT_OPEN');
  if(typeof content!=='string')throw Error('DOCUMENT_TEXT_INVALID');
  if(expectedVersion!==undefined&&expectedVersion!==d.version)throw Error('DOCUMENT_VERSION_CONFLICT');
  if(d.content!==content){d.content=content;d.version++;d.access=++clock;notify(d);}
  return snapshot(d);
 }
 function markSaved(path,attemptedContent,hash=null){
  const d=get(path);if(!d)throw Error('DOCUMENT_NOT_OPEN');
  if(typeof attemptedContent!=='string')throw Error('DOCUMENT_TEXT_INVALID');
  // The disk may have accepted an earlier version while the user kept editing.
  d.saved=attemptedContent;d.hash=hash;d.version++;notify(d);
  return snapshot(d);
 }
 function release(path,owner){
  const d=get(path);if(!d)return;
  d.owners.delete(owner);
  // Dirty drafts remain available for later reconciliation; do not silently
  // discard them when an editor view is unmounted.
  if(!d.owners.size&&d.content===d.saved)docs.delete(path);
 }
 return Object.freeze({open,edit,markSaved,release,peek:path=>{const d=get(path);return d?snapshot(d):null;},
   subscribe:fn=>{if(typeof fn!=='function')throw Error('DOCUMENT_OBSERVER_INVALID');watchers.add(fn);return()=>watchers.delete(fn);},
   inspect:()=>[...docs.values()].map(snapshot)});
}
export const sharedDocuments=createDocumentManager();
