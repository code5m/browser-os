import assert from 'node:assert/strict';
import { registerHooks } from 'node:module';
registerHooks({ resolve(specifier, context, next) {
  try { return next(specifier, context); } catch (error) {
    for (const extension of ['.ts', '.mjs', '.js']) {
      try { return next(specifier + extension, context); } catch {}
    }
    throw error;
  }
} });
const timers = new Map();
let timerId = 0;
globalThis.setTimeout = callback => { timers.set(++timerId, callback); return timerId; };
globalThis.clearTimeout = id => timers.delete(id);
globalThis.window = { setTimeout: () => 0 };
globalThis.confirm = () => true;
const flushDraftTimers = () => {
  const callbacks = [...timers.values()]; timers.clear();
  for (const callback of callbacks) callback();
};
const deferred = () => {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};
const { bridge } = await import('../src/bridge.ts');
const { createPinia, setActivePinia } = await import('pinia');
const { documentOwnership } = await import('../src/application/documentOwnership.mjs');
const { useFileStore } = await import('../src/capabilities/workspace/state/useFileStore.ts');
const { useLayoutStore } = await import('../src/stores/useLayoutStore.ts');
// Load Vue in its real Node environment before supplying the narrow tree lookup
// used by these owner tests. This is not a renderer or an incomplete DOM.
globalThis.document = { querySelectorAll: () => [] };
bridge.editorDraftList = async () => [];
bridge.getStartDirs = async () => [];
bridge.listDir = async () => [];
bridge.editorDraftClear = async () => {};
bridge.readFile = async path => path.endsWith('b.txt') ? 'document B' : 'document A';
setActivePinia(createPinia());
const files = useFileStore(), layout = useLayoutStore();
await files.openFile({ path: '/work/a.txt', name: 'a.txt', is_dir: false, size: 10 });
assert.equal(layout.fileEditorOpen, true, 'successful editor publication opens the Layout-owned surface');
const oldJournal = deferred(), cas = deferred();
const journals = [];
bridge.editorDraftSave = async (path, session, baseline, content) => {
  journals.push({ path, content });
  if (journals.length === 1) await oldJournal.promise;
};
files.setFileContent('saved A');
flushDraftTimers();
bridge.writeFileCas = () => cas.promise;
const saving = files.saveFile();
files.setFileContent('newer A');
cas.resolve();
await Promise.resolve(); await Promise.resolve();
await files.openFile({ path: '/work/b.txt', name: 'b.txt', is_dir: false, size: 10 });
oldJournal.resolve();
await saving;
assert(journals.some(item => item.path === '/work/a.txt' && item.content === 'newer A'));
assert(!journals.some(item => item.path === '/work/a.txt' && item.content === 'document B'),
  'a queued save must use its own document snapshot');

const late = deferred();
bridge.readFile = () => late.promise;
const opening = files.openFile({ path: '/work/late.txt', name: 'late.txt', is_dir: false, size: 4 });
layout.activateHome();
late.resolve('late');
await opening;
assert.equal(layout.mainView, 'home');
assert.equal(layout.fileEditorOpen, false, 'a stale editor read cannot reopen the surface after Home wins');
assert.equal(files.editorFilePath, '/work/b.txt', 'late editor reads do not replace the newer navigation');

const inline = deferred();
bridge.readFile = () => inline.promise;
const openingInline = files.openFileInline({ path: '/old/a.txt', name: 'a.txt', is_dir: false, size: 4 });
await files.enterDir('/new');
inline.resolve('old inline');
await openingInline;
assert.equal(files.filePath, '/new');
assert.equal(files.treeRoots[0].path, '/new');
assert.equal(files.inlineFile, '');

const tabCount = layout.modTabs.length;
const denied = layout.prepareDirTab('/unreadable');
bridge.listDir = async () => { throw Error('UNREADABLE'); };
assert.equal(await files.enterDir('/unreadable', denied), false);
assert.equal(layout.modTabs.length, tabCount, 'failed directory reads publish no tab');

let removed = 0, nativeWrites = 0;
bridge.editorDraftCatalog = async () => ({
  items: [{ recovery_id: 'source', path: '/missing/orphan.txt', session_id: 'old', updated_at_ms: 1, revision: 'catalog-r1' }],
  next_cursor: null,
});
bridge.editorDraftRead = async (id, fork = false) => ({
  recovery_id: fork ? 'fork' : id, path: '/missing/orphan.txt', session_id: 'recovery',
  baseline: 'base', content: 'orphan contents', revision: 'r1', updated_at_ms: 1,
});
const updates = [];
bridge.editorDraftUpdateRecovered = async (id, revision, content) => {
  updates.push({ id, revision, content }); return { revision: 'r' + (updates.length + 1) };
};
bridge.editorDraftRemove = async () => { removed++; };
bridge.writeFileCas = async () => { nativeWrites++; };
await files.loadRecoverableDrafts();
assert.equal(files.recoverableDrafts.length, 1);
await files.openRecoveredDraft('source');
assert.equal(files.editorRecoveryId, 'fork', 'recovery edits use an isolated native fork');
assert.equal(files.fileContent, 'orphan contents');
assert.equal(layout.fileEditorOpen, true, 'a recovered document uses the same Layout publication');
globalThis.confirm = () => false;
assert.equal(layout.activateHome(), false, 'a dirty recovery can refuse navigation');
assert.equal(layout.mainView, 'editor');
assert.equal(layout.fileEditorOpen, true, 'a refused navigation preserves the editor surface');
globalThis.confirm = () => true;
await files.saveFile();
assert.equal(nativeWrites, 0, 'ordinary save cannot recreate an orphan source');
files.setFileContent('edited recovery');
flushDraftTimers();
await Promise.resolve(); await Promise.resolve();
assert.equal(updates[0].id, 'fork');
bridge.editorDraftSaveAs = async () => { throw Error('FILE_CONFLICT'); };
await files.saveRecoveredDraftAs('/existing.txt');
assert.equal(files.editorRecoveryId, 'fork');
assert.equal(files.fileContent, 'edited recovery');
bridge.editorDraftSaveAs = async (_id, path, content) => ({ path, content });
await files.saveRecoveredDraftAs('/restored.txt');
assert.equal(files.editorFilePath, '/restored.txt');
assert.equal(files.editorRecoveryId, '');
assert.equal(removed, 0, 'saving a recovery copy preserves the encrypted source');

documentOwnership.replace('coding-workbench', [{ path: '/project/old/new.rs', dirty: false }]);
documentOwnership.invalidatePath('/project/old', true);
assert.throws(() => documentOwnership.beginSave('coding-workbench', '/project/old/new.rs'), /DOCUMENT_PATH_CHANGED/,
  'an unopened proposal registered with its owner cannot recreate a deleted directory');
documentOwnership.replace('coding-workbench', []);
documentOwnership.replace('workspace-overlay', []);
documentOwnership.replace('workspace-inline', []);
setActivePinia(createPinia());
timers.clear();
const reopenedFiles = useFileStore();
bridge.readFile = async () => 'disk baseline';
let clearedNewInstance = false;
bridge.editorDraftClear = async () => { clearedNewInstance = true; };
await reopenedFiles.openFile({ path: '/work/reopen.txt', name: 'reopen.txt', is_dir: false, size: 4 });
reopenedFiles.setFileContent('first instance save');
const firstInstanceCas = deferred();
bridge.writeFileCas = () => firstInstanceCas.promise;
const firstInstanceSave = reopenedFiles.saveFile();
await reopenedFiles.openFile({ path: '/work/reopen.txt', name: 'reopen.txt', is_dir: false, size: 4 });
reopenedFiles.setFileContent('new instance draft');
firstInstanceCas.resolve();
await firstInstanceSave;
assert.equal(reopenedFiles.fileContent, 'new instance draft');
assert.equal(clearedNewInstance, false, 'an old save cannot retire a reopened document instance');
documentOwnership.replace('workspace-overlay', []);
documentOwnership.replace('workspace-inline', []);

setActivePinia(createPinia());
const startupFiles = useFileStore(), startupLayout = useLayoutStore();
const startupRead = deferred();
bridge.listDir = () => startupRead.promise;
const warming = startupFiles.initializeDirectory('/startup');
startupLayout.activateHome();
startupRead.resolve([]);
assert.equal(await warming, false);
assert.equal(startupLayout.mainView, 'home');
assert.equal(startupFiles.filePath, '', 'a late startup read cannot populate a superseded directory intent');
bridge.listDir = async () => [];
assert.equal(await startupFiles.initializeDirectory('/startup'), true);
assert.equal(startupLayout.mainView, 'home', 'background initialization never opens a Files tab');
assert.equal(startupLayout.modTabs.length, 0);
await startupFiles.ctxOpenInNewTab({ path: '/first', name: 'first', is_dir: true, size: 0 });
const firstDirectoryTab = startupLayout.activeModTab;
await startupFiles.ctxOpenInNewTab({ path: '/second', name: 'second', is_dir: true, size: 0 });
assert.equal(startupLayout.modTabs.length, 2, 'explicit directory new-tab does not reuse the active directory');
assert.equal(startupLayout.modTabs.find(tab => tab.id === firstDirectoryTab).path, '/first');
assert.equal(startupFiles.filePath, '/second');
bridge.listDir = async () => { throw Error('UNREADABLE_CONTEXT_DIRECTORY'); };
await startupFiles.ctxOpenInNewTab({ path: '/denied', name: 'denied', is_dir: true, size: 0 });
assert.equal(startupLayout.modTabs.length, 2, 'an unreadable directory creates no context-menu tab');
assert.equal(startupFiles.filePath, '/second');
console.log('WORKSPACE_RECOVERY_NAVIGATION=PASS');
