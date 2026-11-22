// Real Vue UI with simulated navigation and native transport. No Bilibili calls.
const assert=require('node:assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const base=process.env.CODING_UI_URL||'http://127.0.0.1:1433';
(async()=>{
 const [componentSource,storeSource]=await Promise.all([
   fetch(base+'/src/capabilities/home/ui/HomePanel.vue').then(response=>response.text()),
   fetch(base+'/src/stores/useLayoutStore.ts').then(response=>response.text()),
 ]);
 const vue=componentSource.match(/from ["']([^"']+\/vue\.js\?[^"']+)["']/)?.[1];
 const pinia=storeSource.match(/from ["']([^"']+\/pinia\.js\?[^"']+)["']/)?.[1];
 assert.ok(vue,'use the same optimized Vue module as the real panels');
 assert.ok(pinia,'use the same optimized Pinia module as the real owners');
 const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox']});
 try{
  const page=await browser.newPage(),errors=[];
  page.on('pageerror',e=>errors.push(e.message));
  let workingDraft=null;
  let bfcScenario='logged-out',ticketIndex=0,activeTicket='',statusReads=0,finishAttempts=0,finalState='';
  const bfcCalls=[];
  const importResult={scan_id:'fixture-scan',pages:1,cards:2,unique:2,gap:0};
  const readbackStatus=(state)=>({
    state,login_status:['logged_out','access_limited'].includes(state)?state:'logged_in',
    mid:['logged_out','access_limited'].includes(state)?null:'1234',
    folder_id:'fixture-folder',folder_name:'Fixture favorites',
    pages_captured:['ready','imported','outcome_unknown'].includes(state)?1:0,
    expected_pages:1,reported_total:2,
    items_captured:['ready','imported','outcome_unknown'].includes(state)?2:0,
    message_code:state==='access_limited'?'BFC_BILIBILI_HTTP_412':null,
    imported:state==='imported'?importResult:null,
  });
  await page.exposeFunction('homeWorkingDraftNative',(method,draft)=>{
    if(method==='load')return structuredClone(workingDraft);
    if(method==='save'){
      assert.equal(Object.hasOwn(draft,'app_secret'),false);
      workingDraft={...structuredClone(draft),updated_at_ms:Date.now()};
      return workingDraft.updated_at_ms;
    }
    if(method==='clear'){workingDraft=null;return;}
    throw Error('unexpected working-draft fixture operation');
  });
  await page.exposeFunction('bfcFixtureNative',(method,...args)=>{
    bfcCalls.push([method,args[0]]);
    if(method==='service')return {
      online:true,local_only:true,bilibili_network_requests:0,
      read_snapshot_only:true,complete_snapshot:false,message_code:null,
    };
    if(method==='begin'){
      assert.equal(args[0],'tab-1','readback must bind the selected official tab');
      activeTicket='fixture-ticket-'+(++ticketIndex);
      statusReads=0;finishAttempts=0;finalState='';
      return {ticket:activeTicket,scan_id:'fixture-scan',expires_in_seconds:300};
    }
    if(method==='eval'){
      assert.equal(args[0],'tab-1');
      assert.equal(typeof args[1],'string');
      return; // The generated probe is sent across simulated IPC, never evaluated on Bilibili.
    }
    assert.equal(args[0],activeTicket,'all readback actions must retain their native ticket');
    if(method==='status'){
      statusReads+=1;
      if(finalState)return readbackStatus(finalState);
      if(bfcScenario==='logged-out')return readbackStatus('logged_out');
      if(bfcScenario==='limited')return readbackStatus('access_limited');
      return readbackStatus(statusReads===1?'logged_in':'ready');
    }
    if(method==='finish'){
      finishAttempts+=1;
      if(bfcScenario==='unknown'){
        finalState='outcome_unknown';
        throw Error('BFC_IMPORT_OUTCOME_UNKNOWN');
      }
      assert.equal(bfcScenario,'retry','login/HTTP 412 failures must never reach local import');
      if(finishAttempts===1)throw Error('BFC_LOCAL_SERVICE_UNAVAILABLE');
      finalState='imported';
      return importResult;
    }
    if(method==='cancel'){finalState='failed';return;}
    throw Error('unexpected BFC fixture operation: '+method);
  });
  const html=`<link rel="stylesheet" href="/src/styles/tokens.css"><div id="app"></div><script type="module">
    import {createApp} from '${vue}';
    import {createPinia} from '${pinia}';
    import Home from '/src/capabilities/home/ui/HomePanel.vue';
    import WechatWorkbenchPanel from '/src/components/system/WechatWorkbenchPanel.vue';
    import {registerWechatContributions} from '/src/capabilities/wechat/index.ts';
    import Bfc from '/src/capabilities/bfc/ui/BfcProjectPanel.vue';
    import {useBrowserStore} from '/src/capabilities/browser/public.ts';
    import {useLayoutStore} from '/src/stores/useLayoutStore.ts';
    import {bridge} from '/src/bridge.ts';
    const pinia=createPinia();
    window.calls=[];
    bridge.getStartDirs=async()=>[];
    bridge.wechatAccountStatus=async()=>{throw Error('WECHAT_ACCOUNT_NOT_CONFIGURED')};
    bridge.wechatDraftPermissionCheck=async()=>({draft_read_access:true,draft_count:7});
    bridge.wechatWorkingDraftLoad=()=>window.homeWorkingDraftNative('load');
    bridge.wechatWorkingDraftSave=draft=>window.homeWorkingDraftNative('save',draft);
    bridge.wechatWorkingDraftClear=()=>window.homeWorkingDraftNative('clear');
    bridge.bfcServiceStatus=()=>window.bfcFixtureNative('service');
    bridge.bfcReadbackBegin=tabId=>window.bfcFixtureNative('begin',tabId);
    bridge.bfcReadbackStatus=ticket=>window.bfcFixtureNative('status',ticket);
    bridge.bfcReadbackFinish=ticket=>window.bfcFixtureNative('finish',ticket);
    bridge.bfcReadbackCancel=ticket=>window.bfcFixtureNative('cancel',ticket);
    bridge.evalInTab=(tabId,script)=>window.bfcFixtureNative('eval',tabId,script);
    bridge.debugLog=async()=>{};
    const browserStore=useBrowserStore(pinia),layout=useLayoutStore(pinia);
    registerWechatContributions();
    window.layout=layout;
    browserStore.tabs.push({id:'tab-1',url:'https://space.bilibili.com/1234/favlist',title:'Fixture favorites',busy:false});
    browserStore.activeTabId='tab-1';
    browserStore.tabNew=async(url)=>{window.calls.push(['open',url]);return true};
    browserStore.tabSwitch=async(id)=>{window.calls.push(['switch',id]);browserStore.activeTabId=id;return true};
    const activateView=layout.activateView;
    layout.activateView=(view)=>{window.calls.push(['view',view]);return activateView(view)};
    window.mount=(component)=>{window.instance?.unmount();window.instance=createApp(component).use(pinia);window.instance.mount('#app')};
    window.mount(Home);
    window.mountBfc=()=>window.mount(Bfc);
    window.mountWechat=()=>window.mount(WechatWorkbenchPanel);
  </script>`;
  await page.route('**/__home_bfc_fixture',route=>route.fulfill({contentType:'text/html',body:html}));
  await page.goto(base+'/__home_bfc_fixture');
  const draft=page.getByRole('button',{name:'📣 公众号工作台'});
  await draft.waitFor();
  await draft.click();
  assert.equal(await page.evaluate(()=>window.layout.mainView),'wechat','home action opens the WeChat module view');
  assert.equal(await page.evaluate(()=>window.layout.modTabs.length),1,'a first-class module tab is created');
  assert.equal(await page.evaluate(()=>window.layout.modTabs[0].label),'公众号工作台');
  const wechatTabId=await page.evaluate(()=>window.layout.activeModTab);
  await draft.click();
  assert.equal(await page.evaluate(()=>window.layout.modTabs.length),1,'repeated clicks reuse the same module tab');
  assert.equal(await page.evaluate(()=>window.layout.activeModTab),wechatTabId);
  await page.evaluate(()=>window.mountWechat());
  await page.getByRole('heading',{name:'公众号个人工作台'}).waitFor({timeout:15000});
  const nav=page.getByRole('navigation',{name:'公众号工作台导航'});
  assert.equal(await nav.getByRole('button').count(),9,'all nine reference screenshot sections are navigable');
  const shell=page.locator('.wxwb');
  assert.match(await shell.evaluate(el=>getComputedStyle(el).backgroundImage),/gradient/);
  await page.getByRole('button',{name:'验证草稿读取权限'}).first().click();
  await page.getByText('草稿读取权限已验证；写入仍须通过真实草稿核验。').waitFor();
  await nav.getByRole('button',{name:'AI 选题库'}).click();
  await page.getByRole('textbox',{name:'主题 / 标题'}).fill('AI 工具如何提升生产效率？');
  await page.getByRole('textbox',{name:'选题依据 / 论证角度'}).fill('已核实的产品功能、同行比较与反例');
  await page.getByRole('button',{name:/加入待写选题/}).click();
  await page.getByRole('button',{name:/带入草稿/}).click();
  assert.equal(await page.getByRole('textbox',{name:'标题'}).inputValue(),'AI 工具如何提升生产效率？');
  await page.getByRole('button',{name:'立即保存当前草稿'}).click();
  await page.getByText('当前工作草稿已保存到系统密钥库。').waitFor();
  assert.equal(workingDraft.title,'AI 工具如何提升生产效率？','topic-to-draft persists through the native working-draft owner');
  assert.equal(await page.getByRole('alert').filter({hasText:'系统密钥库不可用'}).count(),0);
  await nav.getByRole('button',{name:'发布排期'}).click();
  await page.getByRole('textbox',{name:'文章标题'}).fill('周五内容复核');
  await page.locator('input[type="datetime-local"]').fill('2026-11-07T20:00');
  await page.getByRole('button',{name:/添加本机计划/}).click();
  await page.getByText('周五内容复核').waitFor();
  await page.getByText(/未发送/).first().waitFor();
  await nav.getByRole('button',{name:'数据复盘'}).click();
  await page.getByText(/官方阅读量、完读率、粉丝和分享接口尚未接入/).waitFor();
  await nav.getByRole('button',{name:'爆文拆解'}).click();
  await page.getByPlaceholder('粘贴标题，分析数字、问题与冲突等结构…').fill('AI 如何让效率提升 3 倍？');
  await page.getByText('数字标题').waitFor();
  await nav.getByRole('button',{name:'工作台总览'}).click();
  await page.getByText('来自微信草稿读取 API').waitFor();
  await page.evaluate(()=>document.documentElement.dataset.theme='dark');
  const homeBackground=await page.locator('.wxwb').evaluate(el=>getComputedStyle(el).backgroundImage);
  assert.match(homeBackground,/gradient/);
  await page.setViewportSize({width:700,height:900});
  const mobile=await page.locator('.wxwb').evaluate(el=>({w:el.getBoundingClientRect().width,available:el.parentElement.getBoundingClientRect().width}));
  assert.ok(mobile.w<=mobile.available+2,'WeChat dashboard must fit a narrow BrowserOS window');
  await page.evaluate(()=>window.mountBfc());
  await page.getByRole('heading',{name:'个人内容库'}).waitFor();
  await page.getByText('本机服务在线',{exact:true}).waitFor();
  assert.equal(bfcCalls.filter(call=>call[0]==='begin').length,0,'mounting or checking the local service must not read an account');
  await page.getByText(/当前 BFC 服务尚未声明完整快照能力/).waitFor();
  await page.getByRole('button',{name:/本地 BFC 工作台/}).click();
  assert.deepEqual(await page.evaluate(()=>window.calls.slice(-2)),[
    ['view','browser'],['open','http://127.0.0.1:4198/#updates'],
  ],'the local workbench uses the current view-navigation contract');
  await page.getByRole('button',{name:'查看本地新增候选',exact:true}).click();
  await page.getByRole('button',{name:'打开安全补采',exact:true}).click();
  assert.equal(bfcCalls.filter(call=>call[0]==='begin').length,0,'local navigation cannot claim or begin account synchronization');

  const startReadback=page.getByRole('button',{name:'开始读取当前账号收藏'});
  const result=page.locator('.bfc-result');
  await startReadback.click();
  await result.getByText(/当前页面没有检测到已登录账号/).waitFor();
  await result.getByText('需要登录',{exact:true}).waitFor();
  assert.equal(bfcCalls.filter(call=>call[0]==='finish').length,0,'logged-out readback must not import a snapshot');

  bfcScenario='limited';
  await startReadback.click();
  await result.getByText(/检测到 B站 HTTP 412/).waitFor();
  await result.getByText('平台访问受限',{exact:true}).waitFor();
  await result.getByText('BFC_BILIBILI_HTTP_412',{exact:true}).waitFor();
  const limitedCalls=bfcCalls.length;
  // One polling interval must elapse without another read or an automatic retry.
  await page.waitForTimeout(850);
  assert.equal(bfcCalls.length,limitedCalls,'HTTP 412 must stop polling and must not retry');
  assert.equal(bfcCalls.filter(call=>call[0]==='finish').length,0,'HTTP 412 must not reach local import');
  assert.equal(await page.locator('.bfc-import-result').count(),0,'blocked access cannot be presented as imported');
  await page.getByText(/不要轮换代理、导入他人 Cookie、注入 Cookie 或绕过验证码/).waitFor();

  bfcScenario='retry';
  const beginsBeforeRetry=bfcCalls.filter(call=>call[0]==='begin').length;
  await startReadback.click();
  const retrySave=page.getByRole('button',{name:'重试保存到本地 BFC'});
  await retrySave.waitFor();
  await result.getByText(/读取结果仍保留在内存中/).waitFor();
  assert.equal(finishAttempts,1,'a known failed local write must await an explicit retry');
  const preservedTicket=activeTicket;
  await page.waitForTimeout(850);
  assert.equal(finishAttempts,1,'the owner must not repeat the failed local write automatically');
  await retrySave.click();
  await result.getByText(/BFC 已接收 2 条收藏/).waitFor();
  await result.getByText(/不代表完整同步/).waitFor();
  assert.equal(finishAttempts,2,'one explicit click retries once');
  assert.equal(activeTicket,preservedTicket,'retry preserves the original native readback ticket');
  assert.equal(bfcCalls.filter(call=>call[0]==='begin').length,beginsBeforeRetry+1,'retry saves the retained snapshot without rescanning');
  assert.equal(await page.locator('.bfc-import-result').count(),1);

  bfcScenario='unknown';
  await startReadback.click();
  await result.getByText(/本地 BFC 写入结果不确定/).waitFor();
  await result.getByText('保存结果不确定',{exact:true}).waitFor();
  assert.equal(await retrySave.count(),0,'an uncertain local write must not expose the known-failure retry action');
  const unknownCalls=bfcCalls.length;
  await page.waitForTimeout(850);
  assert.equal(bfcCalls.length,unknownCalls,'an uncertain write must not automatically rescan or import again');
  assert.equal(finishAttempts,1);
  assert.equal(await page.locator('.bfc-import-result').count(),0,'unknown results must not show an imported snapshot');
  assert.deepEqual(await page.evaluate(()=>window.calls.filter(call=>call[0]==='open').map(call=>call[1])),[
    'http://127.0.0.1:4198/#updates',
    'http://127.0.0.1:4198/#updates',
    'http://127.0.0.1:4198/manage#sync',
  ]);
  assert.deepEqual(errors,[]);
  console.log('HOME_BFC_MANAGED_UI=PASS nine-section WeChat dashboard, native working-draft transport, plans, responsive contrast; BFC selected-tab readback, login/HTTP 412 refusal, retained-snapshot explicit retry and unknown-write stop; simulated IPC, no real account acceptance');
 }finally{await browser.close()}
})().catch(error=>{console.error(error);process.exitCode=1});
