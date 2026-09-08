// Browser UI fixtures only; native IPC is verified separately by workbench-smoke.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
(async () => {
  const browser = await chromium.launch({headless:true,executablePath:process.env.CHROME_BIN || '/usr/bin/google-chrome'});
  const page = await browser.newPage({viewport:{width:1200,height:800}});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.goto(process.env.WORKBENCH_URL || 'http://127.0.0.1:1421');
  await page.getByRole('button',{name:'统一命令',exact:true}).click();
  await page.getByRole('combobox',{name:'搜索命令'}).fill('vault');
  await page.keyboard.press('Enter');
  await page.getByLabel('Vault 目录').waitFor();
  await page.evaluate(async () => {
    const {bridge}=await import('/src/bridge.ts');
    bridge.vaultOpen=async()=>({root:'/fixture/Vault',notes:[
      {path:'Home.md',text:'# Home\n\n[[notes/Plan|项目计划]]\n\n<script>window.__unsafe=true</script>'},
      {path:'notes/Plan.md',text:'# Plan\n\n## Milestone\n验收定位关键字\n\n[Home](../Home.md)\n\n```ts\nconst answer = 42;\n```'},
    ],skipped:0,truncated:false});
    bridge.dbListConnections=async()=>[{id:'fixture',name:'Fixture SQLite',kind:'sqlite',database:'/fixture/test.db',allow_write:false,enabled:true,ssl_mode:'disable'}];
    bridge.dbQuery=async p=>({query_id:p.query_id||'fixture',columns:['name','value'],rows:p.sql.includes('sqlite_schema')?[[{text:'items'},{text:'table'}]]:Array.from({length:123},(_,i)=>[{text:`row-${i}`},{i64:i}]),row_count:p.sql.includes('sqlite_schema')?1:123,truncated:false,field_truncated:false,elapsed_ms:2});
  });
  await page.getByLabel('Vault 目录').fill('/fixture/Vault');
  await page.getByTitle('打开 Vault',{exact:true}).click();
  await page.getByRole('link',{name:'项目计划'}).click();
  await page.getByRole('heading',{name:'Plan',exact:true}).waitFor();
  assert.equal(await page.evaluate(()=>!!window.__unsafe),false);
  await page.getByLabel('搜索笔记',{exact:true}).fill('验收定位关键字');
  await page.locator('.note-row').click();
  assert.equal(await page.locator('.vault-reader .hit').getAttribute('data-line'),'4');
  await page.getByRole('button',{name:'阅读',exact:true}).click();
  await page.getByLabel('搜索笔记',{exact:true}).fill('');
  assert.ok(await page.locator('.vault-graph circle').count()>=2);
  fs.mkdirSync(process.env.WORKBENCH_SCREENSHOTS || '/tmp/workbench-screens',{recursive:true});
  for (const [width,height] of [[900,600],[1200,800],[1920,1080]]) {
    await page.setViewportSize({width,height});
    await page.screenshot({path:`${process.env.WORKBENCH_SCREENSHOTS || '/tmp/workbench-screens'}/vault-${width}.png`});
    assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth),`overflow at ${width}`);
  }
  await page.setViewportSize({width:1200,height:800});
  await page.locator('.workbench-rail').getByRole('button',{name:'数据库',exact:true}).click();
  await page.getByRole('button',{name:'Fixture SQLite'}).click();
  await page.getByLabel('SQL 编辑器').fill('SELECT 123');
  await page.getByRole('button',{name:'运行',exact:true}).click();
  await page.locator('.result table tbody tr').first().waitFor();
  assert.equal(await page.locator('.result table tbody tr').count(),50);
  await page.getByTitle('下一页',{exact:true}).click();
  assert.equal(await page.locator('.result table tbody tr').first().innerText(),'row-50\t50');
  await page.getByTitle('新建查询',{exact:true}).click();
  assert.equal(await page.getByLabel('SQL 编辑器').inputValue(),'');
  await page.getByRole('tab',{name:/SQL 1/}).click();
  assert.equal(await page.getByLabel('SQL 编辑器').inputValue(),'SELECT 123');
  await page.screenshot({path:`${process.env.WORKBENCH_SCREENSHOTS || '/tmp/workbench-screens'}/database-1200.png`});
  await page.getByRole('button',{name:'折叠或恢复工具窗'}).click();
  assert.equal(await page.locator('.workbench-rail button').count(),1);
  await page.locator('.workbench-rail').getByRole('button',{name:'恢复工具窗'}).click();
  await page.keyboard.press('Control+k');
  await page.getByRole('combobox',{name:'搜索命令'}).waitFor();
  await page.keyboard.press('Escape');
  assert.deepEqual(errors,[]);
  await browser.close();
  console.log('WORKBENCH_UI=PASS (fixture bridge; command focus, Vault links/search/sanitization/graph, SQL pagination/documents, 3 viewport screenshots)');
})().catch(e=>{console.error(e);process.exit(1)});
