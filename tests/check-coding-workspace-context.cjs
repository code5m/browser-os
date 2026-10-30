// Real public adapters and Pinia owners. Native Git transport is simulated; no database query.
const assert=require('node:assert/strict');const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');const base=process.env.CODING_UI_URL||'http://127.0.0.1:1433';
(async()=>{
 const source=await(await fetch(base+'/src/capabilities/database/state/useDatabaseStore.ts')).text();const pinia=source.match(/from ["']([^"']+\/pinia\.js\?[^"']+)["']/)?.[1];assert.ok(pinia);
 const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox']});
 try{
  const page=await browser.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.route('**/__context_test',r=>r.fulfill({contentType:'text/html',body:`<div id="app"></div><script type="module">
  import {createPinia,setActivePinia} from '${pinia}';import {bridge} from '/src/bridge.ts';import {setCapabilityRuntime} from '/src/capability/runtimeSingleton.ts';
  import {readCodingGitContext} from '/src/capabilities/git/public.ts';import {readCodingDatabaseContext,useDatabaseStore} from '/src/capabilities/database/public.ts';import {readCodingBrowserContext,useBrowserStore} from '/src/capabilities/browser/public.ts';
  setActivePinia(createPinia());let active=false;setCapabilityRuntime({get:()=>({state:active?'ACTIVE':'SUSPENDED',enabled:active})});const calls=[];
  bridge.gitLocalInspect=async root=>{calls.push(root);return {root,branch:'fixture',head:'abc',has_conflicts:false,files:[],truncated:false};};bridge.dbQuery=async()=>{throw Error('unexpected database query');};
  window.fixture={calls,activate:()=>active=true,git:readCodingGitContext,database:readCodingDatabaseContext,browser:readCodingBrowserContext,prepare:()=>{const db=useDatabaseStore();db.activeId='fixture-db';db.schema=[{name:'fixture_table',type:'table'}];db.schemaLoaded=true;const browser=useBrowserStore();browser.tabs.push({id:'a',title:'Fixture tab',url:'https://person:fixture-pass@example.invalid/path?token=fixture-token#fragment'},{id:'b',title:'Internal',url:'file:///private/fixture'});}};
  </script>`}));await page.goto(base+'/__context_test');await page.waitForFunction(()=>!!window.fixture);
  const denied=await page.evaluate(async()=>{const f=window.fixture;const results=[];for(const fn of [()=>f.git('/fixture'),()=>f.database(),()=>f.browser()]){try{await fn();results.push(false);}catch{results.push(true);}}return results;});assert.deepEqual(denied,[true,true,true]);
  await page.evaluate(()=>{window.fixture.activate();window.fixture.prepare();});
  const result=await page.evaluate(async()=>({git:await window.fixture.git('/fixture'),database:window.fixture.database(),browser:window.fixture.browser(),calls:window.fixture.calls}));
  assert.equal(result.git.branch,'fixture');assert.deepEqual(result.calls,['/fixture']);assert.deepEqual(result.database.objects,[{name:'fixture_table',type:'table'}]);
  assert.equal(result.browser.tabs[0].url,'https://example.invalid/path');assert.equal(result.browser.tabs[1].url,'[internal or unavailable]');assert.ok(!JSON.stringify(result).includes('fixture-pass'));assert.ok(!JSON.stringify(result).includes('fixture-token'));
  assert.deepEqual(errors,[]);console.log('CODING_WORKSPACE_CONTEXT=PASS capability-guards/git-public-adapter/database-no-query/browser-url-redaction; simulated Git transport');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
