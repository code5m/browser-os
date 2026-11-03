import assert from 'node:assert/strict';
import {projectTabResources,tabResourceByKey} from '../src/application/tabResourceProjection.mjs';

const web=[
  {id:'tab-1',url:'https://example.com',title:'Example'},
  {id:'dir-1',url:'about:blank',title:''}
];
const modules=[
  {id:'dir-1',view:'files',icon:'📁',label:'Project',path:'/project'},
  {id:'mod-1',view:'db',icon:'▤',label:'Database'}
];
const tabs=projectTabResources(web,modules,'browser','tab-1','mod-1');
assert.deepEqual(tabs.map(t=>t.key),['web:tab-1','web:dir-1','module:dir-1','module:mod-1']);
assert.equal(tabs.filter(t=>t.active).length,1);
assert.equal(tabs[0].nativeWebviewId,'tab-1');
assert.equal(tabs[2].nativeWebviewId,null,'a logical module must not fake a native WebView');
assert.equal(tabResourceByKey(tabs,'module:dir-1').path,'/project');
assert.equal(tabResourceByKey(tabs,'not-found'),null);
const activeModule=projectTabResources(web,modules,'db','tab-1','mod-1');
assert.deepEqual(activeModule.filter(t=>t.active).map(t=>t.key),['module:mod-1']);
const hiddenModule=projectTabResources(web,modules,'files','tab-1','mod-1');
assert.equal(hiddenModule.filter(t=>t.active).length,0,'stale module selection must not render active on another view');
assert.throws(()=>projectTabResources([web[0],web[0]],[], 'browser','tab-1',''),/TAB_ID_DUPLICATE/);
assert.throws(()=>projectTabResources([], [modules[0],modules[0]],'files','','dir-1'),/TAB_ID_DUPLICATE/);
console.log('UNIFIED_TAB_RESOURCE_PROJECTION=PASS');
