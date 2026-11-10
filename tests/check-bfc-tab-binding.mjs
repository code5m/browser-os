import assert from 'node:assert/strict';
import { runInNewContext } from 'node:vm';
import { buildLoginProbeScript } from '../src/capabilities/bfc/model/readbackScripts.mjs';
import { selectBfcReadbackTab, safeBfcTabSummary } from '../src/capabilities/bfc/model/tabBinding.mjs';

const tabs = [
  {id:'tab-3',title:'旧 B站',url:'https://www.bilibili.com/'},
  {id:'tab-9',title:'默认收藏夹',url:'https://space.bilibili.com/471851304/favlist?token=secret-value'},
  {id:'tab-10',title:'BFC 工作台',url:'http://127.0.0.1:4198/'}
];
assert.equal(selectBfcReadbackTab(tabs,'tab-9').id,'tab-9');
assert.throws(()=>selectBfcReadbackTab(tabs,'tab-9','tab-3'),/BFC_TAB_ID_MISMATCH/);
assert.throws(()=>selectBfcReadbackTab(tabs,'tab-404'),/BFC_TAB_ID_MISMATCH/);
assert.throws(()=>selectBfcReadbackTab(tabs,''),/BFC_TAB_NOT_ACTIVE/);
assert.throws(()=>selectBfcReadbackTab(tabs,'tab-10'),/BFC_FAVORITES_PAGE_REQUIRED/);
const safe = safeBfcTabSummary(tabs[1]);
assert.equal(safe.isFavorites,true);
assert.equal(safe.url,'https://space.bilibili.com/471851304/favlist');
assert.ok(!JSON.stringify(safe).includes('secret-value'));

function fixture({withBridge,midInHeader=false,midInPublicState=false}) {
  const calls=[];
  const anchor={className:'header-entry-avatar',getAttribute:(key)=>key==='href'?'https://space.bilibili.com/471851304':null,querySelector:()=>({tagName:'IMG'})};
  const header={innerText:'个人中心',querySelectorAll:(selector)=>selector.includes('space.bilibili.com') ? (midInHeader?[anchor]:[]) : []};
  const dataset={};
  const document={readyState:'complete',body:{innerText:'默认收藏夹 4869'},title:'我的收藏',documentElement:{dataset},querySelector:()=>header};
  const window={__INITIAL_STATE__:midInPublicState?{loginInfo:{mid:471851304}}:undefined};
  if(withBridge)window.__TAURI__={core:{invoke:async(command,data)=>{calls.push({command,data});}}};
  return {calls,dataset,document,window,location:{hostname:'space.bilibili.com'},setTimeout:(cb)=>queueMicrotask(cb)};
}
const script=buildLoginProbeScript('01234567-89ab-cdef-0123-456789abcdef');
{
 const env=fixture({withBridge:true,midInHeader:true});
 await runInNewContext(script,env);
 assert.deepEqual(env.calls.map(x=>x.command),['bfc_readback_submit']);
 assert.equal(env.calls[0].data.payload.login_status,'logged_in');
 assert.equal(env.calls[0].data.payload.mid,'471851304');
}
{
 const env=fixture({withBridge:true,midInPublicState:true});
 await runInNewContext(script,env);
 assert.equal(env.calls[0].data.payload.mid,'471851304');
 assert.equal(env.calls[0].data.payload.login_status,'logged_in');
}
{
 const env=fixture({withBridge:false,midInHeader:true});
 await runInNewContext(script,env);
 assert.equal(env.dataset.browserosBfcError,'BFC_LOGIN_PROBE_INJECTION_FAILED');
 assert.notEqual(env.dataset.browserosBfcLoginProbe,'logged_out');
 assert.equal(env.calls.length,0);
}
{
 const env=fixture({withBridge:true});
 await runInNewContext(script,env);
 assert.equal(env.calls[0].data.payload.login_status,'unknown');
 assert.notEqual(env.calls[0].data.payload.login_status,'logged_out');
}
console.log('BFC_TAB_ID_AND_LOGIN_PROBE_REGRESSION=PASS (correct tab, wrong tab, public MID, injection failure != logged_out)');
