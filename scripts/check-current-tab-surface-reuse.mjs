#!/usr/bin/env node
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
const read = p => readFileSync(p,"utf8");
const browser=read("src/capabilities/browser/state/useBrowserStore.ts");
const nav=read("src/composables/browserNav.ts");
const bar=read("src/components/layout/ActivityBar.vue");
const layout=read("src/stores/useLayoutStore.ts");
const grid=read("src/capabilities/grid/state/useGridStore.ts");
const gridNav=read("src/capabilities/grid/ui/GridNav.vue");
const app=read("src/App.vue");
const top=read("src/components/layout/TopBar.vue");
const tabs=read("src/components/layout/UnifiedTabBar.vue");
const checks=[
["only blank browser tabs are replaceable",/layout.mainView !== "browser" \|\| activeTab.value\?\.url !== "about:blank"/.test(browser)],
["blank replacement closes same tab",/async function consumeActiveBlankTab\(\)[\s\S]*?await tabClose\(id\)/.test(browser)],
["module router awaits replacement",/await useBrowserStore\(\)\.consumeActiveBlankTab\(\)/.test(nav)],
["grid uses shared router",/return openModuleInCurrentTab\("grid"\)/.test(grid)],
["grid button uses canonical action",/grid\.activateGrid\(\)/.test(gridNav)],
["grid shortcut uses router",/openModuleInCurrentTab\("grid"\)/.test(app)],
["directory enters then replaces placeholder",/await fs\.enterDir\(p\);\s*await browser\.consumeActiveBlankTab\(\);/.test(bar)],
["directory navigation reuses active directory",/reuseCurrent && mainView.value === "files"/.test(layout)],
["context menu retains explicit new-tab policy",/layout\.openDirTab\(entry.path\)/.test(read("src/capabilities/workspace/state/useFileStore.ts"))],
["URL omnibox retains current tab",/browser\.navigateCurrent\(\)/.test(bar)],
["fallback bar retains current tab",/browser\.navigateCurrent/.test(top) && !/@keyup.enter="browser.openBrowser"/.test(top)],
["titlebar has arrow cursor",/\.titlebar-drag \{[^}]*cursor: default;/.test(tabs)],
["native dragging handler retained",/@mousedown="startWindowDrag"/.test(tabs)],
];
for(const [name,valid] of checks)console.log((valid?"PASS ":"FAIL ")+name);
assert.ok(checks.every(x=>x[1]),"Navigation regression failed");
console.log("CURRENT_TAB_SURFACE_REUSE=PASS");
