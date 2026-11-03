import assert from 'node:assert/strict';
import { registerHooks } from 'node:module';
registerHooks({resolve(specifier, context, next) {
  try { return next(specifier, context); } catch (error) {
    for (const ext of ['.ts', '.mjs', '.js']) {
      try { return next(specifier + ext, context); } catch {}
    }
    throw error;
  }
}});
globalThis.window = {setTimeout};
let approveDiscard = false;
globalThis.confirm = () => approveDiscard;
const {bridge} = await import('../src/bridge.ts');
const {createPinia, setActivePinia} = await import('pinia');
const {useFileStore} = await import('../src/capabilities/workspace/state/useFileStore.ts');
bridge.getStartDirs = async () => [];
bridge.editorDraftList = async () => [];
bridge.editorDraftSave = async () => {};
bridge.editorDraftClear = async () => {};
bridge.listDir = async () => [];
const disk = new Map([['/tmp/a.txt','before'],['/tmp/b.txt','other']]);
let writes = 0;
bridge.readFile = async path => disk.get(path);
bridge.writeFileCas = async (path,expected,text) => { if(disk.get(path)!==expected) throw new Error('FILE_CONFLICT'); writes++;disk.set(path,text); };
setActivePinia(createPinia());
const store = useFileStore();
const a={name:'a.txt',path:'/tmp/a.txt',is_dir:false,size:6};
const b={name:'b.txt',path:'/tmp/b.txt',is_dir:false,size:5};
let deleted=null, renamed=null, moved=null;
bridge.deletePath = async path => { deleted=path; };
bridge.renamePath = async (path,name) => { renamed=[path,name]; };
bridge.movePath = async (path,to) => { moved=[path,to]; };

await store.openFile(a);
assert.equal(store.editorFilePath,a.path);
assert.equal(store.filePath,'','directory state must not be replaced by the editor document');
store.setFileContent('unsaved');
assert.equal(store.editorDirty,true);
await store.ctxDelete({name:'tmp',path:'/tmp',is_dir:true,size:0});
assert.equal(deleted,null,'dirty overlay descendant must prevent directory deletion');
await store.ctxRename({name:'tmp',path:'/tmp',is_dir:true,size:0});
assert.equal(renamed,null,'dirty overlay descendant must prevent directory rename');
store.requestMove('/tmp', '/elsewhere');
await store.confirmMove();
assert.equal(moved,null,'dirty overlay descendant must prevent move');
await store.openFile(b);
assert.equal(store.editorFilePath,a.path,'declined discard must retain current document');
await store.enterDir('/tmp/next');
assert.equal(store.filePath,'','declined navigation must retain directory');
disk.set(a.path,'external update');
await store.saveFile();
assert.equal(writes,0,'external modification must block advisory save');
assert.equal(store.editorDirty,true);
disk.set(a.path,'before');
await store.saveFile();
assert.equal(writes,1);
assert.equal(store.editorDirty,false);
approveDiscard=true;
await store.openFile(b);
assert.equal(store.editorFilePath,b.path);
const {useLayoutStore} = await import('../src/stores/useLayoutStore.ts');
const layout = useLayoutStore();
store.setFileContent('dirty again');
approveDiscard=false;
layout.setView('editor');
assert.equal(layout.setView('browser'),false,'module navigation must block dirty overlay');
assert.equal(layout.mainView,'editor');
approveDiscard=true;
assert.equal(layout.setView('browser'),true);
assert.equal(layout.mainView,'browser');
console.log('MAIN_EDITOR_DRAFT_PROTECTION=PASS (native lock/rename tested separately)');
