#!/usr/bin/env node
// Focused regression guards for omnibox, native tab placement, and grid startup.
// These are static source contracts; GUI evidence remains required for release.
import { readFileSync } from "node:fs";
const read = (p) => readFileSync(p, "utf8");
const browser = read("src/capabilities/browser/state/useBrowserStore.ts");
const host = read("src/composables/useBrowserHost.ts");
const bar = read("src/components/layout/ActivityBar.vue");
const grid = read("src/capabilities/grid/state/useGridStore.ts");
const tabs = read("src/components/layout/UnifiedTabBar.vue");
const cases = [
  ["native viewport normalizer imported", /import\s*\{\s*normalizeHostRect\s*\}/.test(host)],
  ["failed tab position invalidates dedupe cache", /tabPosition\(tabId,[\s\S]*?\.catch\(\(error\)[\s\S]*?deduper\.reset\(\)/.test(host)],
  ["omnibox does not create a second tab when one is active", /async function navigateCurrent\(\)[\s\S]*?const current = tabs\.find[\s\S]*?if \(!current\) \{\s*await tabNew\(target\);[\s\S]*?await bridge\.tabOpen\(current\.id, target\)/.test(browser)],
  ["address submit uses navigateCurrent", /function onAddrGo\(\)[\s\S]*?browser\.navigateCurrent\(\)/.test(bar)],
  ["history selection uses navigateCurrent", /function pickUrl\(u: string\)[\s\S]*?browser\.navigateCurrent\(\)/.test(bar)],
  ["new tab is still explicit", /@click="browser\.tabNew\(\)"/.test(tabs)],
  ["grid startup guarded against overlap", /let gridBuilding = false;[\s\S]*?async function buildGrid\(\) \{\s*if \(gridBuilding\)/.test(grid)],
  ["grid startup clears guard on failure", /catch \(error\)[\s\S]*?finally \{\s*gridBuilding = false;/.test(grid)],
  ["no obsolete browser grid lifecycle calls", !/browser\.(?:gridOpen|layoutGrid|openGrid)\b/.test(tabs)],
];
let failures=0;
for(const [name,passed] of cases) {
  console.log((passed?"PASS ":"FAIL ")+name);
  if(!passed) failures++;
}
console.log("BROWSER_GRID_NAV_REGRESSION="+(failures?"FAIL":"PASS"));
process.exitCode=failures?1:0;
