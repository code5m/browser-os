#!/usr/bin/env node
import { build } from "esbuild";
import { pathToFileURL, fileURLToPath } from "node:url";
import { writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import os from "node:os";
const ROOT=join(dirname(fileURLToPath(import.meta.url)),"..");
globalThis.window=globalThis.window||{setTimeout:(f,m)=>setTimeout(f,m),clearTimeout:(h)=>clearTimeout(h),requestAnimationFrame:(cb)=>setTimeout(()=>cb(Date.now()),1),addEventListener(){},removeEventListener(){}};
globalThis.localStorage=globalThis.localStorage||{getItem(){return null},setItem(){}};
const STUBS={
 "stub-bridge":`const calls=globalThis.__bridgeCalls||(globalThis.__bridgeCalls=[]);export const bridge=new Proxy({},{get(_t,p){if(p==='createGrid')return async(n)=>{calls.push({name:'createGrid',args:[n]});return n};if(p==='debugLog')return()=>{};return async(...a)=>{calls.push({name:String(p),args:a)};}});`,
 "stub-guard":`export const isGridResourceAllowed=()=>true;export const isBrowserResourceAllowed=()=>true;`,
 "stub-tauri":`export const invoke=async()=>{};export default {invoke:async()=>{}};`
};
const plugin={name:"stubs",setup(b){
 b.onResolve({filter:/\.\.\/bridge$/},()=>({path:"stub-bridge",namespace:"stub"}));
 b.onResolve({filter:/resource\/guard$/},()=>({path:"stub-guard",namespace:"stub"}));
 b.onResolve({filter:/^@tauri-apps\//},()=>({path:"stub-tauri",namespace:"stub"}));
 b.onLoad({filter:/.*/,namespace:"stub"},a=>({contents:STUBS[a.path],loader:"ts"}));
}};
const res=await build({stdin:{contents:
 'export { useGridStore } from "./src/capabilities/grid/state/useGridStore.ts";\n'+
 'export { useBrowserStore } from "./src/capabilities/browser/state/useBrowserStore.ts";\n'+
 'export { useLayoutStore } from "./src/stores/useLayoutStore.ts";\n'+
 'export { createPinia,setActivePinia } from "pinia";\n',resolveDir:ROOT,loader:"ts"},
 bundle:true,format:"esm",platform:"node",write:false,plugins:[plugin],logLevel:"error"});
const p=join(os.tmpdir(),"grid-runtime-v2.mjs");writeFileSync(p,res.outputFiles[0].text);
const m=await import(pathToFileURL(p).href);m.setActivePinia(m.createPinia());
const grid=m.useGridStore(), browser=m.useBrowserStore(), layout=m.useLayoutStore(), calls=globalThis.__bridgeCalls;
const R=[];const check=(n,c,d="")=>R.push({n,pass:!!c,d});
const creates=()=>calls.filter(x=>x.name==="createGrid").length, closes=()=>calls.filter(x=>x.name==="closeGrid").length;
layout.activateBrowser();
check("init grid absent",!grid.gridOpen);
await grid.activateGrid();check("activate view",layout.mainView==="grid");check("create resource",grid.gridOpen&&creates()===1);
layout.activateBrowser();check("hide not destroy",grid.gridOpen&&closes()===0);
const s=grid.gridSession;await grid.activateGrid();check("reactivate no rebuild",creates()===1&&grid.gridSession===s);
for(let i=0;i<10;i++){layout.activateBrowser();await grid.activateGrid()}check("repeat no duplicate create",creates()===1);
grid.forceGridRelayout();check("relayout no destroy",grid.gridOpen&&closes()===0);
await grid.closeGrid();check("destroy",!grid.gridOpen&&closes()>=1);check("view converges browser",layout.mainView==="browser");
check("Browser retains tab owner",typeof browser.tabSwitch==="function");check("Grid has independent owner",typeof grid.buildGrid==="function");
const pass=R.filter(x=>x.pass).length;for(const r of R)console.log((r.pass?"  ok ":"  FAIL ")+r.n+(r.d?" ["+r.d+"]":""));
console.log(`RUNTIME_HARNESS=${pass===R.length?"PASS":"FAIL"} (${pass}/${R.length})`);process.exit(pass===R.length?0:1);
