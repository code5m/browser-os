import assert from 'node:assert/strict';
import {registerHooks} from 'node:module';
registerHooks({resolve(specifier,context,next){
  try{return next(specifier,context)}catch(error){
    for(const ext of ['.ts','.mjs','.js'])try{return next(specifier+ext,context)}catch{}
    throw error;
  }
}});
globalThis.window={setTimeout};
globalThis.confirm=()=>true;
const {bridge}=await import('../src/bridge.ts');
const {createPinia,setActivePinia}=await import('pinia');
const {documentOwnership}=await import('../src/application/documentOwnership.mjs');
const {useFileStore}=await import('../src/capabilities/workspace/state/useFileStore.ts');

bridge.getStartDirs=async()=>[];
bridge.editorDraftList=async()=>[];
bridge.editorDraftSave=async()=>{};
bridge.editorDraftClear=async()=>{};
bridge.listDir=async()=>[];
const disk=new Map([['/tmp/cross.txt','base']]);
let saves=0,deleted=0;
bridge.readFile=async path=>disk.get(path);
bridge.writeFileCas=async(path,baseline,content)=>{
  assert.equal(disk.get(path),baseline);
  saves++;disk.set(path,content);
};
bridge.deletePath=async()=>{deleted++};
bridge.movePath=async()=>{throw Error('NOT_EXPECTED')};
setActivePinia(createPinia());
const store=useFileStore();
await store.openFile({name:'cross.txt',path:'/tmp/cross.txt',is_dir:false,size:4});
store.setFileContent('workspace draft');
documentOwnership.replace('coding-workbench',[{path:'/tmp/cross.txt',dirty:true}]);
assert.deepEqual(documentOwnership.conflicts('workspace-overlay','/tmp/cross.txt'),['coding-workbench']);
await store.saveFile();
assert.equal(saves,0,'Coding dirty buffer blocks Workspace save');
await store.ctxDelete({name:'tmp',path:'/tmp',is_dir:true,size:0});
assert.equal(deleted,0,'dirty document in Coding blocks containing-directory delete');
documentOwnership.replace('coding-workbench',[{path:'/tmp/cross.txt',dirty:false}]);
await store.saveFile();
assert.equal(saves,1,'clean Coding buffer does not block verified native save');
const release=documentOwnership.beginSave('coding-workbench','/tmp/cross.txt');
store.setFileContent('new edits');
await store.saveFile();
assert.equal(saves,1,'in-flight Coding write blocks Workspace save');
release();
await store.saveFile();
assert.equal(saves,2);
documentOwnership.replace('coding-workbench',[]);
documentOwnership.replace('workspace-overlay',[]);
console.log('CROSS_WORKBENCH_FILE_STORE=PASS');
