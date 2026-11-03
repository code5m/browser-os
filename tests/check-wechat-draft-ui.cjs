// Real Vue browser interaction with an intentionally simulated IPC boundary.
// This test NEVER contacts WeChat; it cannot establish real account acceptance.
const assert = require('node:assert/strict');
const { chromium } = require('playwright');
(async () => {
  const browser = await chromium.launch({ executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox']});
  try {
    const page = await browser.newPage();
    const errors=[];
    page.on('pageerror', error => errors.push(error.message));
    const html=`<div id="app"></div><script type="module">
       import {createApp} from '/node_modules/.vite/deps/vue.js';
       import Panel from '/src/components/system/WechatDraftPanel.vue';
       import {bridge} from '/src/bridge.ts';
       window.calls=[];
       bridge.wechatAccountStatus=async()=>{throw Error('WECHAT_ACCOUNT_NOT_CONFIGURED')};
       bridge.wechatAccountSave=async(id,secret)=>{window.calls.push(['config',id,secret.length]);return '***cdef';};
       bridge.wechatDraftPermissionCheck=async()=>({draft_count:2,draft_read_access:true});
       bridge.wechatUploadBodyImage=async(path)=>{window.calls.push(['body',path]);return {url:'https://mmbiz.qpic.cn/mock.png'}};
       bridge.wechatUploadCover=async(path)=>{window.calls.push(['cover',path]);return {media_id:'cover-fixture-id'}};
       bridge.wechatCreateDraft=async(...fields)=>{window.calls.push(['draft',fields]);throw Error('WECHAT_DRAFT_OUTCOME_UNKNOWN_NO_AUTO_RETRY')};
       bridge.wechatGetDraftStatus=async id=>({media_id:id,verified:true});
       createApp(Panel).mount('#app');</script>`;
    await page.route('**/__wechat_draft_fixture', route => route.fulfill({contentType:'text/html',body:html}));
    await page.goto('http://127.0.0.1:1433/__wechat_draft_fixture');
    await page.getByPlaceholder('wx...').fill('wx1234567890abcdef');
    await page.getByPlaceholder('仅写入系统密钥库').fill('a-test-app-secret');
    await page.getByRole('button',{name:'保存账户'}).click();
    await page.getByText('配置已存入系统密钥库，未保存到浏览器存储。').waitFor();
    assert.equal(await page.getByPlaceholder('仅写入系统密钥库').inputValue(),'', 'UI must clear secret after save');
    await page.getByRole('button',{name:'验证草稿读取权限'}).click();
    await page.getByText(/官方草稿读取权限检查通过/).waitFor();
    await page.getByPlaceholder('/home/.../cover.jpg').fill('/tmp/cover.jpg');
    await page.getByRole('button',{name:'上传永久封面'}).click();
    await page.getByText(/封面素材上传成功/).waitFor();
    await page.getByPlaceholder('/home/.../body.png').fill('/tmp/body.png');
    await page.getByRole('button',{name:'上传正文图片'}).click();
    await page.getByText(/正文图片已上传/).waitFor();
    await page.getByRole('button',{name:'插入正文'}).click();
    await page.getByRole('textbox',{name:'标题'}).fill('草稿验收，不发布');
    const draftButton=page.getByRole('button',{name:'创建到草稿箱（不发布）'});
    await draftButton.click();
    await page.getByText(/操作失败：WECHAT_DRAFT_OUTCOME_UNKNOWN_NO_AUTO_RETRY/).waitFor();
    assert.equal(await draftButton.isDisabled(),true, 'unknown outcome must prevent duplicate draft request');
    const calls=await page.evaluate(()=>window.calls);
    assert.equal(calls.filter(call=>call[0]==='draft').length,1);
    assert.deepEqual(errors,[]);
    console.log('WECHAT_DRAFT_UI_PASS=mock transport, no publish endpoints, unknown-outcome retry gate');
  } finally { await browser.close(); }
})().catch(error=>{console.error(error);process.exitCode=1});
