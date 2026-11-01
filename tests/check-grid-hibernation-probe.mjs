#!/usr/bin/env node
// Executed in an isolated VM with deterministic browser stubs; the actual
// WebKitGTK eval_result/IPC contract must still be verified in installed GUI.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';

const code = readFileSync(new URL('../src-tauri/injected/grid-hibernation-probe.js', import.meta.url), 'utf8');

function probe({url='about:blank', ready='complete', body=true, children=0,
                text='', interactive=false, beforeUnload=false, optOut=false,
                brokenDom=false}={}) {
  const document = {
    readyState: ready,
    body: body ? {children:{length:children},textContent:text} : null,
    querySelector() {
      if (brokenDom) throw Error('destroyed document');
      return interactive ? {} : null;
    },
  };
  return JSON.parse(runInNewContext(code, {
    document,
    location:{href:url},
    window:{
      onbeforeunload:beforeUnload ? (()=>true) : null,
      __browserosPreventHibernate:optOut,
    },
    JSON,
  }, {timeout:1000}));
}

assert.deepEqual(probe(),{safe:true,url:'about:blank'});
for (const scenario of [
  {url:'https://chat.deepseek.com'},
  {url:'about:blank#draft'},
  {ready:'loading'},
  {body:false},
  {children:1},
  {text:'Unsent message'},
  {interactive:true},
  {beforeUnload:true},
  {optOut:true},
]) {
  assert.equal(probe(scenario).safe,false,JSON.stringify(scenario));
}
assert.deepEqual(probe({brokenDom:true}),{safe:false,url:''});
console.log('GRID_HIBERNATION_PROBE_SELF_TEST=PASS (VM fixtures only; GUI_PENDING)');
