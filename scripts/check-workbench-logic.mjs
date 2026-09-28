#!/usr/bin/env node
import assert from 'node:assert/strict';
import { registerHooks } from 'node:module';
import { resolveNote, noteLinks, searchNotes } from '../packages/capability-vault/src/internal/vault.mjs';
registerHooks({ resolve(specifier, context, next) {
  try { return next(specifier,context); } catch(e) {
    for (const ext of ['.ts','.mjs','.js']) { try { return next(specifier+ext,context); } catch {} }
    throw e;
  }
} });
globalThis.window = {setTimeout};
const paths = ['Home.md','notes/One.md','a/Duplicate.md','b/Duplicate.md'];
assert.deepEqual(resolveNote('One#Heading','notes/Current.md',paths), ['notes/One.md']);
assert.deepEqual(resolveNote('../Home.md','notes/One.md',paths), ['Home.md']);
assert.deepEqual(resolveNote('../../Home','notes/One.md',paths), []);
assert.deepEqual(resolveNote('Duplicate','Home.md',paths), ['a/Duplicate.md','b/Duplicate.md']);
assert.deepEqual(resolveNote('javascript:alert(1)','Home.md',paths), []);
assert.deepEqual(resolveNote('%ZZ','Home.md',paths), []);
assert.deepEqual(resolveNote('#Title','Home.md',paths), ['Home.md']);
assert.deepEqual(noteLinks('[[Home|Alias]]\n[One](notes/One.md)\n```md\n[[Ignored]]\n```\n`[[Code]]`'), ['Home','notes/One.md']);
assert.equal(searchNotes([{path:'Home.md',text:'# Title\nneedle here'}],'needle')[0].line,2);
assert.equal(searchNotes(Array.from({length:120},(_,i)=>({path:`${i}.md`,text:'needle'})),'needle').length,100);

const { createPinia,setActivePinia } = await import('pinia');
const { bridge } = await import('../src/bridge.ts');
const { useDatabaseStore } = await import('../src/capabilities/database/state/useDatabaseStore.ts');
const { useGitStore } = await import('../src/capabilities/git/state/useGitStore.ts');
setActivePinia(createPinia());
const cfg = {id:'local',name:'Local',kind:'sqlite',database:'/home/user/test.db',host:null,port:null,username:null,ssl_mode:'disable',allow_write:false,production_hint:false,enabled:true,created_at:'',updated_at:''};
bridge.dbListConnections = async () => [cfg];
const pending = new Map();
const result = value => ({query_id:'test',columns:['value'],rows:[[{i64:value}]],row_count:1,truncated:false,field_truncated:false,elapsed_ms:1});
bridge.dbQuery = async p => {
  if (p.sql.includes('sqlite_schema')) return {...result(1),rows:[]};
  return new Promise((resolve,reject) => pending.set(p.sql,{resolve,reject,id:p.query_id}));
};
let cancelled;
bridge.dbCancel = async id => {cancelled=id;return true;};
const db = useDatabaseStore();
await db.refreshConnections(); db.selectConnection(db.connections[0]);
db.sql='SELECT 1';db.requestRun();const first=db.document;
db.newDocument();db.sql='SELECT 2';db.requestRun();const second=db.document;
assert.notEqual(first.queryId,second.queryId);
assert.equal(first.busy,true);assert.equal(second.busy,true);
await db.cancelQuery();assert.equal(cancelled,second.queryId);
pending.get('SELECT 2').resolve(result(2));await new Promise(setImmediate);
assert.equal(second.result.rows[0][0],'2');assert.equal(first.busy,true);
pending.get('SELECT 1').resolve(result(1));await new Promise(setImmediate);
assert.equal(first.result.rows[0][0],'1');assert.equal(second.result.rows[0][0],'2');
db.closeDocument(second.id);assert.equal(db.closePending,second.id);
db.closeDocument(second.id,true);assert.equal(db.documents.length,1);
cfg.allow_write=true;await db.refreshConnections();db.sql='DELETE FROM things';db.requestRun();
assert.equal(pending.has('DELETE FROM things'),false);assert.equal(db.confirmOpen,true);
db.cancelConfirm();assert.equal(db.confirmOpen,false);

const git = useGitStore();git.repoId='one';
const diffs = [];
bridge.gitDiff = () => new Promise(resolve=>diffs.push(resolve));
const old=git.loadDiff('old');const latest=git.loadDiff('new');
diffs[1]({hunks:[{file:'new'}],more:false});await latest;
diffs[0]({hunks:[{file:'old'}],more:false});await old;
assert.equal(git.diff.hunks[0].file,'new');
console.log('WORKBENCH_LOGIC=PASS (Vault boundaries/search, independent SQL lifecycle/confirmation/cancel, stale Git replies)');
