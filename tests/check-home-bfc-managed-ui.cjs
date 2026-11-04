// Real Vue UI with simulated navigation and native transport. No Bilibili calls.
const assert=require('node:assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const base=process.env.CODING_UI_URL||'http://127.0.0.1:1433';
(async()=>{
 const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox']});
 try{
  const page=await browser.newPage(),errors=[];
  page.on('pageerror',e=>errors.push(e.message));
  const html=`<link rel="stylesheet" href="/src/styles/tokens.css"><div id="app"></div><script type="module">
    import {createApp} from '/node_modules/.vite/deps/vue.js';
    import {createPinia} from '/node_modules/.vite/deps/pinia.js';
    import Home from '/src/capabilities/home/ui/HomePanel.vue';
    import Bfc from '/src/capabilities/bfc/ui/BfcProjectPanel.vue';
    import {useBrowserStore} from '/src/capabilities/browser/public.ts';
    import {useLayoutStore} from '/src/stores/useLayoutStore.ts';
    import {bridge} from '/src/bridge.ts';
    const pinia=createPinia();
    window.calls=[];
    bridge.getStartDirs=async()=>[];
    bridge.wechatAccountStatus=async()=>{throw Error('WECHAT_ACCOUNT_NOT_CONFIGURED')};
    const browserStore=useBrowserStore(pinia),layout=useLayoutStore(pinia);
    browserStore.tabNew=async(url)=>{window.calls.push(url)};
    layout.activateBrowser=()=>window.calls.push('browser-activated');
    window.mount=(component)=>{window.instance?.unmount();window.instance=createApp(component).use(pinia);window.instance.mount('#app')};
    window.mount(Home);
    window.mountBfc=()=>window.mount(Bfc);
  </script>`;
  await page.route('**/__home_bfc_fixture',route=>route.fulfill({contentType:'text/html',body:html}));
  await page.goto(base+'/__home_bfc_fixture');
  const draft=page.getByRole('button',{name:'📣 公众号草稿箱'});
  await draft.waitFor();
  assert.equal(await draft.getAttribute('aria-expanded'),'false');
  await draft.click();
  await page.getByRole('heading',{name:'公众号 · 素材与草稿箱'}).waitFor({timeout:12000}).catch(async()=>{
    await page.getByText('微信公众号 · 素材与草稿箱').waitFor({timeout:2000});
  });
  assert.equal(await draft.getAttribute('aria-expanded'),'true');
  await page.evaluate(()=>document.documentElement.dataset.theme='dark');
  const homeBackground=await page.locator('.home-wechat-drafts').evaluate(el=>getComputedStyle(el).backgroundColor);
  assert.notEqual(homeBackground,'rgb(255, 255, 255)','dark theme WeChat panel must use dark surface');
  await draft.click();
  assert.equal(await page.getByText('微信公众号 · 素材与草稿箱').count(),0);
  await page.evaluate(()=>window.mountBfc());
  await page.getByRole('button',{name:/新发现的收藏/}).waitFor();
  const latest=page.getByRole('button',{name:/获取最新收藏/});
  await latest.waitFor();
  await latest.click();
  await page.getByText(/已打开 BFC 数据管理的安全补采页面/).waitFor();
  assert.deepEqual(await page.evaluate(()=>window.calls),['http://127.0.0.1:4198/manage#sync','browser-activated']);
  await page.evaluate(()=>document.documentElement.dataset.theme='dark');
  const contrast=await page.locator('.bfc-card').first().evaluate(el=>({background:getComputedStyle(el).backgroundColor,foreground:getComputedStyle(el).color}));
  assert.notEqual(contrast.background,'rgb(255, 255, 255)','dark theme must not render white launcher cards');
  assert.deepEqual(errors,[]);
  console.log('HOME_BFC_MANAGED_UI=PASS lazy WeChat draft entry, BFC authorized import flow, dark-theme contrast; no live provider requests');
 }finally{await browser.close()}
})().catch(error=>{console.error(error);process.exitCode=1});
