import assert from 'node:assert/strict';
import { quickCreateAction } from '../src/composables/quickCreate.mjs';

const event = overrides => ({key:'n',ctrlKey:true,shiftKey:false,altKey:false,metaKey:false,isComposing:false,repeat:false,defaultPrevented:false,...overrides});
assert.equal(quickCreateAction(event(), 'files'), 'file');
assert.equal(quickCreateAction(event({shiftKey:true}), 'files'), 'directory');
assert.equal(quickCreateAction(event(), 'home'), 'tab');
for (const change of [{shiftKey:true},{altKey:true},{metaKey:true},{ctrlKey:false},{repeat:true},{isComposing:true},{defaultPrevented:true},{key:'x'}]) {
  assert.equal(quickCreateAction(event(change), 'home'), null);
}
console.log('QUICK_CREATE_SHORTCUTS=PASS');
