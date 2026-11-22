import assert from 'node:assert/strict';
import {persistDraftAfterSave} from '../src/capabilities/workspace/model/draftRecovery.mjs';

const calls = [];
const save = async (...args) => { calls.push(['save', ...args]); };
const clear = async (...args) => { calls.push(['clear', ...args]); };

await persistDraftAfterSave({
  path:'/workspace/a.ts', currentSessionId:'current',
  savedText:'saved', currentText:'newer-unsaved', save, clear,
});
assert.deepEqual(calls, [
  ['save','/workspace/a.ts','current','saved','newer-unsaved'],
], 'persist current edits without deleting the recovered source owned by another session');

calls.length = 0;
await persistDraftAfterSave({
  path:'/workspace/a.ts', currentSessionId:'current',
  savedText:'saved', currentText:'saved', save, clear,
});
assert.deepEqual(calls, [
  ['clear','/workspace/a.ts','current'],
], 'a clean saved document clears only the current surface journal');

calls.length = 0;
await persistDraftAfterSave({
  path:'/workspace/a.ts', currentSessionId:'same',
  savedText:'saved', currentText:'saved', save, clear,
});
assert.deepEqual(calls, [
  ['clear','/workspace/a.ts','same'],
], 'a successful save retires its own journal once');

calls.length = 0;
await assert.rejects(() => persistDraftAfterSave({
  path:'/workspace/a.ts', currentSessionId:'current',
  savedText:'saved', currentText:'newer-unsaved',
  save:async()=>{throw new Error('disk full');}, clear,
}), /disk full/);
assert.deepEqual(calls, [], 'a failed current-session journal write must not clear any recovery source');

console.log('Workspace draft recovery: 4 scenarios passed');
