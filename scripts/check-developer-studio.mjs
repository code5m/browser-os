#!/usr/bin/env node
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,unlinkSync} from 'node:fs';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {build} from 'esbuild';
import {createPinia,setActivePinia} from 'pinia';
import {nextTick} from 'vue';
import {lexSql,sqlStatements,selectedSql,formatSql,sqlCompletions,quoteIdentifier,exportRows,parseDelimited} from '../src/capabilities/database/model/studioSql.mjs';
import {patchHunks,applyPatchSelection,blameLines,sourceIdentity,safeGithubLink} from '../src/capabilities/git/model/studioPatch.mjs';
let checks=0;
const eq=(actual,expected)=>{assert.deepEqual(actual,expected);checks++;};
const rejects=fn=>{assert.throws(fn);checks++;};
eq(sqlStatements("SELECT ';'; -- semicolon;\nSELECT $$a;b$$;").length,2);
eq(sqlStatements('/* outer /* inner */ ; */ SELECT 1').length,1);
eq(sqlStatements('SELECT "a;b", `x;y`, [x;y]; SELECT 2').length,2);
eq(sqlStatements('-- only comment').length,0);
eq(lexSql("SELECT 'unfinished").issues[0].code,'UNCLOSED_STRING');
eq(lexSql('SELECT (1').issues[0].code,'UNCLOSED_PAREN');
eq(selectedSql('SELECT 1; SELECT 2',11),'SELECT 2');
eq(selectedSql('SELECT 1; SELECT 2',0,8),'SELECT 1');
eq(formatSql("select '; x' from test where id=2"),"select '; x'\nfrom test\nwhere id=2");
eq(formatSql('SELECT 1 -- keep comment\nFROM x'),'SELECT 1 -- keep comment\nFROM x');
eq(sqlCompletions("SELECT 'S",9),[]);
eq(sqlCompletions('SEL',3)[0].label,'SELECT');
eq(quoteIdentifier('odd"name','postgres'),'"odd""name"');
eq(parseDelimited('a,b\r\n1,"two,three"\r\n2,"line\nline"'),{columns:['a','b'],rows:[['1','two,three'],['2','line\nline']]});
eq(exportRows(['a'],[['=HYPERLINK("x")']]),'a\r\n"\'=HYPERLINK(""x"")"');
for(const input of ['a,a\n1,2','a,b\n1','a\n"unterminated'])rejects(()=>parseDelimited(input));
for(const sql of ["SELECT 'don''t;split';SELECT 2;",'SELECT $tag$;$tag$;SELECT 1','SELECT /* ; */ 1;SELECT 2'])eq(sqlStatements(sql).length,2);
for(const value of ['SELECT col::int FROM t',"SELECT a->>'name' FROM t",'SELECT -1.2e-3 FROM t',"SELECT E'a\\nb' FROM t"]){eq(lexSql(formatSql(value)).tokens.filter(t=>t.kind!=='space').map(t=>t.text),lexSql(value).tokens.filter(t=>t.kind!=='space').map(t=>t.text));}
const patch='diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,2 +1,2 @@\n one\n-old\n+new\n';
const hunk=patchHunks(patch)[0];
eq(applyPatchSelection('one\nold\n',patch,[]),'one\nold\n');
eq(applyPatchSelection('one\nold\n',patch,hunk.lines.filter(l=>l.sign!==' ').map(l=>l.id)),'one\nnew\n');
eq(applyPatchSelection('one\nold\n',patch,hunk.lines.filter(l=>l.sign==='+').map(l=>l.id)),'one\nold\nnew\n');
rejects(()=>applyPatchSelection('wrong\nold\n',patch,[]));
const noEnd='@@ -1 +1 @@\n-old\n\\ No newline at end of file\n+new\n\\ No newline at end of file\n';
eq(applyPatchSelection('old',noEnd,patchHunks(noEnd)[0].lines.map(l=>l.id)),'new');
rejects(()=>sourceIdentity({path:'/x',repo_id:'other'}));
rejects(()=>sourceIdentity({}));
eq(safeGithubLink('javascript:alert(1)'),'');
eq(safeGithubLink('https://github.com.evil.invalid/repo'),'');
eq(safeGithubLink('https://github.com/code5m/browser-os'),'https://github.com/code5m/browser-os');
const oid='a'.repeat(40);eq(blameLines(`${oid} 3 4 1\nauthor Alice\nsummary example\n\tactual line`),[{oid,original:3,line:4,author:'Alice',summary:'example',text:'actual line'}]);

// Real production store + Vue/Pinia. Only the external bridge/lifecycle are
// replaced; assertions observe the store's actual source and session behavior.
const root=fileURLToPath(new URL('../',import.meta.url));
const path=root+'scripts/.developer-studio-test-'+process.pid+'.mjs';
const configs=['A','B'].map(id=>({id,name:'Source '+id,kind:'sqlite',database:'/data/'+id+'.db',host:null,port:null,username:null,ssl_mode:'disable',allow_write:false,production_hint:false,enabled:true,created_at:'2026-10-16T00:00:00Z',updated_at:'2026-10-16T00:00:00Z'}));
const calls=[],sessions=new Map();const cleanup=[];let generation=1,active=true;
const result={columns:['value'],rows:[[{i64:7}]],row_count:1,truncated:false,field_truncated:false,elapsed_ms:1,risk:'read',production_verdict:'NonProduction'};
const bridge={
  dbListConnections:async()=>configs,
  dbConnect:async cfg=>({conn_id:cfg.id}),
  dbDisconnect:async()=>true,
  dbQuery:async request=>{calls.push(['query',request]);return {columns:['name','type'],rows:[],row_count:0};},
  dbCancel:async()=>true,
  dbStudioOpen:async connId=>{const id='session-'+(sessions.size+1)+'-'+connId;sessions.set(id,{connId,transaction:'idle'});calls.push(['open',connId]);return {session_id:id,conn_id:connId,transaction:'idle'};},
  dbStudioCall:async(id,request)=>{const session=sessions.get(id);if(!session)throw new Error('DB_SESSION_EXPIRED_OR_CLOSED');calls.push(['call',session.connId,request.action]);if(request.action==='begin')session.transaction=request.writable?'read_write':'read_only';if(['commit','rollback'].includes(request.action))session.transaction='idle';if(request.action==='batch')return {results:[result,result],transaction:session.transaction};return {result,transaction:session.transaction};},
  dbStudioClose:async id=>{calls.push(['close',id]);sessions.delete(id);return {closed:true,pending_rollback:false};},
  dbStudioCancel:async()=>({cancel_requested:true,completed:false}),
};
globalThis.__browserosDeveloperStudio={bridge,lifecycle:{databaseLifecycleSnapshot:()=>({active,generation}),registerDatabaseCleanup:fn=>cleanup.push(fn)}};
try{
  const compiled=await build({entryPoints:[root+'src/capabilities/database/state/useDatabaseStore.ts'],bundle:true,write:false,format:'esm',platform:'node',external:['vue','pinia'],plugins:[{name:'real-store-native-fixture',setup(builder){builder.onResolve({filter:/bridge$/},args=>({path:'bridge',namespace:'native-fixture'}));builder.onResolve({filter:/\/lifecycle$/},args=>({path:'lifecycle',namespace:'native-fixture'}));builder.onLoad({filter:/.*/,namespace:'native-fixture'},args=>({contents:args.path==='bridge'?'export const bridge=globalThis.__browserosDeveloperStudio.bridge;':'export const databaseLifecycleSnapshot=globalThis.__browserosDeveloperStudio.lifecycle.databaseLifecycleSnapshot;export const registerDatabaseCleanup=globalThis.__browserosDeveloperStudio.lifecycle.registerDatabaseCleanup;',loader:'js'}));}}]});
  writeFileSync(path,compiled.outputFiles[0].text);
  setActivePinia(createPinia());
  const {useDatabaseStore}=await import(pathToFileURL(path).href);
  const store=useDatabaseStore();await store.refreshConnections();
  eq(store.configs.length,2);
  store.selectConnection(store.connections[0]);store.sql='SELECT 7';const first=store.activeDocument;
  store.selectConnection(store.connections[1]);await nextTick();
  eq(store.documents.find(d=>d.id===first).connId,'A');
  eq(store.documents.find(d=>d.id===first).sql,'SELECT 7');
  eq(store.document.connId,'B');
  store.activateDocument(first);await nextTick();eq(store.form.database,'/data/A.db');
  store.setRememberQueries(true);await store.requestRunText('SELECT 7',false);
  eq(store.document.raw.rows[0][0].i64,7);eq(store.queryHistory[0].connId,'A');
  eq(calls.filter(c=>c[0]==='open').at(-1)[1],'A');
  store.selectConnection(store.connections[1]);await nextTick();store.reuseHistory(store.queryHistory[0]);await nextTick();
  eq(store.document.connId,'A');eq(store.sql,'SELECT 7');
  await store.sessionAction('begin',{writable:false,confirmed:false});eq(store.document.transaction,'read_only');
  const closedDocument=store.activeDocument;await store.closeDocument(closedDocument,true);
  eq(store.documents.some(d=>d.id===closedDocument),false);
  eq(calls.some(c=>c[0]==='close'),true);
  store.activateDocument(first);await nextTick();store.reuseHistory({sql:'SELECT secret',connection:'removed',connId:'MISSING'});
  eq(store.error.includes('静默'),true);eq(store.document.connId,'A');
  await store.requestRunText('SELECT 1; SELECT 2',false);eq(store.error.includes('单条'),true);
  await store.requestRunText('SELECT 1; SELECT 2',true);eq(store.document.batchResults.length,2);
  active=false;generation++;for(const callback of cleanup)await callback();eq(sessions.size,0);
}finally{try{unlinkSync(path);}catch{}delete globalThis.__browserosDeveloperStudio;}
const native=readFileSync(root+'src-tauri/src/capabilities/git/studio.rs','utf8');
for(const guard of ['GIT_STUDIO_CHANGED_SINCE_PREVIEW','GIT_STUDIO_READ_BASIS_CHANGED','GIT_STUDIO_PR_CHANGED','GIT_STUDIO_PREMERGE_NOT_EXECUTED']){assert(native.includes(guard));checks++;}
console.log(`DEVELOPER_STUDIO_BEHAVIOR_TESTS=PASS assertions=${checks}; native driver/process tests run separately with cargo test studio`);
