// Actual owner regressions with controlled native completions; GUI evidence is separate.
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";
import ts from "typescript";
import { createTabCoordinator } from "../src/application/tabCoordinator.mjs";
import { projectTabResources } from "../src/application/tabResourceProjection.mjs";
import * as documentPaths from "../src/application/documentPaths.mjs";

// Exercise the actual Layout/Browser owners with controlled native completions.
// This does not run Tauri, install an application, or infer native GUI acceptance.
function fixture() {
  const isRef = Symbol("test-ref");
  const reactive = (value) => value;
  const ref = (value) => ({ [isRef]: true, value });
  const computed = (getter) => ({ [isRef]: true, get value() { return getter(); } });
  const pinia = {
    defineStore(_id, setup) {
      let instance;
      return () => instance ??= new Proxy(setup(), {
        get(target, key) { const value = target[key]; return value?.[isRef] ? value.value : value; },
        set(target, key, value) {
          if (target[key]?.[isRef]) target[key].value = value;
          else target[key] = value;
          return true;
        },
      });
    },
  };
  const vue = { ref, reactive, computed, watch() {}, nextTick: (fn) => Promise.resolve().then(() => fn?.()) };
  function owner(path, modules) {
    const source = fs.readFileSync(new URL("../" + path, import.meta.url), "utf8");
    const output = ts.transpileModule(source, {
      compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
    }).outputText;
    const exports = {};
    vm.runInNewContext(output, {
      exports,
      require(name) {
        if (name === "pinia") return pinia;
        if (name === "vue") return vue;
        if (Object.hasOwn(modules, name)) return modules[name];
        throw Error("Unexpected dependency: " + name);
      },
      window: { setTimeout, clearTimeout }, setTimeout, clearTimeout, console,
    }, { filename: path });
    return exports;
  }
  let browser, lifecycle;
  const availability = { browser: true };
  const layoutModule = owner("src/stores/useLayoutStore.ts", {
    "../application/documentPaths.mjs": documentPaths,
    "../capabilities/browser/public": { useBrowserStore: () => browser },
  });
  const layout = layoutModule.useLayoutStore();
  const calls = [];
  let nextId = 0;
  const native = {
    debugLog() {},
    tabNew: async (url) => { calls.push(["new", url]); return { id: "new-" + ++nextId, url, title: url }; },
    tabActivate: async (id) => { calls.push(["activate", id]); },
    tabOpen: async (id, url) => { calls.push(["open", id, url]); },
    tabClose: async (id) => { calls.push(["close", id]); },
    hideAllWebviews: async () => { calls.push(["hide"]); },
    hideWebview: async () => {},
    evalInTab: async () => {},
  };
  const browserModule = owner("src/capabilities/browser/state/useBrowserStore.ts", {
    "../../../bridge": { bridge: native },
    "../../../utils/webviewFreeze": { WEBVIEW_FREEZE_JS: "", WEBVIEW_UNFREEZE_JS: "" },
    "../resource/guard": { isBrowserResourceAllowed: () => availability.browser },
    "../../../stores/useLayoutStore": { useLayoutStore: () => layout },
    "../model/nativeTabLoadFailure.mjs": { classifyNativeTabLoadFailure: () => null, recentNativeTabLoadFailure: () => null },
    "../../../composables/recentsNav": { recordRecentUrl() {} },
    "../lifecycle": { registerBrowserLifecycleBinding(binding) { lifecycle = binding; } },
  });
  browser = browserModule.useBrowserStore();
  const resources = () => projectTabResources(browser.tabs, layout.modTabs, layout.mainView, browser.activeTabId, layout.activeModTab);
  const actions = createTabCoordinator({
    resources,
    activate: (tab) => tab.kind === "web" ? browser.tabSwitch(tab.id) : layout.activateModTab(tab.id),
    close: (tab) => tab.kind === "web" ? browser.tabClose(tab.id) : layout.closeModTab(tab.id),
  });
  const navigation = owner("src/composables/browserNav.ts", {
    "../capabilities/browser/public": { useBrowserStore: () => browser },
    "../stores/useLayoutStore": { useLayoutStore: () => layout },
  });
  return { layout, browser, native, calls, actions, resources, lifecycle, navigation, availability };
}

{
  const { layout, browser, calls, actions } = fixture();
  layout.openModule("files");
  browser.tabs.push({ id: "web-1", url: "https://one.test", title: "One" });
  browser.activeTabId = "web-1";
  layout.registerBeforeViewChange("dirty", (next) => next !== "browser");
  assert.equal(await browser.tabSwitch("web-1"), false);
  assert.equal(await browser.tabNew(), false);
  assert.equal(await actions.closeOthers("web:web-1"), false);
  assert.equal(layout.mainView, "files");
  assert.equal(browser.activeTabId, "web-1");
  assert.equal(browser.tabs.length, 1);
  assert.equal(calls.length, 0, "guard refusal must not reach native mutations");
}

{
  const { layout, browser, native } = fixture();
  layout.openModule("files");
  browser.tabs.push({ id: "a", url: "https://a.test" }, { id: "b", url: "https://b.test" });
  browser.activeTabId = "a";
  browser.url = "/working-directory";
  native.tabActivate = async () => { throw Error("native failure"); };
  await assert.rejects(browser.tabSwitch("b"), /native failure/);
  assert.equal(layout.mainView, "files", "failed IPC must not publish the browser view");
  assert.equal(browser.activeTabId, "a");
  assert.equal(browser.url, "/working-directory");
}

{
  const { layout, browser, native } = fixture();
  layout.openModule("files");
  browser.tabs.push({ id: "a", url: "https://a.test" }, { id: "b", url: "https://b.test" });
  browser.activeTabId = "a";
  let release;
  native.tabActivate = async (id) => { if (id === "b") await new Promise((resolve) => { release = resolve; }); };
  const pending = browser.tabSwitch("b");
  await Promise.resolve();
  layout.openModule("term");
  release();
  assert.equal(await pending, false);
  assert.equal(layout.mainView, "term", "an old native response cannot steal a newer module navigation");
  assert.equal(browser.activeTabId, "a");
}

{
  const { layout, browser } = fixture();
  layout.openModule("files");
  const remembered = layout.activeModTab;
  browser.tabs.push({ id: "a", url: "https://a.test" });
  browser.activeTabId = "a";
  browser.url = "/working-directory";
  assert.equal(await browser.tabClose("a"), true);
  assert.equal(layout.mainView, "files", "closing the last background web tab must preserve the module");
  assert.equal(layout.activeModTab, remembered);
  assert.equal(browser.url, "/working-directory");
  layout.activateBrowser();
  assert.equal(layout.closeModTab(remembered), true);
  assert.equal(layout.mainView, "browser", "closing a remembered module must not steal the browser view");
}

{
  const { layout, browser, actions, resources } = fixture();
  layout.openModule("files");
  const kept = layout.activeModTab;
  layout.openModule("term");
  // Deliberate ID collision across owners: the namespaced keys must stay distinct.
  browser.tabs.push({ id: kept, url: "https://a.test" }, { id: "b", url: "https://b.test" });
  browser.activeTabId = kept;
  assert.equal(await actions.closeOthers("web:" + kept), true);
  assert.deepEqual(resources().map((tab) => tab.key), ["web:" + kept]);
  assert.equal(layout.mainView, "browser");
}

{
  const { layout, browser, actions } = fixture();
  layout.openModule("files");
  const module = layout.activeModTab;
  browser.tabs.push({ id: "a", url: "https://a.test" }, { id: "b", url: "https://b.test" });
  browser.activeTabId = "b";
  layout.activateBrowser();
  assert.equal(await actions.cycle(1), true);
  assert.equal(layout.mainView, "files");
  assert.equal(layout.activeModTab, module);
  assert.equal(await actions.closeActive(), true);
  assert.equal(layout.modTabs.length, 0);
  assert.equal(browser.tabs.length, 2, "Ctrl+W on a module must not close its remembered web tab");
}

{
  const { layout } = fixture();
  assert.equal(layout.openDirTab("C:\\Work"), true);
  const id = layout.activeModTab;
  assert.equal(layout.openDirTab("c:/work"), true);
  assert.equal(layout.modTabs.length, 1, "Windows directory aliases share a logical tab");
  assert.equal(layout.activeModTab, id);
  assert.equal(layout.modTabs[0].label, "Work");
  const publish = layout.prepareDirTab("C:\\Other");
  assert.ok(publish);
  layout.openModule("term");
  assert.equal(publish(), false);
  assert.equal(layout.mainView, "term");
  assert.equal(layout.modTabs.some((tab) => tab.path === "C:\\Other"), false);
}

{
  let tabs = [{ key: "module:keep", active: true }, { key: "web:old", active: false }];
  const actions = createTabCoordinator({
    resources: () => tabs,
    activate: () => true,
    close: (tab) => {
      tabs = tabs.filter((item) => item.key !== tab.key);
      tabs.push({ key: "web:arrived-later", active: false });
      return true;
    },
  });
  assert.equal(await actions.closeOthers("module:keep"), true);
  assert.deepEqual(tabs.map((tab) => tab.key), ["module:keep", "web:arrived-later"], "close-others uses a bounded intent snapshot");
}

{
  const { layout } = fixture();
  layout.openDirTab("C:\\First");
  const first = layout.activeModTab;
  layout.openDirTab("C:\\Second");
  const second = layout.activeModTab;
  const prepared = layout.prepareModTabClose(second);
  assert.ok(prepared);
  assert.equal(prepared.next.id, first);
  assert.equal(prepared.publish.isCurrent(), true);
  assert.equal(layout.activeModTab, second, "preparing a close must not discard the current tab");
  assert.equal(layout.modTabs.length, 2, "a failed directory read can leave the uncommitted close untouched");
  layout.openModule("term");
  assert.equal(prepared.publish.isCurrent(), false);
  assert.equal(prepared.publish(), false, "a stale directory close cannot overwrite a newer navigation");
  assert.equal(layout.modTabs.some((tab) => tab.id === second), true);
}

{
  const { layout } = fixture();
  const first = layout.prepareViewChange("browser");
  const newer = layout.prepareDirTab("C:\\Queued");
  assert.ok(first && newer);
  assert.equal(first.isCurrent(), false, "an accepted newer intent wins before its async work completes");
  assert.equal(first(), false);
  assert.equal(newer(), true);
  assert.equal(layout.mainView, "files");
}

{
  const { layout } = fixture();
  layout.openDirTab("C:\\Work\\src");
  const id = layout.activeModTab;
  const oldIntent = layout.prepareModTabActivation(id);
  layout.relocateDirectoryTabs("c:/work", "D:\\Renamed");
  assert.equal(layout.modTabs[0].path, "D:\\Renamed\\src");
  assert.equal(layout.modTabs[0].label, "src");
  assert.equal(oldIntent.isCurrent(), false);
  assert.equal(oldIntent(), false, "a pre-rename navigation cannot restore an old path");
}

{
  const { layout } = fixture();
  assert.throws(() => layout.prepareDirTab(""), /DOCUMENT_PATH_INVALID/);
  assert.equal(layout.modTabs.length, 0, "an empty directory path must not create a logical tab");
  assert.equal(layout.mainView, "home");
}

{
  const { browser, native, lifecycle } = fixture();
  browser.tabs.push({ id: "closed", url: "https://closed.test" }, { id: "retry", url: "https://retry.test" });
  browser.activeTabId = "closed";
  native.tabClose = async (id) => { if (id === "retry") throw Error("native close unavailable"); };
  await assert.rejects(lifecycle.deactivate(), /BROWSER_CLEANUP_FAILED/);
  assert.deepEqual(Array.from(browser.tabs, tab => tab.id), ["retry"]);
  assert.equal(browser.activeTabId, "retry");
  native.tabClose = async () => {};
  await lifecycle.deactivate();
  assert.equal(browser.tabs.length, 0, "a failed cleanup remains retryable");
}
{
  const { layout, browser, native } = fixture();
  browser.tabs.push({ id: "placeholder", url: "about:blank" });
  browser.activeTabId = "placeholder";
  layout.activateBrowser();
  const placeholder = browser.activeTabId;
  layout.openDirTab("/work");
  assert.equal(await browser.consumeActiveBlankTab(placeholder), true,
    "a placeholder is consumed only after the directory has published");
  browser.tabs.push({ id: "changed", url: "https://occupied.test" });
  assert.equal(await browser.consumeActiveBlankTab("changed"), false,
    "an occupied page is never consumed as a stale placeholder");
}
{
  const { layout, browser, native } = fixture();
  browser.tabs.push({ id: "queued", url: "https://queued.test" });
  let activations = 0;
  native.tabActivate = async () => { activations++; };
  const switching = browser.tabSwitch("queued");
  layout.activateHome();
  assert.equal(await switching, false);
  assert.equal(activations, 0, "navigation is reserved before entering the native mutation queue");
  assert.equal(layout.mainView, "home");
}

{
  const { layout, browser, calls } = fixture();
  browser.tabs.push({ id: "current", url: "https://before.test", title: "Before" });
  browser.activeTabId = "current";
  layout.activateBrowser();
  browser.url = "https://after.test";
  assert.equal(await browser.navigateCurrent(), true);
  assert.equal(browser.tabs.length, 1, "omnibox navigation reuses the current native tab");
  assert.equal(browser.activeTabId, "current");
  assert.equal(browser.tabs[0].url, "https://after.test");
  assert.deepEqual(calls.filter(([kind]) => kind === "open"), [["open", "current", "https://after.test"]]);
  assert.equal(calls.some(([kind]) => kind === "new"), false);
}
{
  const { browser, calls } = fixture();
  browser.url = "https://first.test";
  assert.equal(await browser.navigateCurrent(), true);
  assert.equal(browser.tabs.length, 1);
  assert.deepEqual(calls.filter(([kind]) => kind === "new"), [["new", "https://first.test"]]);
  assert.equal(calls.some(([kind]) => kind === "open"), false, "no current tab means one explicit creation");
}
{
  const { layout, browser, calls } = fixture();
  browser.tabs.push({ id: "kept", url: "https://kept.test" });
  browser.activeTabId = "kept";
  layout.openModule("files");
  layout.registerBeforeViewChange("dirty", next => next !== "browser");
  browser.url = "https://refused.test";
  assert.equal(await browser.navigateCurrent(), false);
  assert.equal(calls.length, 0);
  assert.equal(browser.tabs[0].url, "https://kept.test");
  assert.equal(layout.mainView, "files");
}
{
  const { layout, browser, calls } = fixture();
  browser.tabs.push({ id: "queued-address", url: "https://kept.test" });
  browser.activeTabId = "queued-address";
  layout.activateBrowser();
  browser.url = "https://stale.test";
  const navigating = browser.navigateCurrent();
  layout.activateHome();
  assert.equal(await navigating, false);
  assert.equal(layout.mainView, "home", "a queued old address cannot take over a newer navigation");
  assert.equal(calls.some(([kind]) => kind === "open" || kind === "new"), false);
}
{
  const { browser, native, calls } = fixture();
  browser.tabs.push({ id: "closing-address", url: "https://kept.test" });
  browser.activeTabId = "closing-address";
  browser.url = "https://stale.test";
  let release;
  native.tabClose = () => new Promise(resolve => { release = resolve; });
  const closing = browser.tabClose("closing-address");
  await Promise.resolve();
  const navigating = browser.navigateCurrent();
  release();
  await closing;
  assert.equal(await navigating, false);
  assert.equal(browser.tabs.length, 0);
  assert.equal(calls.some(([kind]) => kind === "open" || kind === "new"), false,
    "a tab removed while an address is queued must not be recreated or retargeted");
}
{
  const { layout, browser, native } = fixture();
  browser.tabs.push({ id: "failed-open", url: "https://kept.test", title: "Kept" });
  browser.activeTabId = "failed-open";
  layout.openModule("files");
  browser.url = "https://failed.test";
  native.tabOpen = async () => { throw Error("native open failed"); };
  await assert.rejects(browser.navigateCurrent(), /native open failed/);
  assert.equal(browser.tabs[0].url, "https://kept.test");
  assert.equal(browser.tabs[0].title, "Kept");
  assert.equal(layout.mainView, "files", "failed IPC cannot publish a browser navigation");
}
{
  const { layout, browser, calls } = fixture();
  browser.tabs.push({ id: "raced-blank", url: "about:blank" });
  browser.activeTabId = "raced-blank";
  layout.activateBrowser();
  const replacing = browser.consumeActiveBlankTab("raced-blank");
  browser.tabs[0].url = "https://occupied.test";
  assert.equal(await replacing, false);
  assert.equal(browser.tabs.length, 1);
  assert.equal(calls.some(([kind]) => kind === "close"), false,
    "the blank-only condition is rechecked at the queued native operation");
}
{
  const { layout, browser, calls, navigation } = fixture();
  browser.tabs.push({ id: "denied-module", url: "about:blank" });
  browser.activeTabId = "denied-module";
  layout.activateBrowser();
  layout.registerBeforeViewChange("deny-grid", next => next !== "grid");
  await navigation.openModuleInCurrentTab("grid");
  assert.equal(layout.mainView, "browser");
  assert.equal(layout.modTabs.length, 0);
  assert.equal(calls.length, 0, "a rejected module intent never disposes its blank browser tab");
}
{
  const { layout, browser, native, navigation } = fixture();
  browser.tabs.push({ id: "slow-placeholder", url: "about:blank" });
  browser.activeTabId = "slow-placeholder";
  layout.activateBrowser();
  let release;
  native.tabClose = () => new Promise(resolve => { release = resolve; });
  const opening = navigation.openModuleInCurrentTab("grid");
  await Promise.resolve();
  layout.activateHome();
  release();
  await opening;
  assert.equal(layout.mainView, "home", "a late placeholder close cannot resurrect its old module request");
  assert.equal(layout.modTabs.length, 0);
}
{
  const { layout, browser, native, navigation } = fixture();
  browser.tabs.push({ id: "failed-placeholder", url: "about:blank" });
  browser.activeTabId = "failed-placeholder";
  layout.activateBrowser();
  native.tabClose = async () => { throw Error("native close failed"); };
  await assert.rejects(navigation.openModuleInCurrentTab("grid"), /native close failed/);
  assert.equal(layout.mainView, "browser");
  assert.equal(layout.modTabs.length, 0);
  assert.equal(browser.tabs.length, 1, "a failed native close keeps its owner record retryable");
}
{
  const { layout, browser, calls, navigation } = fixture();
  browser.tabs.push({ id: "replace-once", url: "about:blank" });
  browser.activeTabId = "replace-once";
  layout.activateBrowser();
  let approvals = 0;
  layout.registerBeforeViewChange("count", () => { approvals++; return true; });
  await navigation.openModuleInCurrentTab("grid");
  assert.equal(layout.mainView, "grid");
  assert.equal(layout.modTabs.length, 1);
  assert.equal(browser.tabs.length, 0);
  assert.equal(approvals, 1, "the async module route preserves one guarded owner intent");
  assert.deepEqual(calls.filter(([kind]) => kind === "close"), [["close", "replace-once"]]);
}

for (const deactivate of [false, true]) {
  const { layout, browser, native, lifecycle, availability } = fixture();
  browser.tabs.push({ id: "suspending-open", url: "https://before.test" });
  browser.activeTabId = "suspending-open";
  layout.openModule("files");
  browser.url = "https://pending.test";
  let release;
  native.tabOpen = () => new Promise(resolve => { release = resolve; });
  const opening = browser.navigateCurrent();
  await Promise.resolve();
  availability.browser = false;
  lifecycle.suspend();
  const releasing = deactivate ? lifecycle.deactivate() : Promise.resolve();
  release();
  assert.equal(await opening, false, "resource revocation during IPC cancels navigation publication");
  await releasing;
  assert.equal(layout.mainView, "files", "suspension and cleanup preserve the unrelated visible module");
  assert.equal(browser.tabs.length, deactivate ? 0 : 1, "only successful owner cleanup removes the native tab record");
}
console.log("UNIFIED_TAB_ACTIONS=PASS");
