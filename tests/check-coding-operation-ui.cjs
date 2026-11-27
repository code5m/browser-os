// Coding Operation Review follows the native Coding Owner receipt contract.
// MCP and proposed commands are never dispatched directly by this UI test.
const assert=require('node:assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const base=process.env.CODING_UI_URL||'http://127.0.0.1:1433';
(async()=>{
 const source=await(await fetch(base+'/src/capabilities/agent/ui/CodingOperationReview.vue')).text();
 const vue=source.match(/from ["']([^"']+\/vue\.js\?[^"']+)["']/)?.[1];assert.ok(vue);
 const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox']});
 try{
  const page=await browser.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.route('**/__operation_test',r=>r.fulfill({contentType:'text/html; charset=utf-8',body:`<div id="app"></div><script type="module">
   import {createApp,h,ref} from '${vue}';import Review from '/src/capabilities/agent/ui/CodingOperationReview.vue';
   import {bridge} from '/src/bridge.ts';import {setCapabilityRuntime} from '/src/capability/runtimeSingleton.ts';
   setCapabilityRuntime({get:id=>id==='script'?{state:'ACTIVE',enabled:true}:null});
   // Match the native JSON IPC boundary; Vue nested reactive proxies are not structured-cloneable.
   const wire=value=>JSON.parse(JSON.stringify(value));
   const operation=ref({id:'mcp-first',kind:'mcp',input:{server_id:'fixture',tool:'write',arguments:{value:1}},status:'pending'});
   const calls=[],reviews=[],resolutions=[],dispatches=[],claims=[],receipts=[],directCommands=[];let revision='a'.repeat(64);
   bridge.codingService=async request=>{request=wire(request);calls.push(request);if(request.action!=='mcp_review')throw Error('direct MCP dispatch forbidden');
    const reference='ai:1234567890abcdef12345678:11111111-1111-4111-8111-111111111111';
    const review={action:'mcp_call',revision,credential_refs:[reference],environment_names:['FIXTURE_MODE'],command:'/usr/bin/fixture',args:['--fixture'],tool:request.tool,arguments:request.arguments,workspace:request.root,cwd:request.root,source:'Fixture MCP settings',isolation:'user_process',timeout_seconds:20,credentials:[{environment:'FIXTURE_TOKEN',name:'Fixture model',endpoint:'https://fixture.invalid/v1',reference}]};
    reviews.push(review);return review;};
   bridge.snippetList=async()=>[{id:'fixture-command',name:'Fixture',enabled:true,revision,interpreter:'node',argv:['fixture'],params:[]}];
   bridge.runCommand=async(...args)=>{directCommands.push(wire(args));throw Error('direct command dispatch forbidden');};
   const executeOwned=async(id,grant)=>{
    grant=wire(grant);calls.push({action:'execute_operation',operation_id:id,...grant});
    if(id!==operation.value.id||operation.value.status!=='pending'||receipts.some(r=>r.id===id))throw Error('AGENT_OPERATION_NOT_PENDING');
    if(id==='owner-fails')throw Error('AGENT_BUSY private-token-value');
    if(operation.value.kind==='mcp'){const reviewed=reviews.at(-1);if(grant.review_revision!==reviewed.revision||JSON.stringify(grant.approved_credential_refs)!==JSON.stringify(reviewed.credential_refs)||JSON.stringify(grant.approved_environment_names)!==JSON.stringify(reviewed.environment_names))throw Error('MCP_REVIEW_CHANGED');}
    let next;
    if(id==='command-stale'&&grant.expected_revision!==revision){next={...operation.value,status:'cancelled',result:'SNIPPET_REVIEW_CHANGED',execution:{phase:'not_dispatched'}};}
    else{dispatches.push(id);
      if(['mcp-unknown','mcp-timeout','mcp-legacy','command-unknown'].includes(id))next={...operation.value,status:'unknown',review_revision:'receipt:'+id,can_reconcile:id!=='mcp-legacy',result:id==='mcp-unknown'?'fixture tool error':id==='mcp-timeout'?'MCP_TIMEOUT_OR_CLOSED':id==='command-unknown'?'AGENT_EXTERNAL_OUTCOME_UNKNOWN':'旧记录缺少审批证据',execution:{phase:'returned'}};
      else next={...operation.value,status:'result_ready',result:id==='mcp-large'?'AGENT_OPERATION_OUTPUT_WITHHELD':'原生持久化回执：fixture',execution:{phase:'returned'}};
    }
    receipts.push(wire(next));operation.value=next;
   };
   const resolve=value=>{value=wire(value);resolutions.push(value);const op=operation.value;
    if(value.id!==op.id)throw Error('operation identity changed');
    if(value.status==='accepted'){if(op.status!=='result_ready'||value.result!=='')throw Error('native receipt required');operation.value={...op,status:'succeeded'};}
    else if(value.status.startsWith('reconciled_')){if(op.status!=='unknown'||op.can_reconcile!==true||value.review_revision!==op.review_revision||!value.result.trim())throw Error('native reconciliation evidence required');operation.value={...op,status:value.status};}
    else if(value.status==='rejected'){if(op.status!=='pending')throw Error('cannot reject dispatched operation');operation.value={...op,status:'rejected'};}
    else if(value.status==='unknown'){operation.value={...op,status:'unknown',can_reconcile:false};}
    else throw Error('unexpected result');
   };
   const claimOperation=async id=>{claims.push(id);throw Error('claim failed');};
   const runDiagnostics=async()=>{throw Error('must not run after failed claim');};
   window.fixture={operation,calls,reviews,resolutions,dispatches,claims,receipts,directCommands,rotate:()=>revision='b'.repeat(64)};
   createApp({setup:()=>()=>['pending','executing','unknown','result_ready'].includes(operation.value.status)
    ?h(Review,{key:operation.value.id,operation:operation.value,root:'/fixture',executeOwned,claimOperation,diagnosticsAvailable:true,runDiagnostics,onResolve:resolve})
    :h('p',{role:'status'},operation.value.execution?.phase==='not_dispatched'?'已确认未派发，旧审批不会重放':'已处理')}).mount('#app');
  </script>`}));
  await page.goto(base+'/__operation_test');
  const opApprove=page.getByRole('checkbox',{name:'我批准这一次工作区操作',exact:true});
  const credentialApprove=page.getByRole('checkbox',{name:'我允许本次进程取得以上凭据和配置环境变量',exact:true});
  const execute=page.getByRole('button',{name:'执行本次 MCP 请求',exact:true});
  const accept=page.getByRole('button',{name:'接受原生回执并记录结果',exact:true});
  const reviewAndApprove=async()=>{
   await page.getByRole('button',{name:'读取本次执行范围',exact:true}).click();
   await page.locator('.mcp-execution-review').waitFor();
   assert.match(await page.locator('.mcp-execution-review').innerText(),/FIXTURE_MODE/);
   assert.equal(await opApprove.isChecked(),false);assert.equal(await credentialApprove.isChecked(),false);
   await opApprove.check();assert.equal(await execute.isDisabled(),true);
   await credentialApprove.check();
  };
  await opApprove.check();assert.equal(await execute.isDisabled(),true,'unreviewed MCP must not execute');
  await reviewAndApprove();await execute.click();await accept.waitFor();
  const first=await page.evaluate(()=>({calls:window.fixture.calls,reviews:window.fixture.reviews}));
  assert.deepEqual(first.calls.map(x=>x.action),['mcp_review','execute_operation'],'native owner receives the only execution request');
  assert.deepEqual(first.calls[1],{action:'execute_operation',operation_id:'mcp-first',review_revision:first.reviews[0].revision,
   approved_credential_refs:first.reviews[0].credential_refs,approved_environment_names:['FIXTURE_MODE']});
  assert.equal(await page.getByRole('button',{name:'拒绝此请求',exact:true}).count(),0);
  await accept.click();
  assert.deepEqual(await page.evaluate(()=>window.fixture.resolutions.at(-1)),{id:'mcp-first',status:'accepted',result:''},
   'accepting a native receipt must not fabricate result text');
  assert.deepEqual(await page.evaluate(()=>window.fixture.dispatches),['mcp-first']);
  assert.equal(await execute.count(),0,'accepted receipt cannot replay');
  await page.evaluate(()=>window.fixture.operation.value={id:'reject',kind:'mcp',input:{server_id:'fixture',tool:'write',arguments:{}},status:'pending'});
  await page.getByRole('button',{name:'拒绝此请求',exact:true}).click();
  assert.equal(await page.evaluate(()=>window.fixture.resolutions.at(-1).status),'rejected');
  assert.deepEqual(await page.evaluate(()=>window.fixture.dispatches),['mcp-first'],'rejection must not dispatch');
  await page.evaluate(()=>window.fixture.operation.value={id:'mcp-unknown',kind:'mcp',input:{server_id:'fixture',tool:'write',arguments:{}},status:'pending'});
  await reviewAndApprove();await execute.click();await page.getByText('外部结果尚未确认。',{exact:false}).waitFor();
  assert.equal(await execute.count(),0);assert.equal(await accept.count(),0);
  assert.equal(await page.getByRole('button',{name:'拒绝此请求',exact:true}).count(),0);
  const evidence=page.getByLabel('对账依据',{exact:true}),ended=page.getByRole('checkbox',{name:'已核实原执行者和外部任务均已终止，不存在迟到副作用',exact:true});
  const reconcile=page.getByRole('button',{name:'确认并记录对账结果',exact:true});
  await evidence.fill('外部系统已核实');assert.equal(await reconcile.isDisabled(),true,'evidence alone is insufficient');
  await ended.check();await page.locator('.operation-review select').selectOption('not_applied');await reconcile.click();
  assert.deepEqual(await page.evaluate(()=>window.fixture.resolutions.at(-1)),{id:'mcp-unknown',status:'reconciled_not_applied',result:'外部系统已核实',review_revision:'receipt:mcp-unknown'});
  for(const [id,expected] of [['mcp-timeout','MCP_TIMEOUT_OR_CLOSED'],['mcp-large','AGENT_OPERATION_OUTPUT_WITHHELD']]){
   await page.evaluate(id=>window.fixture.operation.value={id,kind:'mcp',input:{server_id:'fixture',tool:'write',arguments:{}},status:'pending'},id);
   const count=await page.evaluate(()=>window.fixture.resolutions.length);
   await reviewAndApprove();await execute.click();
   if(id==='mcp-timeout'){
    await page.getByText('外部结果尚未确认。',{exact:false}).waitFor();assert.equal(await accept.count(),0);assert.equal(await execute.count(),0);
    assert.equal(await page.evaluate(()=>window.fixture.resolutions.length),count,'timeout cannot auto-record success');
    await evidence.fill('fixture confirmed not applied');await ended.check();await page.locator('.operation-review select').selectOption('not_applied');await reconcile.click();
   }else{
    await accept.waitFor();assert.equal(await page.getByText(expected,{exact:true}).count(),1);assert.equal(await page.getByLabel('待记录的操作结果',{exact:true}).count(),0);await accept.click();
    assert.deepEqual(await page.evaluate(()=>window.fixture.resolutions.at(-1)),{id,status:'accepted',result:''});
   }
   assert.equal(await page.evaluate(()=>window.fixture.receipts.at(-1).result),expected);
  }
  await page.evaluate(()=>window.fixture.operation.value={id:'mcp-legacy',kind:'mcp',input:{server_id:'fixture',tool:'write',arguments:{}},status:'pending'});
  await reviewAndApprove();await execute.click();await page.getByText('原始审批或工作区证据不足',{exact:false}).waitFor();
  await evidence.fill('fixture evidence');await ended.check();assert.equal(await reconcile.isDisabled(),true,'legacy unknown without native proof cannot be settled');
  await page.evaluate(()=>window.fixture.operation.value={id:'recovered',kind:'mcp',input:{server_id:'fixture',tool:'write',arguments:{}},status:'executing'});
  await page.getByText('外部结果尚未确认。',{exact:false}).waitFor();assert.equal(await execute.count(),0);
  await page.getByRole('button',{name:'暂时记录为结果未知',exact:true}).click();
  await evidence.fill('fixture evidence');await ended.check();assert.equal(await reconcile.isDisabled(),true);
  await page.evaluate(()=>window.fixture.operation.value={id:'stale-evidence',kind:'mcp',input:{server_id:'fixture',tool:'write',arguments:{}},status:'unknown',review_revision:'old',can_reconcile:true});
  await evidence.fill('old evidence');await ended.check();assert.equal(await reconcile.isDisabled(),false);
  await page.evaluate(()=>window.fixture.operation.value={...window.fixture.operation.value,review_revision:'new'});
  assert.equal(await evidence.inputValue(),'');assert.equal(await ended.isChecked(),false);assert.equal(await reconcile.isDisabled(),true);
  await page.evaluate(()=>window.fixture.operation.value={id:'command-first',kind:'command',input:{snippet_id:'fixture-command',values:{}},status:'pending'});
  await page.getByRole('checkbox',{name:'我已检查命令和参数，批准本次执行',exact:true}).check();
  await page.getByRole('button',{name:'执行已审核命令',exact:true}).click();await accept.waitFor();
  assert.deepEqual(await page.evaluate(()=>window.fixture.calls.filter(x=>x.action==='execute_operation').at(-1)),
   {action:'execute_operation',operation_id:'command-first',values:{},expected_revision:'a'.repeat(64)});
  await accept.click();
  await page.evaluate(()=>window.fixture.operation.value={id:'command-stale',kind:'command',input:{snippet_id:'fixture-command',values:{}},status:'pending'});
  await page.getByRole('checkbox',{name:'我已检查命令和参数，批准本次执行',exact:true}).waitFor();
  await page.evaluate(()=>window.fixture.rotate());
  await page.getByRole('checkbox',{name:'我已检查命令和参数，批准本次执行',exact:true}).check();
  await page.getByRole('button',{name:'执行已审核命令',exact:true}).click();
  await page.getByText('已确认未派发',{exact:false}).waitFor();
  assert.equal(await page.evaluate(()=>window.fixture.dispatches.includes('command-stale')),false,'stale native revision must not dispatch');
  assert.equal(await page.getByRole('button',{name:'执行已审核命令',exact:true}).count(),0);
  assert.equal(await accept.count(),0);
  assert.equal(await page.getByRole('button',{name:'将此输出加入模型上下文',exact:true}).count(),0);
  assert.equal(await page.evaluate(()=>window.fixture.receipts.at(-1).execution.phase),'not_dispatched');
  await page.evaluate(()=>window.fixture.operation.value={id:'command-unknown',kind:'command',input:{snippet_id:'fixture-command',values:{}},status:'pending'});
  await page.getByRole('checkbox',{name:'我已检查命令和参数，批准本次执行',exact:true}).check();
  await page.getByRole('button',{name:'执行已审核命令',exact:true}).click();await page.getByText('外部结果尚未确认。',{exact:false}).waitFor();
  assert.equal(await accept.count(),0);assert.equal(await page.getByRole('button',{name:'执行已审核命令',exact:true}).count(),0);
  assert.deepEqual(await page.evaluate(()=>window.fixture.claims),[]);
  await page.evaluate(()=>window.fixture.operation.value={id:'owner-fails',kind:'mcp',input:{server_id:'fixture',tool:'write',arguments:{}},status:'pending'});
  await reviewAndApprove();await execute.click();
  await page.getByRole('status').filter({hasText:'原生执行结果未确认'}).waitFor();
  assert.equal(await execute.isDisabled(),true,'failed native owner admission cannot be retried');
  assert.ok(!(await page.locator('body').innerText()).includes('private-token-value'));
  await page.evaluate(()=>window.fixture.operation.value={id:'diagnostics-claim-fails',kind:'code_diagnostics',input:{path:'src/main.rs'},status:'pending'});
  await page.getByRole('checkbox',{name:'我批准这一次工作区操作',exact:true}).check();
  await page.getByRole('button',{name:'执行本次代码诊断',exact:true}).click();
  await page.getByRole('status').filter({hasText:'请求未发出'}).waitFor();
  assert.deepEqual(await page.evaluate(()=>window.fixture.claims),['diagnostics-claim-fails']);
  assert.deepEqual(await page.evaluate(()=>window.fixture.directCommands),[]);
  assert.equal(await page.evaluate(()=>window.fixture.calls.filter(x=>x.action==='mcp_call').length),0);
  const ids=await page.evaluate(()=>window.fixture.calls.filter(x=>x.action==='execute_operation').map(x=>x.operation_id));assert.equal(new Set(ids).size,ids.length);
  assert.deepEqual(errors,[]);
  console.log('CODING_OPERATION_UI=PASS native-owner-receipts/approval-scope/unknown-effect-reconciliation/one-shot-execution; simulated native owner');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
