#!/usr/bin/env node
// Focused regression guards for omnibox, native tab placement, and grid startup.
// These are static source contracts; actual owner behavior is exercised by
// tests/check-unified-tab-actions.mjs and native GUI evidence remains separate.
import { readFileSync } from "node:fs";
const read = (p) => readFileSync(p, "utf8");
const browser = read("src/capabilities/browser/state/useBrowserStore.ts");
const host = read("src/composables/useBrowserHost.ts");
const bar = read("src/components/layout/ActivityBar.vue");
const grid = read("src/capabilities/grid/state/useGridStore.ts");
const tabs = read("src/components/layout/UnifiedTabBar.vue");
function ownerFunction(source, name, indent = "  ") {
  const start = source.indexOf("function " + name + "(");
  const end = start < 0 ? -1 : source.indexOf("\n" + indent + "}", start);
  return end < 0 ? "" : source.slice(start, end);
}
function preservesCurrentTab(source) {
  const body = ownerFunction(source, "navigateCurrent");
  const reserve = body.indexOf('const publish = layout.prepareViewChange("browser");');
  const queue = body.indexOf("return mutateTabs(async () => {");
  return /const current = tabs\.find\(\(t\) => t\.id === activeTabId\.value\);/.test(body)
    && /if \(!current\) \{\s*const created = await tabNew\(target\);[\s\S]*?return created;\s*\}/.test(body)
    && (body.match(/\btabNew\(/g) ?? []).length === 1
    && reserve >= 0 && queue > reserve
    && /!tabs\.some\(\(t\) => t\.id === current\.id\)[\s\S]*?!publish\.isCurrent\(\)\) return false;[\s\S]*?await bridge\.tabOpen\(current\.id, target\)/.test(body);
}
function hasExplicitNewTab(source) {
  const handler = ownerFunction(source, "newTab", "");
  return /class="tab-new"[^>]*@click="newTab"/.test(source)
    && /try \{\s*await browser\.tabNew\(\); \}/.test(handler)
    && /catch \{\s*layout\.showToast\(/.test(handler);
}
const cases = [
  ["native viewport normalizer imported", /import\s*\{\s*normalizeHostRect\s*\}/.test(host)],
  ["failed tab position invalidates dedupe cache", /tabPosition\(tabId,[\s\S]*?\.catch\(\(error\)[\s\S]*?deduper\.reset\(\)/.test(host)],
  ["omnibox preserves its captured tab and guarded navigation", preservesCurrentTab(browser)],
  ["address submit uses navigateCurrent", /function onAddrGo\(\)[\s\S]*?browser\.navigateCurrent\(\)/.test(bar)],
  ["history selection uses navigateCurrent", /function pickUrl\(u: string\)[\s\S]*?browser\.navigateCurrent\(\)/.test(bar)],
  ["new tab is explicit and reports native failure", hasExplicitNewTab(tabs)],
  ["grid startup guarded against overlap", /let gridBuilding = false;[\s\S]*?async function buildGrid\(\) \{\s*if \(gridBuilding\)/.test(grid)],
  ["grid startup clears guard on failure", /catch \(error\)[\s\S]*?finally \{[\s\S]*?gridBuilding = false;/.test(grid) && /finally \{\s*gridStarting\.value = false;\s*gridBuilding = false;/.test(grid)],
  ["no obsolete browser grid lifecycle calls", !/browser\.(?:gridOpen|layoutGrid|openGrid)\b/.test(tabs)],
  ["negative: occupied tab cannot enter the create branch", !preservesCurrentTab(browser.replace("if (!current) {", "if (true) {"))],
  ["negative: queued navigation must check its reservation", !preservesCurrentTab(browser.replace("!isBrowserResourceAllowed() || !publish.isCurrent()) return false;\n      await bridge.tabOpen(current.id, target);", "!isBrowserResourceAllowed()) return false;\n      await bridge.tabOpen(current.id, target);"))],
  ["negative: native navigation cannot retarget the latest selection", !preservesCurrentTab(browser.replace("await bridge.tabOpen(current.id, target);", "await bridge.tabOpen(activeTabId.value, target);"))],
  ["negative: the new-tab action cannot become a reload", !hasExplicitNewTab(tabs.replace("await browser.tabNew();", "await browser.reloadActive();"))],
];
let failures = 0;
for (const [name, passed] of cases) {
  console.log((passed ? "PASS " : "FAIL ") + name);
  if (!passed) failures++;
}
console.log("BROWSER_GRID_NAV_REGRESSION=" + (failures ? "FAIL" : "PASS"));
process.exitCode = failures ? 1 : 0;
