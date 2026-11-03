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
bridge.listDir = async () => [];
const disk = new Map([['/tmp/a.txt','before'],['/tmp/b.txt','other']]);
let writes = 0;
bridge.readFile = async path => disk.get(path);
bridge.writeFile = async (path,text) => {writes++;disk.set(path,text);};
setActivePinia(createPinia());
const store = useFileStore();
const a={name:'a.txt',path:'/tmp/a.txt',is_dir:false,size:6};
const b={name:'b.txt',path:'/tmp/b.txt',is_dir:false,size:5};
await store.openFile(a);
assert.equal(store.editorFilePath,a.path);
assert.equal(store.filePath,'','directory state must not be replaced by the editor document');
store.setFileContent('unsaved');
assert.equal(store.editorDirty,true);
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
console.log('MAIN_EDITOR_DRAFT_PROTECTION=PASS (advisory conflict check only; NOT native CAS)');
