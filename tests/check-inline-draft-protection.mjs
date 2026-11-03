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
let approved = false;
globalThis.confirm = () => approved;
const {bridge} = await import('../src/bridge.ts');
const {createPinia, setActivePinia} = await import('pinia');
const {useFileStore} = await import('../src/capabilities/workspace/state/useFileStore.ts');
bridge.getStartDirs = async () => [];
bridge.listDir = async () => [];
bridge.writeFile = async () => {};
let deletedPath = null;
bridge.deletePath = async path => {deletedPath = path;};
const pending = new Map();
bridge.readFile = path => new Promise(resolve => pending.set(path, resolve));
setActivePinia(createPinia());
const store = useFileStore();
const entry = name => ({name, path:'/tmp/'+name, is_dir:false, size:0});
const first = store.openFileInline(entry('first.txt'));
pending.get('/tmp/first.txt')('original');
await first;
store.setInlineText('unsaved');
await store.openFileInline(entry('second.txt'));
assert.equal(store.inlineFile, '/tmp/first.txt');
assert.equal(store.inlineText, 'unsaved');
store.closeInline();
assert.equal(store.inlineFile, '/tmp/first.txt');
await store.openDirPreview({name:'dir',path:'/tmp/dir',is_dir:true,size:0});
assert.equal(store.inlineFile, '/tmp/first.txt');
await store.ctxDelete({name:'tmp',path:'/tmp',is_dir:true,size:0});
assert.equal(deletedPath, null, 'declining to discard a nested dirty file must prevent deleting its directory');
assert.equal(store.inlineText, 'unsaved');
approved = true;
const second = store.openFileInline(entry('second.txt'));
store.setInlineText('newer typing');
pending.get('/tmp/second.txt')('second');
await second;
assert.equal(store.inlineFile, '/tmp/first.txt');
assert.equal(store.inlineText, 'newer typing');
const older = store.openFileInline(entry('older.txt'));
const newer = store.openFileInline(entry('newer.txt'));
pending.get('/tmp/newer.txt')('newer');
await newer;
pending.get('/tmp/older.txt')('older');
await older;
assert.equal(store.inlineFile, '/tmp/newer.txt');
assert.equal(store.inlineText, 'newer');
await store.saveInline();
approved = false;
store.closeInline();
assert.equal(store.inlineFile, '');
console.log('INLINE_DRAFT_PROTECTION=PASS');
