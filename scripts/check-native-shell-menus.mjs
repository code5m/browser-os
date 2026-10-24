import assert from 'node:assert/strict';
import { build } from 'esbuild';

// Native transport fixture: verifies the adapter contract, not GTK rendering.
const fixture={created:[],closed:0,positions:[],native:true,fail:false};
globalThis.__nativeMenuFixture=fixture;
const result=await build({entryPoints:['src/composables/nativeMenu.ts'],bundle:true,write:false,format:'esm',platform:'node',plugins:[{name:'native-menu-fixture',setup(b){
  b.onResolve({filter:/^@tauri-apps\/api\//},a=>({path:a.path,namespace:'fixture'}));
  b.onLoad({filter:/.*/,namespace:'fixture'},a=>({contents:a.path.endsWith('/menu')?`export const Menu={new:async options=>{const f=globalThis.__nativeMenuFixture;if(f.fail)throw Error('transport failed');f.created.push(options);return {popup:async p=>{f.positions.push(p);if(f.popupFail)throw Error('popup denied');if(f.select)await f.select(options);},close:async()=>{f.closed++;}};}};`:a.path.endsWith('/dpi')?`export class LogicalPosition{constructor(x,y){this.x=x;this.y=y;}}`:`export const isTauri=()=>globalThis.__nativeMenuFixture.native;`,loader:'js'}));
}}]});
const {createNativeMenu}=await import('data:text/javascript;base64,'+Buffer.from(result.outputFiles[0].text).toString('base64'));
let opened='',errors=[];
const menu=createNativeMenu(e=>errors.push(String(e)));
const items=[{text:'Folder & test',children:[{text:'Open',action:()=>{opened='current';}},{text:'New tab',action:()=>{opened='new';}}]}];
fixture.select=async options=>options.items[0].items[1].action();
assert.equal(await menu.open(items,{x:120,y:88}),true);
assert.equal(opened,'new');assert.equal(fixture.positions[0].x,120);assert.equal(fixture.positions[0].y,88);
assert.equal(fixture.closed,1);assert.equal(items[0].text,'Folder & test');
assert.equal(fixture.created[0].items[0].text,'Folder && test');
fixture.select=async options=>{await options.items[0].action();};
await menu.open([{text:'Reject',action:async()=>{throw Error('navigation failed');}}],{x:0,y:0});
assert.match(errors.pop(),/navigation failed/);assert.equal(fixture.closed,2);
fixture.native=false;assert.equal(await menu.open(items,{x:1,y:2}),false);
assert.match(errors.pop(),/桌面客户端/);assert.equal(fixture.created.length,2);
fixture.native=true;fixture.fail=true;assert.equal(await menu.open(items,{x:1,y:2}),false);
assert.match(errors.pop(),/transport failed/);fixture.fail=false;
menu.dispose();assert.equal(await menu.open(items,{x:0,y:0}),false);
assert.equal(fixture.created.length,2);
const later=createNativeMenu(e=>errors.push(String(e)));
fixture.popupFail=true;
assert.equal(await later.open(items,{x:4,y:5}),false);assert.equal(fixture.closed,3);
assert.match(errors.pop(),/popup denied/);fixture.popupFail=false;
fixture.select=async options=>{
 assert.equal(await later.open(items,{x:0,y:0}),false,'duplicate popup blocked');
 later.dispose();
 await options.items[0].items[0].action();
};
opened='unchanged';
assert.equal(await later.open(items,{x:20,y:30}),false);
assert.equal(opened,'unchanged','disposed owner cannot navigate');assert.equal(fixture.closed,4);
console.log('NATIVE_SHELL_MENU_CONTRACT=PASS (transport fixture; desktop rendering remains separate)');
