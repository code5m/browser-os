import assert from 'node:assert/strict';
import { registerHooks } from 'node:module';

registerHooks({ resolve(specifier, context, next) {
  try { return next(specifier, context); } catch (error) {
    for (const ext of ['.ts', '.mjs', '.js']) {
      try { return next(specifier + ext, context); } catch {}
    }
    throw error;
  }
} });
globalThis.window = { setTimeout };
let requestedName = 'created.txt';
globalThis.prompt = () => requestedName;
const { bridge } = await import('../src/bridge.ts');
const { createPinia, setActivePinia } = await import('pinia');
const { useFileStore } = await import('../src/capabilities/workspace/state/useFileStore.ts');
const { useLayoutStore } = await import('../src/stores/useLayoutStore.ts');
let created;
bridge.createFile = async path => { created = path; };
bridge.listDir = async () => [];
bridge.getStartDirs = async () => [];
setActivePinia(createPinia());
const files = useFileStore();
const layout = useLayoutStore();
await files.enterDir('/tmp/project');
await files.quickNew('file');
assert.equal(created, '/tmp/project/created.txt', 'a directory is itself the creation target');
files.previewDir = '/tmp/project/nested';
await files.quickNew('file');
assert.equal(created, '/tmp/project/nested/created.txt', 'a selected directory preview is the creation target');
files.inlineFile = '/tmp/project/src/existing.ts';
await files.quickNew('file');
assert.equal(created, '/tmp/project/src/created.txt', 'an open file targets its parent directory');
files.inlineFile = '';
files.previewDir = '';
files.editorFilePath = '/tmp/project/legacy.txt';
layout.fileEditorOpen = true;
await files.quickNew('file');
assert.equal(created, '/tmp/project/created.txt', 'editor document targets the file parent');
files.inlineFile = '/tmp/old/inline.ts';
files.previewDir = '/tmp/old/preview';
files.editorFilePath = '/tmp/new/legacy.txt';
await files.quickNew('file');
assert.equal(created, '/tmp/new/created.txt', 'active document must win over stale inline and preview paths');
for (const name of ['..', '../escaped', 'nested/file', 'nested\\file']) {
  requestedName = name;
  created = undefined;
  await files.quickNew('file');
  assert.equal(created, undefined, 'quick creation rejects path components');
}
console.log('FILE_QUICK_CREATE=PASS');
