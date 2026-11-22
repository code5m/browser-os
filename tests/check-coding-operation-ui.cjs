// Actual approval component; MCP native transport is deliberately simulated.
const assert=require('node:assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const base=process.env.CODING_UI_URL||'http://127.0.0.1:1433';
(async()=>{
 const text=await(await fetch(base+'/src/capabilities/agent/ui/CodingOperationReview.vue')).text();const vue=text.match(/from ["']([^"']+\/vue\.js\?[^"']+)["']/)?.[1];assert.ok(vue);
 const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox']});
 try{
  const page=await browser.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.route('**/__operation_test',r=>r.fulfill({contentType:'text/html; charset=utf-8',body:`<div id="app"></div><script type="module">
  import {createApp,h,ref} from '${vue}';import Review from '/src/capabilities/agent/ui/CodingOperationReview.vue';import {bridge} from '/src/bridge.ts';import {setCapabilityRuntime} from '/src/capability/runtimeSingleton.ts';
  setCapabilityRuntime({get:id=>id==='script'?{state:'ACTIVE',enabled:true}:null});
  let commandRevision='c'.repeat(64);const commandCalls=[];
  bridge.snippetList=async()=>[{id:'fixture-command',name:'Fixture',enabled:true,revision:commandRevision,interpreter:'node',argv:['fixture'],params:[]}];
  bridge.runCommand=async(id,values,expectedRevision)=>{commandCalls.push({id,values,expectedRevision});if(expectedRevision!==commandRevision)throw Error('SNIPPET_REVIEW_CHANGED');if(operation.value.id==='command-unknown')throw Error('transport failed');return {run_id:'fixture',script_id:'fixture-command',status:'succeeded',output_tail:'ok',exit_code:0,truncated:false};};
  const operation=ref({id:'first',kind:'mcp',input:{server_id:'fixture',tool:'test',arguments:{}},status:'pending'}),calls=[],results=[];
  const reviews=[],events=[];
  bridge.codingService=async request=>{
    calls.push(structuredClone(request));events.push(request.action);
    if(request.action==='mcp_review'){
      const reference='ai:1234567890abcdef12345678:11111111-1111-4111-8111-111111111111';
      const review={action:'mcp_call',command:'/usr/bin/fixture',args:['--fixture'],workspace:request.root,cwd:request.root,
        isolation:'user_process',source:'Fixture MCP settings',environment_names:['FIXTURE_MODE'],credential_refs:[reference],
        credentials:[{environment:'FIXTURE_TOKEN',name:'Fixture model',endpoint:'https://fixture.invalid/v1',reference}],
        tool:request.tool,arguments:request.arguments,timeout_seconds:20,revision:String(reviews.length+1).padStart(64,'0')};
      reviews.push(structuredClone(review));return review;
    }
    if(request.action!=='mcp_call')throw Error('unexpected fixture action');
    if(operation.value.id==='timeout')throw Error('MCP_TIMEOUT_OR_CLOSED private-token-value');
    return {result:operation.value.id==='large'?{content:[{type:'text',text:'数据😀'.repeat(12000)}]}:{isError:true,content:[{type:'text',text:'fixture tool error'}]}};
  };
  const claims=[];window.fixture={operation,calls,results,claims,reviews,events,commandCalls,rotateDefinition:()=>commandRevision='d'.repeat(64)};createApp({setup:()=>()=>h(Review,{key:operation.value.id,operation:operation.value,root:'/fixture',claimOperation:async id=>{claims.push(id);events.push('claim:'+id);if(id==='claim-fails')throw Error('state unknown');},onResolve:r=>results.push(r)})}).mount('#app');</script>`}));
  await page.goto(base+'/__operation_test');
  const operationApproval=page.getByRole('checkbox',{name:'我批准这一次工作区操作',exact:true});
  const scopeApproval=page.getByRole('checkbox',{name:'我允许本次进程取得以上凭据和配置环境变量',exact:true});
  const executeMcp=page.getByRole('button',{name:'执行本次 MCP 请求',exact:true});
  const mcpCallCount=()=>page.evaluate(()=>window.fixture.calls.filter(request=>request.action==='mcp_call').length);
  const reviewAndApproveMcp=async()=>{
    await page.getByRole('button',{name:'读取本次执行范围',exact:true}).click();
    await page.locator('.mcp-execution-review').waitFor();
    assert.equal(await operationApproval.isChecked(),false,'reading a scope requires a fresh operation approval');
    assert.equal(await scopeApproval.isChecked(),false,'credentials require separate explicit approval');
    await operationApproval.check();
    assert.equal(await executeMcp.isDisabled(),true,'operation approval cannot grant credential or environment access');
    await scopeApproval.check();
  };
  await operationApproval.check();assert.equal(await executeMcp.isDisabled(),true,'an unreviewed MCP operation cannot execute');
  assert.equal(await mcpCallCount(),0);await reviewAndApproveMcp();
  await executeMcp.click();await page.getByLabel('待记录的操作结果',{exact:true}).waitFor();
  const first=await page.evaluate(()=>({request:window.fixture.calls.find(request=>request.action==='mcp_call'),review:window.fixture.reviews[0],events:window.fixture.events}));
  assert.deepEqual(first.events,['mcp_review','claim:first','mcp_call'],'native review and persisted claim precede external execution');
  assert.deepEqual(first.request,{root:'/fixture',action:'mcp_call',approved:true,server_id:'fixture',tool:'test',arguments:{},
    review_revision:first.review.revision,approved_credential_refs:first.review.credential_refs,approved_environment_names:['FIXTURE_MODE']},
    'the execution request must bind the native revision and exact approved scopes');
  assert.deepEqual(await page.evaluate(()=>window.fixture.claims),['first'],'claim is persisted before executing');
  assert.equal(await page.getByRole('button',{name:'拒绝此请求',exact:true}).isDisabled(),true,'an executed operation must not be recorded as never executed');
  await page.getByRole('button',{name:'记录结果供智能体续跑',exact:true}).click();assert.equal(await page.evaluate(()=>window.fixture.results[0].status),'failed');
  assert.equal(await page.getByRole('button',{name:'执行本次 MCP 请求',exact:true}).isDisabled(),true);assert.equal(await mcpCallCount(),1);
  await page.evaluate(()=>window.fixture.operation.value={...window.fixture.operation.value,id:'second'});await page.getByRole('button',{name:'拒绝此请求',exact:true}).click();
  assert.equal(await page.evaluate(()=>window.fixture.results.at(-1).status),'rejected');assert.equal(await mcpCallCount(),1);
  await page.evaluate(()=>window.fixture.operation.value={...window.fixture.operation.value,id:'timeout'});await reviewAndApproveMcp();await executeMcp.click();
  const status=page.getByRole('status');await status.filter({hasText:'MCP 服务超时或已关闭'}).waitFor();assert.ok(!(await status.innerText()).includes('private-token-value'));
  await page.getByRole('button',{name:'记录结果供智能体续跑',exact:true}).click();assert.deepEqual(await page.evaluate(()=>window.fixture.results.at(-1)),{id:'timeout',status:'failed',result:'MCP_TIMEOUT_OR_CLOSED'});
  await page.evaluate(()=>window.fixture.operation.value={...window.fixture.operation.value,id:'large'});await reviewAndApproveMcp();await executeMcp.click();
  await page.getByLabel('待记录的操作结果').waitFor();await page.getByRole('button',{name:'记录结果供智能体续跑',exact:true}).click();
  const large=await page.evaluate(()=>window.fixture.results.at(-1));
  assert.equal(large.status,'succeeded');assert.ok(new TextEncoder().encode(large.result).length<=16000);assert.match(large.result,/输出已截断/);assert.ok(!large.result.includes('\uFFFD'));
  await page.evaluate(()=>window.fixture.operation.value={id:'command',kind:'command',input:{snippet_id:'fixture-command',values:{}},status:'pending'});
  await page.getByText('我已检查命令和参数', {exact:false}).waitFor();await page.getByRole('checkbox').check();await page.getByRole('button',{name:'执行已审核命令'}).click();
  await page.getByRole('button',{name:'将此输出加入模型上下文'}).waitFor();assert.equal(await page.getByRole('button',{name:'拒绝此请求'}).isDisabled(),true);
  await page.getByRole('button',{name:'将此输出加入模型上下文'}).click();assert.equal(await page.evaluate(()=>window.fixture.results.at(-1).status),'succeeded');
  assert.deepEqual(await page.evaluate(()=>window.fixture.commandCalls[0]),{id:'fixture-command',values:{},expectedRevision:'c'.repeat(64)},
    'proposed commands must carry the exact reviewed native definition revision');
  await page.evaluate(()=>window.fixture.operation.value={id:'command-stale',kind:'command',input:{snippet_id:'fixture-command',values:{}},status:'pending'});
  await page.getByText('我已检查命令和参数',{exact:false}).waitFor();
  await page.evaluate(()=>window.fixture.rotateDefinition());
  await page.getByRole('checkbox',{name:'我已检查命令和参数，批准本次执行',exact:true}).check();
  await page.getByRole('button',{name:'执行已审核命令'}).click();
  await page.getByRole('button',{name:'记录本次未执行',exact:true}).waitFor();
  assert.equal(await page.getByRole('button',{name:'执行已审核命令'}).isDisabled(),true,'a claimed stale request cannot execute again');
  assert.equal(await page.getByRole('button',{name:'将此输出加入模型上下文'}).count(),0);
  assert.equal(await page.getByRole('button',{name:'记录启动失败或状态未知'}).count(),0,'native rejection is distinct from unknown transport state');
  await page.getByRole('button',{name:'记录本次未执行',exact:true}).click();
  assert.deepEqual(await page.evaluate(()=>window.fixture.results.at(-1)),{id:'command-stale',status:'failed',result:'SNIPPET_REVIEW_CHANGED'});
  await page.evaluate(()=>window.fixture.operation.value={id:'command-unknown',kind:'command',input:{snippet_id:'fixture-command',values:{}},status:'pending'});
  await page.getByText('我已检查命令和参数',{exact:false}).waitFor();await page.getByRole('checkbox').check();await page.getByRole('button',{name:'执行已审核命令'}).click();
  await page.getByRole('button',{name:'记录启动失败或状态未知'}).waitFor();assert.equal(await page.getByRole('button',{name:'拒绝此请求'}).isDisabled(),true);assert.equal(await page.getByRole('button',{name:'执行已审核命令'}).isDisabled(),true);
  await page.getByRole('button',{name:'记录启动失败或状态未知'}).click();assert.equal(await page.evaluate(()=>window.fixture.results.at(-1).result),'COMMAND_START_FAILED_OR_UNKNOWN');
  await page.evaluate(()=>window.fixture.operation.value={id:'recovered',kind:'mcp',input:{server_id:'fixture',tool:'test',arguments:{}},status:'executing'});
  await page.getByText('此操作的执行结果未知',{exact:false}).waitFor();assert.equal(await page.getByRole('button',{name:'执行本次 MCP 请求'}).count(),0);
  await page.getByRole('button',{name:'已核对，记录结果未知'}).click();assert.equal(await page.evaluate(()=>window.fixture.results.at(-1).result),'OPERATION_RESULT_UNKNOWN_USER_REVIEW_REQUIRED');
  const callsBeforeFailure=await mcpCallCount();
  await page.evaluate(()=>window.fixture.operation.value={id:'claim-fails',kind:'mcp',input:{server_id:'fixture',tool:'test',arguments:{}},status:'pending'});
  await reviewAndApproveMcp();await executeMcp.click();
  await page.getByRole('status').filter({hasText:'请求未发出'}).waitFor();assert.equal(await mcpCallCount(),callsBeforeFailure,'claim failure must not invoke external tool');
  assert.deepEqual(errors,[]);console.log('CODING_OPERATION_UI=PASS tool-error-not-success/one-shot-execution/reject-without-execution; simulated native transport');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
