import assert from 'node:assert/strict';
import { runInNewContext } from 'node:vm';
import { buildFavoritesReadbackScript } from '../src/capabilities/bfc/model/readbackScripts.mjs';

const anchors=['BV1234567890','BV1234567891'].map((bvid,i)=>({
  getAttribute:(name)=>name==='href'?`https://www.bilibili.com/video/${bvid}`:name==='title'?`视频${i+1}`:null,
  closest:()=>null, parentElement:{querySelector:()=>null,innerText:`视频${i+1}`},innerText:`视频${i+1}`
}));
const container={getClientRects:()=>[{}],querySelectorAll:()=>anchors};
for (const scenario of [
  {selected:true,text:'默认收藏夹 2'},
  {selected:false,text:'共 2 个视频',header:true},
  {selected:false,text:'总计 2 条收藏',header:true},
  {selected:false,text:'2 个视频',header:true},
  {selected:false,text:'默认收藏夹\n公开 视频数: 2 播放量: 226',header:true},
  {selected:false,text:'视频数：2',header:true},
  {selected:false,text:'视频数：2 共 3 个视频',header:true,unknown:true},
  {selected:false,text:'视频数：2',header:true,hidden:true,unknown:true},
  {selected:true,folderText:'编程学习\n2',text:'其他收藏夹 999'},
  {selected:false,text:'默认收藏夹\n2',generic:true,unknown:true},
  {selected:false,text:'推荐视频 共 2 个视频',unknown:true},
  {selected:false,text:'收藏夹，播放量 200',unknown:true},
]) {
const selectedFolder={innerText:scenario.folderText || '默认收藏夹 (2)',getClientRects:()=>[{}]};
const calls=[];
const document={
  body:{innerText:scenario.text},title:'个人收藏夹',documentElement:{dataset:{}},
  querySelectorAll:(selector)=>selector.includes('fav-item')&&selector.includes('active')?(scenario.selected?[selectedFolder]:[])
    :scenario.generic&&selector.startsWith('[class*="fav"],[id*="fav"]')?[{getClientRects:()=>[{}],innerText:scenario.text,querySelectorAll:()=>[]}]
    :scenario.header&&selector.includes('.fav-info')?[{innerText:scenario.text,getClientRects:()=>scenario.hidden?[]:[{}]}]
    :selector.includes('fav-list')?[container]:[],
  querySelector:(selector)=>selector.includes('fav-title')?{innerText:'默认收藏夹'}:null
};
const window={__TAURI__:{core:{invoke:async(command,data)=>calls.push({command,data})}}};
await runInNewContext(buildFavoritesReadbackScript('01234567-89ab-cdef-0123-456789abcdef','123456'),{
  window,document,location:{href:'https://space.bilibili.com/123456/favlist?fid=99'},URL,setTimeout
});
if (scenario.unknown) {
  assert.deepEqual(calls.map(call=>call.data.payload.kind),['favorites_start','abort']);
  assert.equal(calls[1].data.payload.message_code,'BFC_TOTAL_UNKNOWN');
  continue;
}
assert.equal(document.documentElement.dataset.browserosBfcReadback,'ready');
assert.deepEqual(calls.map(call=>call.data.payload.kind),['favorites_start','progress','checkpoint','favorites']);
assert.equal(calls[1].data.payload.pages_captured,1);
assert.equal(calls[1].data.payload.items_captured,2);
assert.equal(calls[1].data.payload.reported_total,2);
assert.equal(calls[2].data.payload.folder_name,'默认收藏夹');
assert.equal(calls[2].data.payload.pages.length,1);
assert.equal(JSON.stringify(calls[2].data.payload.pages[0].items.map(item=>item.title)),JSON.stringify(['视频1','视频2']));
}
// 虚拟时钟复现第三页分批渲染：不得把首批卡片当作完整页面。
for (const scenario of ['delayed', 'incomplete', 'budget', 'duplicate', 'not-advanced', 'stale-page', 'blocked', 'cancelled']) {
  let page = 1, clock = 0, started = 0;
  const calls = [];
  const dataset = {};
  const fixtureAnchors = () => Array.from({length: page === 3 &&
    (scenario === 'incomplete' || clock - started < 2500) ? 1 : 2}, (_, i) => ({
      getAttribute: key => key === 'href' ? `https://www.bilibili.com/video/BV${String((scenario === 'not-advanced' && page === 3 ? 2 : scenario === 'duplicate' && page === 3 && i === 0 ? 1 : page) * 10 + i).padStart(10, '0')}` : null,
      closest: () => null, querySelector: () => null, innerText: 'fixture',
    }));
  const fixtureContainer = {getClientRects: () => [{}], querySelectorAll: fixtureAnchors};
  const next = {innerText: '下一页', className: 'next', getAttribute: () => null,
    hasAttribute: () => false, click: () => { page++; started = clock; }};
  const pager = {getClientRects: () => [{}], querySelectorAll: () => [next],
    querySelector: () => ({innerText: String(['not-advanced', 'stale-page'].includes(scenario) && page === 3 ? 2 : page)})};
  const document = {title: 'fixture', body: {innerText: ''}, documentElement: {dataset},
    querySelector: () => null, querySelectorAll: selector => selector.includes('pager') ? [pager]
      : selector.includes('fav-item') ? [{innerText: `默认收藏夹 (${scenario === 'budget' ? 8 : 6})`, getClientRects: () => [{}]}]
      : selector.includes('fav-list') ? [fixtureContainer] : []};
  const window = {__TAURI__: {core: {invoke: async (_, data) => calls.push(structuredClone(data.payload))}}};
  await runInNewContext(buildFavoritesReadbackScript('fixture', '123'), {
    document, window,
    location: {href: 'https://space.bilibili.com/123/favlist?fid=99'}, URL,
    Date: {now: () => clock}, setTimeout: (callback, ms) => {
      clock += ms;
      if (page === 3 && scenario === 'blocked') document.body.innerText = '安全验证';
      if (page === 3 && scenario === 'cancelled') window.__browserosBfcCancelScan = 'fixture';
      callback();
    },
  });
  assert.equal(page, 3, '失败或受限后不得继续翻页');
  if (scenario === 'delayed') {
    const snapshot = calls.find(call => call.kind === 'favorites');
    assert.ok(snapshot, '延迟加载完成后应生成完整快照');
    assert.equal(snapshot.pages.flatMap(entry => entry.items).length, 6);
    assert.ok(clock - started >= 3700, '完整条目必须稳定后才能读取');
  } else if (scenario === 'incomplete' || scenario === 'budget') {
    assert.equal(calls.at(-1).kind, 'partial');
    assert.equal(calls.at(-1).pages.flatMap(entry => entry.items).length, scenario === 'incomplete' ? 5 : 6);
    assert.equal(calls.filter(call => call.kind === 'checkpoint').length, 3);
    assert.equal(calls.some(call => call.kind === 'favorites'), false);
  } else {
    assert.equal(calls.some(call => call.kind === 'favorites'), false);
    if (scenario !== 'cancelled') assert.equal(calls.at(-1).message_code,
      scenario === 'blocked' ? 'BFC_BILIBILI_ACCESS_LIMITED' : scenario === 'duplicate' ? 'BFC_READBACK_CROSS_PAGE_DUPLICATE'
        : ['not-advanced', 'stale-page'].includes(scenario) ? 'BFC_PAGINATION_NOT_ADVANCED' : 'BFC_PAGINATION_ITEM_COUNT_MISMATCH');
    if (scenario === 'not-advanced') {
      const diagnostic = JSON.parse(dataset.browserosBfcPaginationDiagnostic);
      assert.equal(diagnostic.targetPage, 3);
      assert.equal(diagnostic.observedPage, scenario === 'not-advanced' ? 2 : 3);
      assert.equal(diagnostic.uniqueItems, scenario === 'incomplete' ? 1 : 2);
      assert.equal(diagnostic.expectedItems, 2);
      assert.equal(diagnostic.waitedMs, 20000);
      assert.equal(/BV|fixture|bilibili|ticket/.test(JSON.stringify(diagnostic)), false);
    }
  }
}
// The candidate selection must ignore uploader-name nodes and card metrics,
// but preserve genuine titles that begin with a number. Synthetic DOM only.
for (const pageCount of [4, 5]) {
  const expectedTitles = [
    '如何使用 AI 自动整理收藏夹',
    '从零实现一个浏览器工作台',
    '5.2 万用户都在看的教程',
    '标题未获取'
  ];
  const metrics = ['5.2万 00:28','3968 02:46','3.6万 13:00:52','8533 2 01:22'];
  const anchors = expectedTitles.map((title, index) => {
    const card = {
      innerText: 'UP主名称\n' + metrics[index],
      querySelectorAll: selector => selector.includes('[class*="title"]') && title !== '标题未获取'
        ? [{innerText:title,getAttribute:()=>null}] :
          selector.includes('[class*="name"]') ? [{innerText:'UP主名称',getAttribute:()=>null}] : []
    };
    return {
      getAttribute: key => key === 'href'
        ? 'https://www.bilibili.com/video/BV' + String(index+1).padStart(10,'0')
        : null,
      closest:()=>card,
      innerText:metrics[index],
    };
  });
  const folder={innerText:'默认收藏夹 ('+pageCount+')',getClientRects:()=>[{}]};
  const wrapper={getClientRects:()=>[{}],querySelectorAll:()=>anchors};
  const calls=[];
  const document={
    title:'fixture',body:{innerText:''},documentElement:{dataset:{}},
    querySelector:selector=>selector.includes('fav-title')?{innerText:'默认收藏夹'}:null,
    querySelectorAll:selector=>selector.includes('fav-item')&&selector.includes('active')?[folder]:
      selector.includes('fav-list')?[wrapper]:[]
  };
  const window={__TAURI__:{core:{invoke:async(_,data)=>calls.push(data.payload)}}};
  await runInNewContext(buildFavoritesReadbackScript('test-fixture','123456'),{
    document,window,URL,setTimeout,location:{href:'https://space.bilibili.com/123456/favlist?fid=99'}
  });
  if (pageCount === 5) {
    assert.equal(calls.at(-1)?.message_code,'BFC_START_FROM_FIRST_PAGE');
  } else {
    const saved=calls.find(call=>call.kind==='favorites');
    assert.ok(saved,'single-page collection without a pager should still succeed');
    assert.deepEqual(Array.from(saved.pages[0].items,item=>item.title),expectedTitles);
  }
}

// Empty folders can be captured only if an explicitly selected visible folder
// reports zero and an actual empty list container exists.
{
  const calls=[];
  const document={
    title:'fixture',body:{innerText:''},documentElement:{dataset:{}},
    querySelector:()=>null,
    querySelectorAll:selector=>selector.includes('fav-item')&&selector.includes('active')
      ? [{innerText:'空收藏夹 (0)',getClientRects:()=>[{}]}]
      : selector.includes('fav-list')
        ? [{getClientRects:()=>[{}],innerText:'暂无收藏的视频',querySelectorAll:()=>[]}] : []
  };
  const window={__TAURI__:{core:{invoke:async(_,data)=>calls.push(data.payload)}}};
  let clock=0;
  await runInNewContext(buildFavoritesReadbackScript('empty-folder-fixture','123456'),{
    document,window,URL,Date:{now:()=>clock},setTimeout:callback=>{clock+=250;callback();},location:{href:'https://space.bilibili.com/123456/favlist?fid=9'}
  });
  const saved=calls.find(call=>call.kind==='favorites');
  assert.ok(saved,'selected zero-count collection should be capturable');
  assert.equal(saved.reported_total,0);
  assert.deepEqual(Array.from(saved.pages[0].items),[]);
}


// A transient 0 header before favorites hydration must not be imported as an
// empty collection. The same fixture becomes a populated folder after 1s.
{
  let clock=0,loaded=false;
  const cards=[0,1].map(i=>({
    getAttribute:key=>key==='href'?'https://www.bilibili.com/video/BV'+String(i).padStart(10,'0'):
      key==='title'?'视频标题'+i:null,
    innerText:'',closest:()=>null,parentElement:{querySelectorAll:()=>[]}
  }));
  const list={getClientRects:()=>[{}],querySelectorAll:()=>loaded?cards:[]};
  const header={getClientRects:()=>[{}],
    get innerText(){return loaded?'默认收藏夹 公开 视频数: 2':'未命名收藏夹 公开 视频数: 0 播放量:';}};
  const calls=[];
  const document={title:'fixture',body:{innerText:''},documentElement:{dataset:{}},
    querySelector:selector=>selector.includes('fav-title')?{innerText:'默认收藏夹'}:null,
    querySelectorAll:selector=>selector.includes('fav-item')&&selector.includes('active')
      ? (loaded?[{innerText:'默认收藏夹 (2)',getClientRects:()=>[{}]}]:[]):
      selector.includes('.fav-info')?[header]:
      selector.includes('fav-list')?[list]:[]};
  const window={__TAURI__:{core:{invoke:async(_,data)=>calls.push(data.payload)}}};
  await runInNewContext(buildFavoritesReadbackScript('hydration-fixture','123456'),{
    document,window,URL,Date:{now:()=>clock},
    setTimeout:callback=>{clock+=250;if(clock>=1000)loaded=true;callback();},
    location:{href:'https://space.bilibili.com/123456/favlist?fid=99'}
  });
  const saved=calls.find(v=>v.kind==='favorites');
  assert.equal(saved?.reported_total,2,'transient zero must not replace real collection total: '+JSON.stringify({clock,loaded,calls:calls.map(v=>({kind:v.kind,message_code:v.message_code,reported_total:v.reported_total}))}));
  assert.equal(saved?.pages[0].items.length,2);
}

// A video cover link may have no text while its sibling title link is
// populated asynchronously. Never persist an all-placeholder page.
for(const titlesHydrate of [true,false]){
  let clock=0;
  const links=[];
  for(let i=0;i<2;i++){
    const href='https://www.bilibili.com/video/BV'+String(i+1).padStart(10,'0');
    const card={querySelectorAll:()=>[]};
    const coverWrapper={parentElement:card,querySelectorAll:()=>[]};
    const cover={getAttribute:key=>key==='href'?href:null,
      querySelector:()=>({getAttribute:()=> '视频封面'}),
      parentElement:coverWrapper,innerText:'00:28',textContent:'00:28',closest:()=>coverWrapper};
    const titleLink={getAttribute:key=>key==='href'?href:null,closest:()=>card,
      get innerText(){return titlesHydrate&&clock>=750?'视频标题'+(i+1):'';},
      get textContent(){return titlesHydrate&&clock>=750?'视频标题'+(i+1):'';}};
    card.querySelectorAll=selector=>selector.includes('a[href*="/video/BV"]')?[cover,titleLink]:
      selector.includes('[class*="title"]')&&titlesHydrate&&clock>=750?[titleLink]:[];
    links.push(cover,titleLink);
  }
  const list={getClientRects:()=>[{}],querySelectorAll:()=>links};
  const calls=[];
  const document={title:'fixture',body:{innerText:''},documentElement:{dataset:{}},
    querySelector:selector=>selector.includes('fav-title')?{innerText:'默认收藏夹'}:null,
    querySelectorAll:selector=>selector.includes('fav-item')&&selector.includes('active')
      ?[{innerText:'默认收藏夹 (2)',getClientRects:()=>[{}]}]:
      selector.includes('fav-list')?[list]:[]};
  const window={__TAURI__:{core:{invoke:async(_,data)=>calls.push(data.payload)}}};
  await runInNewContext(buildFavoritesReadbackScript('title-fixture','123456'),{
    document,window,URL,Date:{now:()=>clock},
    setTimeout:callback=>{clock+=250;callback();},
    location:{href:'https://space.bilibili.com/123456/favlist?fid=99'}
  });
  if(titlesHydrate){
    const saved=calls.find(v=>v.kind==='favorites');
    assert.ok(saved,'sibling BV title links must be picked up once rendered');
    assert.deepEqual(Array.from(saved.pages[0].items,v=>v.title),['视频标题1','视频标题2']);
    assert.ok(clock>=750,'must wait for title hydration, not only BV anchors');
  }else{
    assert.equal(calls.at(-1)?.message_code,'BFC_TITLES_NOT_READY');
    assert.equal(calls.some(v=>v.kind==='favorites'),false);
  }
}

console.log('BFC_FAVORITES_VISIBLE_DOM=PASS (counts, delayed pagination, bounded timeout, access limit, cancellation)');
