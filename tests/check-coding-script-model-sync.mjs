import assert from 'node:assert/strict';
import {createIncrementalScriptModelSync} from '../src/capabilities/agent/model/codingScriptModelSync.mjs';

function model(name, language='typescript') {
  let version=1, disposed=false;
  const uri={toString:()=> 'browseros-coding://session/'+name};
  return {uri, getLanguageId:()=>language, getVersionId:()=>version,
    isDisposed:()=>disposed, edit:()=>++version, dispose:()=>{disposed=true;}};
}
const sync=createIncrementalScriptModelSync();
const calls=[];
const send=async(language,uris)=>{calls.push({language,paths:uris.map(uri=>uri.toString())});return true;};
const first=model('a.ts'),second=model('b.ts'),third=model('c.js','javascript');
const markdown=model('readme.md','markdown');
assert.deepEqual(await sync.sync([first,second,markdown],send),{eligible:2,submitted:2,reused:0});
assert.deepEqual(calls.map(call=>[call.language,call.paths.length]),[['typescript',2]]);
assert.deepEqual(await sync.sync([first,second],send),{eligible:2,submitted:0,reused:2});
assert.equal(calls.length,1,'unchanged switches must not call a worker again');
second.edit();
assert.deepEqual(await sync.sync([first,second],send),{eligible:2,submitted:1,reused:1});
assert.deepEqual(calls.at(-1).paths,[second.uri.toString()],'only edited model is sent');
const replacement=model('a.ts');
assert.deepEqual(await sync.sync([replacement,second],send),{eligible:2,submitted:1,reused:1});
assert.deepEqual(calls.at(-1).paths,[first.uri.toString()],'reopened URI with new model must be sent');
assert.deepEqual(await sync.sync([second],send),{eligible:1,submitted:0,reused:1});
assert.deepEqual(await sync.sync([replacement,second],send),{eligible:2,submitted:1,reused:1},'closed model must be forgotten');

// TS and JS workers each need all opened TS/JS models for cross-language imports.
assert.deepEqual(await sync.sync([replacement,second,third],send),{eligible:3,submitted:4,reused:2});
assert.equal(calls.at(-1).language,'javascript');
assert.equal(calls.at(-1).paths.length,3,'new JavaScript worker receives all open script models');
third.edit();
assert.deepEqual(await sync.sync([replacement,second,third],send),{eligible:3,submitted:2,reused:4});
assert.deepEqual(calls.slice(-2).map(call=>[call.language,call.paths]),[
  ['typescript',[third.uri.toString()]],['javascript',[third.uri.toString()]]
]);
sync.invalidate();
assert.deepEqual(await sync.sync([replacement,second,third],send),{eligible:3,submitted:6,reused:0},'worker restart must resync both workers');
second.dispose();
assert.deepEqual(await sync.sync([replacement,second,third],send),{eligible:2,submitted:0,reused:4},'disposed models must not sync');

const failing=createIncrementalScriptModelSync();
await assert.rejects(failing.sync([first],async()=>{throw new Error('worker failed');}),/worker failed/);
assert.equal((await failing.sync([first],send)).submitted,1,'failed requests are retried');
const declined=createIncrementalScriptModelSync();
assert.equal((await declined.sync([first],async()=>false)).submitted,0,'closed worker must not commit cache');
assert.equal((await declined.sync([first],send)).submitted,1,'closed worker must retry');

const concurrent=createIncrementalScriptModelSync();
const concurrentCalls=[];
const push=async(_language,uris)=>{concurrentCalls.push(uris.length);return true;};
await Promise.all([concurrent.sync([first],push),concurrent.sync([first],push)]);
assert.deepEqual(concurrentCalls,[1],'parallel switches must be single-flight');

const interrupted=createIncrementalScriptModelSync();
let finish,notify;
const started=new Promise(resolve=>{finish=resolve;});
const inFlight=new Promise(resolve=>{notify=resolve;});
const pending=interrupted.sync([first],async()=>{notify();await started;return true;});
await inFlight;
interrupted.invalidate();
finish();
await pending;
assert.equal((await interrupted.sync([first],send)).submitted,1,'worker reset while syncing must not cache stale state');

const editedDuringSend=createIncrementalScriptModelSync();
await editedDuringSend.sync([replacement],async()=>{replacement.edit();return true;});
assert.equal((await editedDuringSend.sync([replacement],send)).submitted,1,'revision edited in-flight must resync');

// Synthetic work-count baseline: not a time, memory or GUI benchmark.
const many=Array.from({length:40},(_,i)=>model('file-'+i+'.ts'));
const benchmark=createIncrementalScriptModelSync();
let requested=0;
for(let turn=0;turn<20;turn++)await benchmark.sync(many,async(_language,uris)=>{requested+=uris.length;return true;});
assert.equal(requested,40);
console.log('CODING_SCRIPT_MODEL_SYNC_PASS baseline_uri_requests=800 incremental_uri_requests='+requested+' (synthetic counts, not CPU or GUI timings)');
