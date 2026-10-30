// Real Vue/Monaco interaction, deliberately simulated LSP transport.
const assert=require('node:assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const base=process.env.CODING_UI_URL||'http://127.0.0.1:1433';
(async()=>{
 const source=await(await fetch(base+'/src/capabilities/agent/ui/CodingWorkbench.vue')).text();
 const vue=source.match(/from ["']([^"']+\/vue\.js\?[^"']+)["']/)?.[1];assert.ok(vue);
 const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox']});
 try{
  const page=await browser.newPage({viewport:{width:1600,height:1000}}),errors=[];
  page.on('pageerror',error=>errors.push(error.message));
  await page.route('**/__lsp_test',route=>route.fulfill({contentType:'text/html',body:`<style>html,body,#app{height:100%;margin:0}</style><div id="app"></div><script type="module">
   import {createApp} from '${vue}';import Workbench from '/src/capabilities/agent/ui/CodingWorkbench.vue';import {bridge} from '/src/bridge.ts';
   const calls=[];window.fixture={calls,resolved:false};
   bridge.aiService=async()=>({profiles:[{id:'fixture',name:'Fixture',model:'fixture-model',kind:'chat'}],bindings:{default:'fixture'}});
   bridge.codingService=async request=>{calls.push(request);
    if(request.action==='tree')return {entries:[{path:'main.rs',name:'main.rs',kind:'file'}]};
    if(request.action==='session')return {root:request.root,session_id:'fixture',messages:[],changes:[],operations:[]};
    if(request.action==='read')return request.path==='math.rs'?{content:'pub fn square(x:i32)->i32{x*x}',hash:window.fixture.stale?'b'.repeat(64):'a'.repeat(64)}:{content:'fn square(x:i32)->i32{x*x}\\nfn main(){let answer=square(4);}',hash:'fixture'};
    if(request.action==='lsp_status')return {profiles:[{path:'/trusted/rust-analyzer',name:'Rust fixture'}],sandbox_available:true};
    if(request.action==='chat'||request.action==='resolve_operation'){
     if(request.action==='resolve_operation')window.fixture.resolved=true;
     return {root:request.root,session_id:'fixture',status:window.fixture.resolved?'complete':'approval_required',messages:window.fixture.resolved?[{role:'assistant',content:'Using reviewed diagnostics'}]:[],changes:[],operations:[{id:'diagnostic-operation',kind:'code_diagnostics',input:{path:'main.rs'},status:window.fixture.resolved?'succeeded':'pending'}]};
    }
    if(request.action==='lsp_request'){
     if(window.fixture.failOnce){window.fixture.failOnce=false;throw Error('LSP_BUSY');}
     const response={document_hash:'a'.repeat(64),scope:request.module_root?'isolated_rust_module_sources':'isolated_current_rust_document',diagnostics:[{range:{start:{line:1,character:14},end:{line:1,character:20}},message:'fixture unused variable',severity:2}],result:null};
     if(request.method==='completion')response.result={items:[{label:'square_lsp_fixture',kind:3,insertText:'square'}]};
     if(request.method==='definition')response.result=[{uri:'file:///work/document.rs',range:{start:{line:0,character:3},end:{line:0,character:9}}}];
     if(request.method==='definition'&&request.module_root){response.result=[{uri:'file:///work/math.rs',range:{start:{line:0,character:7},end:{line:0,character:13}}}];response.definition_targets=[{path:'math.rs',uri:'file:///work/math.rs',hash:'a'.repeat(64),range:response.result[0].range}];}
     if(request.method==='hover')response.result={contents:{kind:'plaintext',value:'fn square(x: i32) -> i32'}};
     if(window.fixture.hold)return new Promise((resolve,reject)=>{window.fixture.finish=()=>resolve(response);window.fixture.failHeld=()=>reject(Error('LSP_BUSY'));});
     return response;
    }return {};
   };createApp(Workbench).mount('#app');</script>`}));
  await page.goto(base+'/__lsp_test');
  await page.getByLabel('项目目录',{exact:true}).fill('/fixture');await page.getByRole('button',{name:'打开项目',exact:true}).click();
  await page.locator('.file-row').click();await page.locator('.coding-code[data-editor-ready="true"]').waitFor();
  await page.evaluate(async()=>{window.fixtureApi=await import(performance.getEntriesByType('resource').map(item=>item.name).find(name=>/monaco-editor.*editor.*api/.test(name)));});
  assert.equal(await page.evaluate(()=>window.fixture.calls.filter(r=>r.action==='lsp_request').length),0,'no implicit server start');
  await page.locator('.language-service summary').click();
  await page.getByRole('button',{name:'检测本机语言服务',exact:true}).click();
  assert.ok(await page.getByRole('button',{name:'启用语言服务',exact:true}).isDisabled());
  await page.getByLabel('批准当前项目语言服务',{exact:true}).check();
  await page.evaluate(()=>{window.fixture.failOnce=true;});
  await page.getByRole('button',{name:'启用语言服务',exact:true}).click();
  await page.waitForFunction(()=>window.fixture.calls.some(r=>r.action==='lsp_request'&&r.method==='diagnostics'));
  await page.locator('.language-service summary').filter({hasText:'语言服务正忙'}).waitFor();
  const monaco=async(fn)=>page.evaluate(async code=>{
   const url=performance.getEntriesByType('resource').map(item=>item.name).find(name=>/monaco-editor.*editor.*api/.test(name));
   const api=await import(url);const editor=api.editor.getEditors().find(e=>!e.getOption(1));return Function('api','editor',code)(api,editor||api.editor.getEditors()[0]);
  },fn);
  await monaco('editor.setPosition({lineNumber:2,column:23});editor.focus();');
  await page.keyboard.press('Control+Space');
  await page.getByText('square_lsp_fixture',{exact:true}).waitFor();await page.keyboard.press('Escape');
  await monaco('editor.setPosition({lineNumber:2,column:23});editor.focus();');await page.keyboard.press('F12');
  await page.waitForFunction(()=>window.fixtureApi.editor.getEditors().some(e=>e.getPosition()?.lineNumber===1&&e.getPosition()?.column===4));
  assert.ok((await monaco('return api.editor.getModelMarkers({owner:"coding-lsp"}).length;'))>0,'diagnostic markers are real Monaco markers');
  const point=await monaco('const at=editor.getScrolledVisiblePosition({lineNumber:1,column:5}),rect=editor.getDomNode().getBoundingClientRect();return {x:rect.left+at.left+2,y:rect.top+at.top+at.height/2};');
  await page.mouse.move(point.x,point.y);
  await page.locator('.monaco-hover').filter({hasText:'fn square(x: i32) -> i32'}).waitFor();
  await page.getByRole('button',{name:'停用语言服务',exact:true}).click();
  await page.locator('.monaco-hover').filter({hasText:'fn square(x: i32) -> i32'}).waitFor({state:'hidden'});
  await page.waitForFunction(()=>window.fixtureApi.editor.getModelMarkers({owner:'coding-lsp'}).length===0);
  await page.getByLabel('批准读取 Rust 模块树',{exact:true}).check();await page.getByLabel('Rust 模块根文件',{exact:true}).fill('main.rs');
  await page.getByLabel('批准当前项目语言服务',{exact:true}).check();await page.getByRole('button',{name:'启用语言服务',exact:true}).click();
  await monaco('editor.setPosition({lineNumber:2,column:23});editor.focus();');await page.keyboard.press('F12');
  await page.locator('.editor-tab.selected').getByRole('button',{name:'math.rs',exact:true}).waitFor();
  await page.locator('.coding-code[data-editor-ready="true"]').waitFor();
  assert.equal(await monaco('return editor.getModel().getValue();'),'pub fn square(x:i32)->i32{x*x}');
  assert.equal(await monaco('return editor.getPosition().column;'),8,'cross-file definition selects the actual target column');
  await monaco('editor.setPosition({lineNumber:1,column:1});');
  await page.locator('.editor-tab').getByRole('button',{name:'main.rs',exact:true}).click();await page.locator('.coding-code[data-editor-ready="true"]').waitFor();
  await page.locator('.editor-tab').getByRole('button',{name:'math.rs',exact:true}).click();await page.locator('.coding-code[data-editor-ready="true"]').waitFor();
  assert.equal(await monaco('return editor.getPosition().column;'),1,'a consumed navigation must not override later restored view state');
  await page.locator('.editor-tab').getByRole('button',{name:'main.rs',exact:true}).click();await page.locator('.coding-code[data-editor-ready="true"]').waitFor();
  await page.evaluate(()=>{window.fixture.stale=true;});await monaco('editor.setPosition({lineNumber:2,column:23});editor.focus();');await page.keyboard.press('F12');
  await page.getByRole('status').filter({hasText:'目标文件在分析后已变化'}).waitFor();
  assert.equal(await page.locator('.editor-tab.selected').getByRole('button',{name:'main.rs',exact:true}).count(),1);
  await page.getByRole('button',{name:'停用语言服务',exact:true}).click();await page.getByLabel('批准读取 Rust 模块树',{exact:true}).uncheck();
  await page.getByLabel('智能体模式',{exact:true}).selectOption('build');await page.getByLabel('智能体指令',{exact:true}).fill('Check code diagnostics');await page.getByRole('button',{name:'发送 ↑',exact:true}).click();
  const review=page.locator('.operation-review');await review.waitFor();await review.getByRole('checkbox').check();
  assert.ok(await review.getByRole('button',{name:'执行本次代码诊断',exact:true}).isDisabled(),'model request cannot bypass project language-service approval');
  await page.getByLabel('批准当前项目语言服务',{exact:true}).check();await page.getByRole('button',{name:'启用语言服务',exact:true}).click();
  await review.getByRole('button',{name:'执行本次代码诊断',exact:true}).click();await review.getByLabel('待记录的操作结果',{exact:true}).waitFor();
  assert.equal(await page.evaluate(()=>window.fixture.calls.filter(request=>request.action==='resolve_operation').length),0,'diagnostics must be reviewed before sending to the model');
  const recorded=JSON.parse(await review.getByLabel('待记录的操作结果',{exact:true}).innerText());assert.equal(recorded.compilation_verified,false);assert.equal(recorded.warnings,1);assert.equal(recorded.document_hash,'a'.repeat(64));
  assert.ok(await review.getByRole('button',{name:'执行本次代码诊断',exact:true}).isDisabled(),'each approval runs only once');
  await review.getByRole('button',{name:'记录结果供智能体续跑',exact:true}).click();await page.getByRole('button',{name:'根据审批结果继续',exact:true}).click();await page.getByText('Using reviewed diagnostics',{exact:true}).waitFor();
  await page.getByRole('button',{name:'停用语言服务',exact:true}).click();
  // An in-flight response after revocation must not resurrect diagnostics.
  await page.getByLabel('批准当前项目语言服务',{exact:true}).check();await page.evaluate(()=>{window.fixture.hold=true;});
  await page.getByRole('button',{name:'启用语言服务',exact:true}).click();await page.waitForFunction(()=>typeof window.fixture.finish==='function');
  await page.getByRole('button',{name:'停用语言服务',exact:true}).click();await page.evaluate(()=>window.fixture.finish());
  await page.waitForTimeout(100);assert.equal(await monaco('return api.editor.getModelMarkers({owner:"coding-lsp"}).length;'),0);
  await page.getByLabel('批准当前项目语言服务',{exact:true}).check();await page.evaluate(()=>{window.fixture.finish=null;window.fixture.failHeld=null;window.fixture.hold=true;});
  await page.getByRole('button',{name:'启用语言服务',exact:true}).click();await page.waitForFunction(()=>typeof window.fixture.failHeld==='function');
  await page.getByRole('button',{name:'停用语言服务',exact:true}).click();await page.evaluate(()=>window.fixture.failHeld());await page.waitForTimeout(100);
  assert.match(await page.locator('.language-service summary').innerText(),/已停用/,'old errors must not replace the revoked status');
  await page.getByLabel('项目目录',{exact:true}).fill('/other');await page.getByRole('button',{name:'打开项目',exact:true}).click();
  assert.equal(await page.getByLabel('批准当前项目语言服务',{exact:true}).isChecked(),false);
  assert.ok(await page.getByRole('button',{name:'启用语言服务',exact:true}).isDisabled());
  assert.deepEqual(errors,[]);console.log('CODING_LSP_UI_PASS (real Monaco; simulated transport)');
 }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
