// Real Vue browser interaction with an intentionally simulated IPC boundary.
// The parent-process owner survives page reload; no real keyring or WeChat account is used.
const assert = require('node:assert/strict');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const base = process.env.CODING_UI_URL || 'http://127.0.0.1:1433';
(async () => {
  const source=await(await fetch(base+'/src/components/system/WechatDraftPanel.vue')).text();
  const vue=source.match(/from ["']([^"']+\/vue\.js\?[^"']+)["']/)?.[1];
  assert.ok(vue,'use the same optimized Vue module as the real panel');
  const browser = await chromium.launch({ executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox']});
  try {
    const page = await browser.newPage();
    const errors = [];
    const calls = [];
    const ownerCalls = [];
    const owner = { draft:null, failLoad:false, failSave:false, failClear:false };
    let savedAt = Date.now();
    let draftMode = 'unknown';
    let failPermission = false;
    const clone = value => structuredClone(value);
    const draftContent = ({updated_at_ms,...draft}) => draft;
    const draftCalls = () => calls.filter(call => call[0] === 'draft');
    const draftKeys = ['version','title','author','digest','content_html','cover_path',
      'body_image_path','cover_media_id','body_image_url','draft_media_id','verified',
      'outcome_unknown','updated_at_ms'].sort();
    page.on('pageerror', error => errors.push(error.message));
    await page.exposeFunction('wechatFixtureNative', (method, value) => {
      if (method === 'load') {
        ownerCalls.push('load');
        if (owner.failLoad) throw Error('WECHAT_DRAFT_STORE_UNAVAILABLE');
        return clone(owner.draft);
      }
      if (method === 'save') {
        ownerCalls.push('save');
        assert.deepEqual(Object.keys(value).sort(), draftKeys, 'working-draft DTO must exclude account credentials');
        assert.equal(value.version, 1);
        assert.equal(typeof value.outcome_unknown, 'boolean');
        assert.equal(typeof value.verified, 'boolean');
        assert.equal(JSON.stringify(value).includes('test-app-secret'), false);
        assert.equal(JSON.stringify(value).includes('another-test-secret'), false);
        if (owner.failSave) throw Error('WECHAT_DRAFT_STORE_UNAVAILABLE');
        owner.draft = { ...clone(value), updated_at_ms:++savedAt };
        return savedAt;
      }
      if (method === 'clear') {
        ownerCalls.push('clear');
        if (owner.failClear) throw Error('WECHAT_DRAFT_STORE_UNAVAILABLE');
        owner.draft = null;
        return;
      }
      if (method === 'account') {
        calls.push(['config',value[0],value[1].length]);
        return '***cdef';
      }
      if (method === 'permission') {
        if (failPermission) throw Error('WECHAT_API_-1 provider details must not appear');
        return {draft_count:2,draft_read_access:true};
      }
      if (method === 'body') {
        calls.push(['body',value]);
        return {url:'https://mmbiz.qpic.cn/mock.png'};
      }
      if (method === 'cover') {
        calls.push(['cover',value]);
        return {media_id:'cover-fixture-id'};
      }
      if (method === 'create') {
        // This assertion checks the ordering across the actual component and native seam.
        assert.equal(owner.draft?.outcome_unknown, true, 'the no-retry latch must be durable before draft/add');
        assert.equal(owner.draft.draft_media_id, '', 'a prior draft cannot authenticate a new request');
        assert.deepEqual(value, [owner.draft.title,owner.draft.author,owner.draft.digest,
          owner.draft.content_html,owner.draft.cover_media_id]);
        calls.push(['draft',clone(value)]);
        if (draftMode === 'created') return {media_id:'draft-fixture-id',verified:false,status:'created'};
        if (draftMode === 'invalid') throw Error('WECHAT_DRAFT_INPUT_INVALID');
        throw Error('WECHAT_DRAFT_OUTCOME_UNKNOWN_NO_AUTO_RETRY');
      }
      if (method === 'verify') {
        calls.push(['verify',value]);
        return {media_id:value,verified:true,status:'verified'};
      }
      throw Error('unexpected fixture native operation: ' + method);
    });
    const html=`<div id="app"></div><script type="module">
       import {createApp} from '${vue}';
       import Panel from '/src/components/system/WechatDraftPanel.vue';
       import {bridge} from '/src/bridge.ts';
       bridge.wechatWorkingDraftLoad=()=>window.wechatFixtureNative('load');
       bridge.wechatWorkingDraftSave=draft=>window.wechatFixtureNative('save',draft);
       bridge.wechatWorkingDraftClear=()=>window.wechatFixtureNative('clear');
       bridge.wechatAccountStatus=async()=>{throw Error('WECHAT_ACCOUNT_NOT_CONFIGURED')};
       bridge.wechatAccountSave=(id,secret)=>window.wechatFixtureNative('account',[id,secret]);
       bridge.wechatDraftPermissionCheck=()=>window.wechatFixtureNative('permission');
       bridge.wechatUploadBodyImage=path=>window.wechatFixtureNative('body',path);
       bridge.wechatUploadCover=path=>window.wechatFixtureNative('cover',path);
       bridge.wechatCreateDraft=(...fields)=>window.wechatFixtureNative('create',fields);
       bridge.wechatGetDraftStatus=id=>window.wechatFixtureNative('verify',id);
       createApp(Panel).mount('#app');</script>`;
    await page.route('**/__wechat_draft_fixture', route => route.fulfill({contentType:'text/html',body:html}));
    await page.goto(base + '/__wechat_draft_fixture');
    await page.locator('.storage-status').filter({hasText:'尚无本地工作草稿'}).waitFor();
    assert.equal(await page.getByRole('button',{name:'选择封面文件'}).count(),1,'native cover file chooser is exposed');
    assert.equal(await page.getByRole('button',{name:'选择正文图片'}).count(),1,'native body image picker is exposed');
    await page.getByPlaceholder('wx...').fill('wx1234567890abcdef');
    await page.getByPlaceholder('仅写入系统密钥库').fill('a-test-app-secret');
    await page.getByRole('button',{name:'保存账户'}).click();
    await page.getByText('账户配置已存入系统密钥库；AppSecret 未写入工作草稿。').waitFor();
    assert.equal(await page.getByPlaceholder('仅写入系统密钥库').inputValue(),'', 'UI must clear secret after save');
    await page.getByRole('button',{name:'验证草稿读取权限'}).click();
    await page.getByText(/官方草稿读取权限检查通过/).waitFor();
    failPermission = true;
    await page.getByRole('button',{name:'验证草稿读取权限'}).click();
    await page.getByText('操作失败：WECHAT_API_-1',{exact:true}).waitFor({timeout:3000});
    assert.equal(await page.getByText(/provider details must not appear/).count(),0,'provider details must stay out of the UI');
    await page.getByPlaceholder('/home/.../cover.jpg').fill('/tmp/cover.jpg');
    await page.getByRole('button',{name:'上传永久封面'}).click();
    await page.getByText(/封面素材上传成功/).waitFor();
    await page.getByPlaceholder('/home/.../cover.jpg').fill('/tmp/other-cover.jpg');
    await page.getByText('封面 media_id：未上传').waitFor({timeout:3000});
    await page.getByPlaceholder('/home/.../cover.jpg').fill('/tmp/cover.jpg');
    await page.getByRole('button',{name:'上传永久封面'}).click();
    await page.getByText(/封面素材上传成功/).waitFor();
    await page.getByPlaceholder('/home/.../body.png').fill('/tmp/body.png');
    await page.getByRole('button',{name:'上传正文图片'}).click();
    await page.getByText(/正文图片已上传/).waitFor();
    await page.getByPlaceholder('仅写入系统密钥库').fill('another-test-secret');
    await page.getByRole('button',{name:'保存账户'}).click();
    await page.getByText('账户配置已存入系统密钥库；AppSecret 未写入工作草稿。').waitFor();
    await page.getByText('封面 media_id：未上传').waitFor();
    assert.equal(await page.getByRole('button',{name:'插入正文'}).isDisabled(),true,'account change must invalidate body image');
    await page.getByRole('button',{name:'上传永久封面'}).click();
    await page.getByText(/封面素材上传成功/).waitFor();
    await page.getByRole('button',{name:'上传正文图片'}).click();
    await page.getByText(/正文图片已上传/).waitFor();
    await page.getByRole('button',{name:'插入正文'}).click();
    await page.getByRole('textbox',{name:'标题'}).fill('草稿验收，不发布');
    const draftButton=page.getByRole('button',{name:'创建到草稿箱（不发布）'});
    await draftButton.click();
    await page.getByText(/操作失败：WECHAT_DRAFT_OUTCOME_UNKNOWN_NO_AUTO_RETRY/).waitFor();
    assert.equal(await draftButton.isDisabled(),true, 'unknown outcome must prevent duplicate draft request');
    assert.equal(draftCalls().length,1);
    const unknownDraft = clone(owner.draft);
    await page.reload();
    await page.getByText('上次提交结果不明，请先到公众号后台核实。').waitFor();
    assert.equal(await draftButton.isDisabled(),true, 'unknown draft outcome must survive page reload');
    assert.equal(await page.getByRole('textbox',{name:'标题'}).inputValue(),unknownDraft.title);
    assert.equal(await page.getByRole('textbox',{name:'草稿 HTML 正文'}).inputValue(),unknownDraft.content_html);
    assert.equal(draftCalls().length,1, 'reload must not submit another draft');
    page.once('dialog', dialog=>dialog.dismiss());
    await page.getByRole('button',{name:'人工核实无重复草稿后解除禁止重试'}).click();
    assert.equal(owner.draft.outcome_unknown,true,'declining reconciliation must retain the durable latch');
    page.once('dialog', dialog=>dialog.accept());
    await page.getByRole('button',{name:'人工核实无重复草稿后解除禁止重试'}).click();
    await page.getByText('已记录人工核实；允许重新创建一次草稿。').waitFor();
    assert.equal(owner.draft.outcome_unknown,false);
    await page.reload();
    await page.locator('.storage-status').filter({hasText:'已从系统密钥库恢复未完成草稿'}).waitFor();
    assert.equal(await page.getByText('上次提交结果不明，请先到公众号后台核实。').count(),0, 'explicit reconciliation clears the persisted latch');

    await page.getByRole('textbox',{name:'标题'}).fill('存储故障测试');
    await page.getByRole('textbox',{name:'草稿 HTML 正文'}).fill('<p>fixture</p>');
    await page.getByRole('button',{name:'立即保存当前草稿'}).click();
    await page.getByText('当前工作草稿已保存到系统密钥库。').waitFor();
    const beforeFailedSave = clone(owner.draft);
    owner.failSave = true;
    await draftButton.click();
    await page.getByRole('alert').filter({hasText:'系统密钥库不可用'}).waitFor();
    assert.equal(await draftButton.isDisabled(),true,'native storage failure must block creation');
    assert.equal(draftCalls().length,1,'failed pre-send persistence must prevent the remote draft request');
    assert.deepEqual(owner.draft,beforeFailedSave,'failed save must preserve the last durable draft');
    await page.getByRole('button',{name:'重试恢复密钥库'}).click();
    await page.getByText(/当前页面存在尚未保存的内容/).waitFor();
    assert.equal(await page.getByRole('textbox',{name:'标题'}).inputValue(),'存储故障测试','recovery must not discard unsaved UI contents');
    assert.deepEqual(owner.draft,beforeFailedSave,'recovery must not overwrite an unverified stored draft');
    await page.reload();
    await page.locator('.storage-status').filter({hasText:'已从系统密钥库恢复未完成草稿'}).waitFor();
    owner.failSave = false;
    assert.equal(owner.draft.outcome_unknown,false,'a failed pre-send save never reached draft/add');

    owner.failLoad = true;
    await page.reload();
    await page.getByRole('alert').filter({hasText:'系统密钥库不可用'}).waitFor();
    assert.equal(await draftButton.isDisabled(),true,'failed native recovery must block creation');
    assert.deepEqual(draftContent(owner.draft),draftContent(beforeFailedSave),'failed load must retain the existing native draft');
    owner.failLoad = false;
    await page.getByRole('button',{name:'重试恢复密钥库'}).click();
    await page.getByText('密钥库已重新连接，原工作草稿已恢复。').waitFor();
    assert.equal(await page.getByRole('textbox',{name:'标题'}).inputValue(),beforeFailedSave.title);
    assert.equal(draftCalls().length,1,'explicit storage recovery must not send a remote draft');

    await page.getByRole('textbox',{name:'标题'}).fill('草稿待核验');
    draftMode = 'created';
    await draftButton.click();
    await page.getByText(/微信接口已返回 media_id，但详情核验未完成/).waitFor();
    assert.equal(await draftButton.isDisabled(),true,'unverified created draft must not be submitted again');
    assert.equal(owner.draft.draft_media_id,'draft-fixture-id');
    assert.equal(owner.draft.outcome_unknown,true);
    await page.reload();
    await page.getByText(/草稿 media_id：draft-fixture-id/).waitFor();
    assert.equal(draftCalls().length,2,'reloading a known draft must not resubmit it');
    await page.getByRole('button',{name:'验证草稿详情'}).click();
    await page.getByText(/草稿详情接口确认存在/).waitFor();
    assert.equal(owner.draft.outcome_unknown,false,'verification clears the native retry latch');
    assert.equal(owner.draft.verified,true);
    await page.getByRole('textbox',{name:'标题'}).fill('草稿验证后');
    await page.getByRole('textbox',{name:'草稿 HTML 正文'}).fill('<p>fixture after verification</p>');
    assert.equal(await draftButton.isDisabled(),false,'verified draft permits a subsequent intentional creation');
    draftMode = 'invalid';
    await draftButton.click();
    await page.getByText(/操作失败：WECHAT_DRAFT_INPUT_INVALID/).waitFor();
    assert.equal(await draftButton.isDisabled(),false,'proven pre-send validation failure must permit correction and retry');
    assert.equal(owner.draft.outcome_unknown,false,'proven pre-send failure clears the native retry latch');
    draftMode = 'unknown';
    await draftButton.click();
    await page.getByText(/操作失败：WECHAT_DRAFT_OUTCOME_UNKNOWN_NO_AUTO_RETRY/).waitFor();
    await page.getByText('上次提交结果不明，请先到公众号后台核实。').waitFor({timeout:3000});
    assert.equal(await page.getByRole('button',{name:'验证草稿详情'}).count(),0,'previous draft ID must not clear a new unknown outcome');
    assert.equal(draftCalls().length,4);

    const clearButton = page.getByRole('button',{name:'清除本机工作草稿'});
    page.once('dialog', dialog=>dialog.dismiss());
    await clearButton.click();
    assert.equal(ownerCalls.filter(method=>method==='clear').length,0,'cancelled clear must not call the native owner');
    owner.failClear = true;
    page.once('dialog', dialog=>dialog.accept());
    await clearButton.click();
    await page.getByText('操作失败：WECHAT_DRAFT_STORE_UNAVAILABLE',{exact:true}).waitFor();
    assert.equal(owner.draft.outcome_unknown,true,'failed clear must retain the native unknown-outcome latch');
    assert.equal(await draftButton.isDisabled(),true,'failed clear cannot authorize a retry');
    owner.failClear = false;
    page.once('dialog', dialog=>dialog.accept());
    await clearButton.click();
    await page.getByText('已清除本机工作草稿；微信公众号后台内容不受影响。').waitFor();
    // Wait past the component's 650 ms debounce to detect resurrection by an old queued edit.
    await page.waitForTimeout(800);
    assert.equal(owner.draft,null,'explicit clear drains queued writes and must not resurrect the draft');
    await page.reload();
    await page.locator('.storage-status').filter({hasText:'尚无本地工作草稿'}).waitFor();
    assert.equal(await page.getByRole('textbox',{name:'标题'}).inputValue(),'');
    assert.equal(draftCalls().length,4,'clearing or reloading local recovery state must never resubmit remotely');
    assert.deepEqual(errors,[]);
    console.log('WECHAT_DRAFT_UI_PASS=mock native owner, durable reload, failed-save preservation, explicit recovery, unknown-outcome retry gate; no real keyring or WeChat acceptance');
  } finally { await browser.close(); }
})().catch(error=>{console.error(error);process.exitCode=1});
