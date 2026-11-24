#!/usr/bin/env node
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
const read = p => readFileSync(p, "utf8");
const browser = read("src/capabilities/browser/state/useBrowserStore.ts");
const nav = read("src/composables/browserNav.ts");
const bar = read("src/components/layout/ActivityBar.vue");
const layout = read("src/stores/useLayoutStore.ts");
const grid = read("src/capabilities/grid/state/useGridStore.ts");
const gridNav = read("src/capabilities/grid/ui/GridNav.vue");
const app = read("src/App.vue");
const top = read("src/components/layout/TopBar.vue");
const tabs = read("src/components/layout/UnifiedTabBar.vue");
const files = read("src/capabilities/workspace/state/useFileStore.ts");
function ownerFunction(source, name, indent = "  ") {
  const start = source.indexOf("function " + name + "(");
  const end = start < 0 ? -1 : source.indexOf("\n" + indent + "}", start);
  return end < 0 ? "" : source.slice(start, end);
}
function blankOnly(source) {
  const consume = ownerFunction(source, "consumeActiveBlankTab");
  const close = ownerFunction(source, "closeTabNow");
  return /const id = expectedId \?\? \(layout\.mainView === "browser" \? activeTabId\.value : ""\);/.test(consume)
    && /if \(!id \|\| tabs\.find\(tab => tab\.id === id\)\?\.url !== "about:blank"\) return false;/.test(consume)
    && /closeTabNow\(id, true\)/.test(consume)
    && /return mutateTabs\(async \(\) => \{[\s\S]*?if \(!target \|\| \(onlyBlank && target\.url !== "about:blank"\)\) return false;[\s\S]*?await closeNativeTab\(id\)/.test(close)
    && /await bridge\.tabClose\(id\)/.test(ownerFunction(source, "closeNativeTab"));
}
function commitsReadableDirectory(source) {
  const body = ownerFunction(source, "openDirCenter", "");
  return /const blankId = layout\.mainView === "browser" && browser\.activeTab\?\.url === "about:blank"\s*\? browser\.activeTabId : undefined;/.test(body)
    && /publish = layout\.prepareDirTab\(p, true\)/.test(body)
    && /if \(!publish \|\| !await fs\.enterDir\(p, publish\)\) return;/.test(body)
    && /if \(blankId\) \{\s*try \{ await browser\.consumeActiveBlankTab\(blankId\); \}/.test(body)
    && body.indexOf("await fs.enterDir(p, publish)") < body.indexOf("await browser.consumeActiveBlankTab(blankId)");
}
function createsDirectoryTab(source) {
  const body = ownerFunction(source, "ctxOpenInNewTab");
  return /if \(entry\.is_dir\) \{[\s\S]*?const publish = layout\.prepareDirTab\(entry\.path\);/.test(body)
    && /if \(!publish \|\| !await enterDir\(entry\.path, publish\)\) return;/.test(body);
}
const checks = [
  ["only captured blank tabs are replaceable, including after queueing", blankOnly(browser)],
  ["module router reserves before closing and publishes after completion", /const publish = useLayoutStore\(\)\.prepareModule\(view\)[\s\S]*?if \(!publish\) return[\s\S]*?await useBrowserStore\(\)\.consumeActiveBlankTab\(\)[\s\S]*?publish\(\)/.test(nav)],
  ["module publishing uses the existing guarded Layout owner", /function prepareModule\(view: MainView\)[\s\S]*?if \(existing\) return prepareModTabActivation\(existing\.id\);[\s\S]*?return navigationCommit\(publish\.isCurrent,[\s\S]*?if \(!publish\(\)\) return false;/.test(layout)],
  ["grid uses shared router", /return openModuleInCurrentTab\("grid"\)/.test(grid)],
  ["grid button uses canonical action", /grid\.activateGrid\(\)/.test(gridNav)],
  ["grid shortcut uses router", /openModuleInCurrentTab\("grid"\)/.test(app)],
  ["directory commits a successful read before consuming its placeholder", commitsReadableDirectory(bar)],
  ["directory navigation reuses active directory", /reuseCurrent && mainView.value === "files"/.test(layout)],
  ["context menu retains explicit new-tab policy", createsDirectoryTab(files)],
  ["URL omnibox retains current tab", /browser\.navigateCurrent\(\)/.test(bar)],
  ["fallback bar retains current tab", /browser\.navigateCurrent/.test(top) && !/@keyup.enter="browser.openBrowser"/.test(top)],
  ["titlebar has arrow cursor", /\.titlebar-drag \{[^}]*cursor: default;/.test(tabs)],
  ["native dragging handler retained", /@mousedown="startWindowDrag"/.test(tabs)],
  ["negative: a queued placeholder becoming occupied cannot be closed", !blankOnly(browser.replace('(onlyBlank && target.url !== "about:blank")', "false"))],
  ["negative: failed directory I/O cannot consume the placeholder", !commitsReadableDirectory(bar.replace("!await fs.enterDir(p, publish)", "false"))],
  ["negative: explicit directory new-tab cannot reuse the active tab", !createsDirectoryTab(files.replace("layout.prepareDirTab(entry.path);", "layout.prepareDirTab(entry.path, true);"))],
];
for (const [name, valid] of checks) console.log((valid ? "PASS " : "FAIL ") + name);
assert.ok(checks.every(x => x[1]), "Navigation regression failed");
console.log("CURRENT_TAB_SURFACE_REUSE=PASS");
