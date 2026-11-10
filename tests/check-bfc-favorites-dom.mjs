import assert from 'node:assert/strict';
import { runInNewContext } from 'node:vm';
import { buildFavoritesReadbackScript } from '../src/capabilities/bfc/model/readbackScripts.mjs';

const anchors=['BV1234567890','BV1234567891'].map((bvid,i)=>({
  getAttribute:(name)=>name==='href'?`https://www.bilibili.com/video/${bvid}`:name==='title'?`视频${i+1}`:null,
  closest:()=>null, parentElement:{querySelector:()=>null,innerText:`视频${i+1}`},innerText:`视频${i+1}`
}));
const container={getClientRects:()=>[{}],querySelectorAll:()=>anchors};
const selectedFolder={innerText:'默认收藏夹 (2)'};
const calls=[];
const document={
  body:{innerText:'默认收藏夹 2'},title:'个人收藏夹',documentElement:{dataset:{}},
  querySelectorAll:(selector)=>selector.includes('fav-item')&&selector.includes('active')?[selectedFolder]
    :selector.includes('fav-list')?[container]:[],
  querySelector:(selector)=>selector.includes('fav-title')?{innerText:'默认收藏夹'}:null
};
const window={__TAURI__:{core:{invoke:async(command,data)=>calls.push({command,data})}}};
await runInNewContext(buildFavoritesReadbackScript('01234567-89ab-cdef-0123-456789abcdef','471851304'),{
  window,document,location:{href:'https://space.bilibili.com/471851304/favlist?fid=99'},URL,setTimeout
});
assert.equal(document.documentElement.dataset.browserosBfcReadback,'ready');
assert.deepEqual(calls.map(call=>call.data.payload.kind),['favorites_start','progress','favorites']);
assert.equal(calls[1].data.payload.pages_captured,1);
assert.equal(calls[1].data.payload.items_captured,2);
assert.equal(calls[1].data.payload.reported_total,2);
assert.equal(calls[2].data.payload.folder_name,'默认收藏夹');
assert.equal(calls[2].data.payload.pages.length,1);
console.log('BFC_FAVORITES_VISIBLE_DOM=PASS (selected folder total, 2 real-card BV, per-page progress, final snapshot)');
