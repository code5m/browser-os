// Actual Vue panel and script public adapter; native process transport is simulated.
const assert=require('node:assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const base=process.env.CODING_UI_URL||'http://127.0.0.1:1433';
(async()=>{
 const source=await(await fetch(base+'/src/capabilities/agent/ui/CodingCommandPanel.vue')).text();
 const vue=source.match(/from ["']([^"']+\/vue\.js\?[^"']+)["']/)?.[1];assert.ok(vue);
 const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox']});
 try{
  const page=await browser.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
  const html=`<div id="app"></div><script type="module">
  import {createApp,h,ref} from '${vue}';import Panel from '/src/capabilities/agent/ui/CodingCommandPanel.vue';import {bridge} from '/src/bridge.ts';import {setCapabilityRuntime} from '/src/capability/runtimeSingleton.ts';import {scriptLifecycleSnapshot} from '/src/capabilities/script/lifecycle.ts';import {commandWorkbench} from '/src/capabilities/script/public.ts';
  const calls=[],attached=[],mounted=ref(true);let active=false,terminal=false,output='fixture output',failNext=false;
  setCapabilityRuntime({get:id=>id==='script'?{state:active?'ACTIVE':'SUSPENDED',enabled:active}:null});
  const snapshot=()=>({run_id:'run-fixture',script_id:'snippet-fixture',status:terminal?'cancelled':'running',output_tail:output,exit_code:null,truncated:false});
  bridge.snippetList=async()=>{calls.push('list');return [{id:'snippet-fixture',name:'Fixture check',enabled:true,interpreter:'node',argv:['check.js','{target}'],dangerous:true,timeout_secs:60,params:[{name:'target',label:'Target',default:'src',required:true,param_type:'string',options:[],secret:false}]},{id:'disabled',name:'Disabled',enabled:false,params:[]}];};
  bridge.runCommand=async(id,values)=>{calls.push({action:'run',id,values});if(failNext){failNext=false;throw Error('fixture transport failure');}terminal=false;return snapshot();};bridge.scriptStatus=async()=>snapshot();bridge.cancelScript=async id=>{calls.push({action:'cancel',id});terminal=true;};
  window.fixture={calls,attached,mounted,activate:()=>active=true,deactivate:()=>active=false,failNext:()=>failNext=true,large:()=>output='中文😀'.repeat(12000)+'末尾',lifecycle:scriptLifecycleSnapshot,commandWorkbench};createApp({setup:()=>()=>mounted.value?h(Panel,{onContext:item=>attached.push(item)}):null}).mount('#app');</script>`;
  await page.route('**/__command_test',r=>r.fulfill({contentType:'text/html; charset=utf-8',body:html}));await page.goto(base+'/__command_test');
  await page.getByRole('status').filter({hasText:'无法加载'}).waitFor();assert.equal(await page.evaluate(()=>window.fixture.calls.length),0);
  await page.evaluate(()=>window.fixture.activate());await page.getByRole('button',{name:'刷新命令',exact:true}).click();
  const select=page.getByRole('combobox',{name:'已保存的命令片段',exact:true});await select.selectOption('snippet-fixture');assert.equal(await select.locator('option').count(),2);
  const run=page.getByRole('button',{name:'执行已审核命令',exact:true});assert.equal(await run.isDisabled(),true);
  const approval=page.getByRole('checkbox');await approval.check();await page.getByLabel('Target',{exact:false}).fill('tests');assert.equal(await approval.isChecked(),false);
  await approval.check();await page.getByRole('button',{name:'刷新命令',exact:true}).click();assert.equal(await approval.isChecked(),false);
  await page.evaluate(()=>window.fixture.deactivate());
  assert.equal(await page.evaluate(async()=>{try{await window.fixture.commandWorkbench.run('snippet-fixture',{});return false;}catch{return true;}}),true);
  await page.evaluate(()=>window.fixture.activate());await approval.check();await run.click();await page.getByRole('button',{name:'取消命令运行',exact:true}).waitFor();assert.equal(await approval.isChecked(),false);
  assert.equal(await page.evaluate(()=>window.fixture.lifecycle().trackedRuns),1);
  const call=await page.evaluate(()=>window.fixture.calls.find(x=>x.action==='run'));assert.deepEqual(call,{action:'run',id:'snippet-fixture',values:{target:'tests'}});
  assert.equal(await page.evaluate(()=>window.fixture.attached.length),0);
  await page.evaluate(()=>window.fixture.large());await page.getByRole('button',{name:'取消命令运行',exact:true}).click();await page.getByText('状态：cancelled',{exact:false}).waitFor();assert.equal(await page.evaluate(()=>window.fixture.lifecycle().trackedRuns),0);
  await page.getByRole('button',{name:'将此输出加入模型上下文',exact:true}).click();assert.equal(await page.evaluate(()=>window.fixture.attached.length),1);
  const text=await page.evaluate(()=>window.fixture.attached[0].text);assert.ok(new TextEncoder().encode(text).length<=16000);assert.match(text,/输出已截断/);assert.ok(text.endsWith('末尾'),JSON.stringify(text.slice(-60)));assert.ok(!text.includes('\uFFFD'));
  await page.evaluate(()=>window.fixture.failNext());await approval.check();await run.click();await page.getByRole('status').filter({hasText:'状态未知'}).waitFor();
  assert.equal(await page.getByRole('button',{name:'将此输出加入模型上下文',exact:true}).count(),0,'old output must not be presented as the failed new run');
  await approval.check();await run.click();await page.getByRole('button',{name:'取消命令运行',exact:true}).waitFor();await page.evaluate(()=>window.fixture.mounted.value=false);
  await page.waitForFunction(()=>window.fixture.calls.filter(x=>x.action==='cancel').length===2);
  assert.deepEqual(errors,[]);console.log('CODING_COMMAND_UI=PASS inactive-denied/approval-revoked/execute/poll/cancel/explicit-context/unmount-cleanup; simulated native transport');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
