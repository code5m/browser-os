#!/usr/bin/env node
// Real Vue SFC setup and actual Pinia owner; only native IO is replaced.
// These assert correct behavior, unlike the original bug-reproduction scripts.
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, unlinkSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { parse, compileScript } from '@vue/compiler-sfc';
import { build } from 'esbuild';
import { reactive, nextTick, effectScope } from 'vue';
import { createPinia, setActivePinia } from 'pinia';
const root=fileURLToPath(new URL('../',import.meta.url));
const temporary=[];
let assertions=0;
function eq(actual,expected){assert.deepEqual(actual,expected);assertions++;}
function deferred(){let resolve;const promise=new Promise(r=>{resolve=r;});return {promise,resolve};}
async function compile(contents, resolveDir, fixtureName, filter, fixtureSource){
  const output=root+`scripts/.pr35-fixture-${process.pid}-${temporary.length}.mjs`;
  const result=await build({stdin:{contents,resolveDir,loader:'ts'},bundle:true,write:false,platform:'node',format:'esm',external:['vue','pinia'],plugins:[{name:fixtureName,setup(b){b.onResolve({filter},args=>({path:args.path,namespace:fixtureName}));b.onLoad({filter:/.*/,namespace:fixtureName},args=>({contents:fixtureSource(args.path),loader:'js'}));}}]});
  temporary.push(output);writeFileSync(output,result.outputFiles[0].text);
  return await import(pathToFileURL(output).href);
}
const page=id=>({editable:true,table:{columns:[{name:'value',generated:false,binary:false}]},rows:[{row_id:id+'-row',values:['original']}],has_more:false});
const table=id=>({name:id,offset:0,filters:[],order:[],page:page(id),edits:{},inserts:[],removed:[],ticket:null,cell:null,notice:''});
try {
  const filename=root+'src/capabilities/database/ui/TableDataEditor.vue';
  const {descriptor}=parse(readFileSync(filename,'utf8'),{filename});
  const component=(await compile(compileScript(descriptor,{id:'pr35-table-regressions'}).content,root+'src/capabilities/database/ui','table-native-fixture',/state\/useDatabaseStore$/,()=> 'export const useDatabaseStore=()=>globalThis.__pr35Table;')).default;
  for(const scenario of ['switch','closed','failed','replaced']){
    const db=reactive({activeDocument:'A',documents:['A','B'].map(id=>({id,connId:id,tableData:table(id),busy:false,closing:false,closeFailed:false,transaction:'idle'})),configs:['A','B'].map(id=>({id,kind:'sqlite'}))});
    Object.defineProperty(db,'document',{get:()=>db.documents.find(d=>d.id===db.activeDocument)});
    Object.defineProperty(db,'busy',{get:()=>db.document.busy});
    const calls=[],pending=deferred();
    db.tableOperation=async(action,request,confirmed,id)=>{
      const doc=db.documents.find(d=>d.id===id);if(!doc)return null;
      calls.push([action,id]);doc.busy=true;
      try{if(action==='table_apply'){const reply=await pending.promise;if(reply)doc.transaction='read_write';return reply;}if(action==='table_read')return page(id);throw new Error('Unexpected operation');}
      finally{doc.busy=false;}
    };
    globalThis.__pr35Table=db;
    const scope=effectScope();
    try {
      const ui=scope.run(()=>component.setup({}, {expose(){}}));
      const a=db.documents[0],b=db.documents[1];
      a.tableData.edits={'A-row':{value:'applied A'}};
      a.tableData.ticket={ticket:'A-approved'};ui.acknowledged.value=true;
      b.tableData.edits={'B-row':{value:'UNSAVED B'}};
      b.tableData.inserts=[{id:'B-new',values:{value:'new B'}}];b.tableData.removed=['B-deleted'];
      const before=JSON.stringify(b.tableData);
      const oldTable=a.tableData;
      const running=ui.apply();
      db.activeDocument='B';await nextTick();
      if(scenario==='closed')db.documents=db.documents.filter(d=>d.id!=='A');
      if(scenario==='replaced')a.tableData=table('replacement');
      pending.resolve(scenario==='failed'?null:{applied:1,transaction:'read_write'});
      await running;await nextTick();
      eq(JSON.stringify(b.tableData),before);
      eq(calls.some(([op,id])=>op==='table_read'&&id==='B'),false);
      eq(calls.some(([op,id])=>op==='table_read'&&id==='A'),scenario==='switch');
      if(scenario==='failed')eq(oldTable.edits,{'A-row':{value:'applied A'}});
      if(scenario==='replaced')eq(a.tableData.edits,{});
    }finally{scope.stop();}
  }
  {
    const db=globalThis.__pr35Table;
    const scope=effectScope();
    try{const ui=scope.run(()=>component.setup({}, {expose(){}}));const before=JSON.stringify(db.document.tableData);await ui.load(0);eq(JSON.stringify(db.document.tableData),before);}
    finally{scope.stop();}
  }
  const storeModule=await compile(readFileSync(root+'src/capabilities/database/state/useDatabaseStore.ts','utf8'),root+'src/capabilities/database/state','session-native-fixture',/(bridge|\/lifecycle)$/,
    path=>path.endsWith('bridge')?'export const bridge=new Proxy({}, {get:(_,key)=>globalThis.__pr35Session.bridge[key]});':'export const databaseLifecycleSnapshot=()=>globalThis.__pr35Session.snapshot;export const registerDatabaseCleanup=fn=>globalThis.__pr35Session.cleanups.push(fn);');
  for(const scenario of ['throws','pending','concurrent','active-query','fatal-error']){
    const callbacks=[],sessions=new Map();let attempts=0;
    const pending=deferred();
    const cfg={id:'A',name:'A',kind:'sqlite',database:'/data/A.db',host:null,port:null,username:null,ssl_mode:'disable',allow_write:false,production_hint:false,enabled:true};
    const bridge={
      dbListConnections:async()=>[cfg],dbConnect:async()=>({conn_id:'A'}),dbDisconnect:async()=>true,
      dbQuery:async()=>({columns:['name','type'],rows:[],row_count:0}),dbCancel:async()=>true,
      dbStudioOpen:async()=>{sessions.set('owned-A',{});return {session_id:'owned-A',transaction:'idle'};},
      dbStudioCall:async()=>{throw JSON.stringify({kind:'database_session_fault',code:'DB_QUERY_HARD_TIMEOUT',message:'Driver connection unusable',transaction:'unknown',discard_connection:true});},
      dbStudioClose:async id=>{attempts++;if(scenario==='pending')return {closed:false};if(scenario==='concurrent')await pending.promise;if(scenario==='throws'||scenario==='active-query')throw new Error('NATIVE_CLOSE_FAILED');sessions.delete(id);return {closed:true};}
    };
    globalThis.__pr35Session={bridge,snapshot:{active:true,generation:1},cleanups:callbacks};
    setActivePinia(createPinia());const store=storeModule.useDatabaseStore();await store.refreshConnections();store.selectConnection(store.connections[0]);await nextTick();
    store.document.sessionId='owned-A';sessions.set('owned-A',{});
    if(scenario==='fatal-error'){
      await store.requestRunText('SELECT 1');eq(store.document.transaction,'unknown');eq(store.document.closeFailed,true);
      eq(await store.releaseSession(),true);eq(store.document.sessionId,null);eq(store.document.transaction,'unknown');
      continue;
    }
    if(scenario==='concurrent'){
      const one=store.releaseSession(),two=store.releaseSession();eq(attempts,1);pending.resolve();eq(await one,true);eq(await two,true);eq(sessions.size,0);continue;
    }
    if(scenario==='active-query'){store.document.busy=true;store.document.queryId='in-flight-query';}
    globalThis.__pr35Session.snapshot={active:false,generation:2};
    const originalTimeout=globalThis.setTimeout;
    if(scenario==='pending')globalThis.setTimeout=callback=>{queueMicrotask(callback);return 0;};
    try{
      const results=await Promise.allSettled(callbacks.map(callback=>callback()));
      eq(results.map(r=>r.status),['rejected']);eq(sessions.size,1);eq(store.document.sessionId,'owned-A');eq(store.document.closeFailed,true);eq(store.document.transaction,'unknown');
      if(scenario==='pending')eq(attempts,180);
      if(scenario==='active-query')eq(store.document.busy,true);
      bridge.dbStudioClose=async id=>{sessions.delete(id);return {closed:true};};
      await callbacks[0]();eq(sessions.size,0);eq(store.document.closeFailed,false);
    }finally{globalThis.setTimeout=originalTimeout;}
  }
  console.log(`PR35_FRONTEND_REGRESSIONS=PASS assertions=${assertions} (real Vue/Pinia; native IO fixtures, not GUI or real database E2E)`);
}finally{
  for(const path of temporary){try{unlinkSync(path);}catch{}}
  delete globalThis.__pr35Table;delete globalThis.__pr35Session;
}
