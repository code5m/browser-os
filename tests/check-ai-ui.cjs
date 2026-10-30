// Real Vue components; synthetic provider transport. Native HTTPS smoke is separate.
const assert=require('node:assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const base=process.env.AI_UI_URL||'http://127.0.0.1:1433';
(async()=>{
 const transformed=await (await fetch(base+'/src/components/system/AiConfiguration.vue')).text();
 const vue=transformed.match(/from ["']([^"']+\/vue\.js\?[^"']+)["']/)?.[1];assert.ok(vue);
 const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox']});
 try{
  const page=await browser.newPage({viewport:{width:1280,height:960}}),errors=[];
  page.on('pageerror',e=>errors.push(e.message));
  const html=`<div id="app"></div><script type="module">
  import {createApp,h,ref} from '${vue}';
  import Config from '/src/components/system/AiConfiguration.vue';
  import Assistant from '/src/components/system/AiAssistant.vue';
  import {bridge} from '/src/bridge.ts';
  import '/src/styles/tokens.css';
  const state={profiles:[],bindings:{},plugins:[],import_sources:[]},calls=[];
  const profile={id:'a'.repeat(24),name:'Fixture chat',model:'fixture-model',base_url:'https://provider.invalid/v1',kind:'chat',source:'fixture',credential_ref:'ai:fixture'};
  bridge.aiService=async input=>{const r=JSON.parse(JSON.stringify(input));calls.push(r);if(r.action==='list')return JSON.parse(JSON.stringify(state));if(r.action==='import'){state.profiles=[profile,{...profile,id:'b'.repeat(24),kind:'image',name:'Fixture image'}];state.bindings.default=profile.id;return {imported:2,total:2,plugin_credentials:0};}if(r.action==='bind'){if(r.id)state.bindings[r.scope]=r.id;else delete state.bindings[r.scope];return {saved:true};}if(r.action==='save'){const i=state.profiles.findIndex(p=>p.id===r.profile.id);state.profiles[i]=r.profile;return {saved:true};}if(r.action==='chat')return {model:'fixture-model',content:'Synthetic response'};return {};};
  const mode=ref('config'),context=ref('SELECT id FROM fixture_table;');window.fixture={state,calls,mode,context};
  createApp({setup:()=>()=>h('main',{style:'padding:20px;font:14px system-ui'},mode.value==='config'?h(Config):h(Assistant,{scope:'database',context:context.value}))}).mount('#app');
  </script>`;
  await page.route('**/__ai_test',r=>r.fulfill({contentType:'text/html',body:html}));
  await page.goto(base+'/__ai_test');
  await page.getByText('尚未配置模型。',{exact:false}).waitFor();
  await page.getByRole('button',{name:'一键导入本机 AI 配置',exact:true}).click();
  await page.locator('.models article strong').filter({hasText:'Fixture chat'}).waitFor();
  assert.equal(await page.locator('.models article').count(),2);
  assert.equal(await page.locator('.models article').nth(1).getByRole('button',{name:'测试连接'}).count(),0);
  await page.getByRole('combobox',{name:'Git AI 模型',exact:true}).selectOption('a'.repeat(24));
  await page.waitForFunction(()=>window.fixture.state.bindings.git);
  await page.getByRole('combobox',{name:'Git AI 模型',exact:true}).selectOption('');
  await page.waitForFunction(()=>!window.fixture.state.bindings.git);
  await page.locator('.models article').first().getByRole('button',{name:'编辑',exact:true}).click();
  assert.equal(await page.getByLabel('API 密钥',{exact:true}).inputValue(),'');
  await page.getByLabel('名称',{exact:true}).fill('Renamed fixture');
  await page.getByRole('button',{name:'保存',exact:true}).click();
  await page.locator('.models article strong').filter({hasText:'Renamed fixture'}).waitFor();
  const saved=await page.evaluate(()=>window.fixture.calls.find(r=>r.action==='save'));
  assert.equal(saved.credential,'');assert.equal(saved.profile.id,'a'.repeat(24));
  await page.evaluate(()=>window.fixture.mode.value='assistant');
  await page.getByLabel('AI 问题',{exact:true}).fill('Explain the query');
  await page.getByRole('button',{name:'发送',exact:true}).click();
  await page.getByLabel('AI 回复',{exact:true}).waitFor();
  let chat=await page.evaluate(()=>window.fixture.calls.filter(r=>r.action==='chat').at(-1));
  assert.equal(chat.message,'Explain the query');
  await page.getByRole('checkbox').check();
  await page.getByRole('button',{name:'发送',exact:true}).click();
  await page.waitForFunction(()=>window.fixture.calls.filter(r=>r.action==='chat').length===2);
  chat=await page.evaluate(()=>window.fixture.calls.filter(r=>r.action==='chat').at(-1));
  assert.ok(chat.message.includes('SELECT id FROM fixture_table;'));
  await page.evaluate(()=>window.fixture.context.value='SELECT 1;');
  await page.waitForFunction(()=>!document.querySelector('input[type=checkbox]').checked);
  assert.equal(await page.getByLabel('AI 回复',{exact:true}).count(),0);
  assert.deepEqual(errors,[]);
  console.log('AI_UI=PASS import/edit/bind/inherit/model-kind/explicit-context/context-switch; simulated transport');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
