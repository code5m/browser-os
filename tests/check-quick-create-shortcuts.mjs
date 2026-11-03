import assert from 'node:assert/strict';
import { quickCreateAction } from '../src/composables/quickCreate.mjs';

const event = overrides => ({key:'n',ctrlKey:true,shiftKey:false,altKey:false,metaKey:false,isComposing:false,repeat:false,defaultPrevented:false,...overrides});
assert.equal(quickCreateAction(event(), 'files'), 'file');
assert.equal(quickCreateAction(event({shiftKey:true}), 'files'), 'directory');
assert.equal(quickCreateAction(event(), 'home'), 'tab');
for (const change of [{shiftKey:true},{altKey:true},{metaKey:true},{ctrlKey:false},{repeat:true},{isComposing:true},{defaultPrevented:true},{key:'x'}]) {
  assert.equal(quickCreateAction(event(change), 'home'), null);
}
for (const target of [
  {tagName:'INPUT'}, {tagName:'TEXTAREA'}, {tagName:'SELECT'},
  {tagName:'DIV',isContentEditable:true},
  {tagName:'DIV',closest:selector => selector.includes('.monaco-editor') ? {} : null},
  {tagName:'DIV',closest:selector => selector.includes('.xterm') ? {} : null}
]) {
  assert.equal(quickCreateAction(event({target}), 'files'), null, 'text/terminal input must keep native shortcut ownership');
  assert.equal(quickCreateAction(event({target}), 'home'), null);
}
assert.equal(quickCreateAction(event({target:{tagName:'BUTTON'}}),'files'),'file','accessible toolbar button focus allows global create');
console.log('QUICK_CREATE_SHORTCUTS=PASS');
