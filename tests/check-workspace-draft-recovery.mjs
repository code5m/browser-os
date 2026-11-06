import assert from 'node:assert/strict';
import {persistDraftAfterSave} from '../src/capabilities/workspace/model/draftRecovery.mjs';

const calls = [];
const save = async (...args) => { calls.push(['save', ...args]); };
const clear = async (...args) => { calls.push(['clear', ...args]); };

await persistDraftAfterSave({
  path:'/workspace/a.ts', currentSessionId:'current', recoveredSessionId:'recovered',
  savedText:'saved', currentText:'newer-unsaved', save, clear,
});
assert.deepEqual(calls, [
  ['save','/workspace/a.ts','current','saved','newer-unsaved'],
  ['clear','/workspace/a.ts','recovered'],
], 'persist current edits before retiring only the recovered source journal');

calls.length = 0;
await persistDraftAfterSave({
  path:'/workspace/a.ts', currentSessionId:'current', recoveredSessionId:'recovered',
  savedText:'saved', currentText:'saved', save, clear,
});
assert.deepEqual(calls, [
  ['clear','/workspace/a.ts','current'],
  ['clear','/workspace/a.ts','recovered'],
], 'a clean saved document clears only current and selected recovered sessions');

calls.length = 0;
await persistDraftAfterSave({
  path:'/workspace/a.ts', currentSessionId:'same', recoveredSessionId:'same',
  savedText:'saved', currentText:'saved', save, clear,
});
assert.deepEqual(calls, [
  ['clear','/workspace/a.ts','same'],
], 'identical current and recovered session IDs are cleared once');

calls.length = 0;
await assert.rejects(() => persistDraftAfterSave({
  path:'/workspace/a.ts', currentSessionId:'current', recoveredSessionId:'recovered',
  savedText:'saved', currentText:'newer-unsaved',
  save:async()=>{throw new Error('disk full');}, clear,
}), /disk full/);
assert.deepEqual(calls, [], 'a failed current-session journal write must not retire the recovered source');

console.log('Workspace draft recovery: 4 scenarios passed');
