#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
const ROOT=join(dirname(fileURLToPath(import.meta.url)),"..");
const read=(p)=>readFileSync(join(ROOT,p),"utf8");
const failures=[]; const ok=(x)=>console.log("  ok   "+x); const fail=(x)=>{failures.push(x);console.error("  FAIL "+x)};
function extract(src,name){const sig="async function "+name+"(";const s=src.indexOf(sig);if(s<0)return"";let d=0;for(let i=src.indexOf("{",s);i<src.length;i++){if(src[i]==="{")d++;else if(src[i]==="}"&&--d===0)return src.slice(s,i+1)}return""}
function run(grid,browser,gridHost,browserHost){
 const close=extract(grid,"closeGridAll");
 if(close) ok("Grid owner closeGridAll exists"); else fail("Grid owner closeGridAll missing");
 for(const [label,re] of [
  ["gridOpen=false",/gridOpen\.value\s*=\s*false/],
  ["native closeGrid",/bridge\.closeGrid\(/],
  ["gridRects clear",/gridRects\.splice\(\s*0\s*,\s*gridRects\.length/],
  ["grid view guard",/layout\.mainView\s*===\s*"grid"/],
  ["browser view restore",/layout\.setView\(\s*"browser"\s*\)/],
  ["Browser public tab restore",/browser\.tabSwitch\(browser\.activeTabId\)/],
  ["Browser public relocate",/browser\.relocate\(\)/],
 ]) re.test(close)?ok(label):fail(label);
 if(/gridOpen|gridSession|buildGrid|closeGridAll|activateGrid/.test(browser)) fail("Browser store still owns Grid lifecycle"); else ok("Browser store has no Grid lifecycle");
 const vis=browser.match(/const isBrowserVisible = computed\([\s\S]*?\);/)?.[0]||"";
 if(/mainView === "browser"/.test(vis)&&!/gridOpen/.test(vis)) ok("Browser visibility independent from Grid"); else fail("Browser visibility formula regressed");
 if(/gridPosition\(|gridSetZoom\(/.test(gridHost)) ok("GridHost owns grid native positioning"); else fail("GridHost missing native positioning");
 if(/gridPosition\(|gridSetZoom\(|scheduleGrid/.test(browserHost)) fail("BrowserHost still owns Grid native positioning"); else ok("BrowserHost free of Grid positioning");
}
if(process.argv.includes("--help")){console.log("check-grid-close-logic.mjs [--self-test|--json]");process.exit(0)}
if(process.argv.includes("--self-test")){
 const g='const gridOpen={value:true}; const gridRects=[]; async function closeGridAll(){gridOpen.value=false;await bridge.closeGrid();gridRects.splice(0,gridRects.length);if(layout.mainView==="grid")layout.setView("browser");await browser.tabSwitch(browser.activeTabId);browser.relocate();}';
 const b='const isBrowserVisible = computed(() => layout.mainView === "browser");';
 run(g,b,'bridge.gridPosition(); bridge.gridSetZoom();','function schedulePosition(){}');
 console.log(failures.length?"GRID_CLOSE_SELF_TEST: FAIL":"GRID_CLOSE_SELF_TEST: ALL_PASS");process.exit(failures.length?1:0);
}
run(
 read("src/capabilities/grid/state/useGridStore.ts"),
 read("src/capabilities/browser/state/useBrowserStore.ts"),
 read("src/capabilities/grid/composables/useGridHost.ts"),
 read("src/composables/useBrowserHost.ts"),
);
console.log("GRID_CLOSE_RESULT="+(failures.length?"FAIL":"PASS"));
process.exit(failures.length?1:0);
