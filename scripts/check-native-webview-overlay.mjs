#!/usr/bin/env node
// Native browser tabs are child WebViews on Linux. HTML z-index cannot place a
// toast above them, so global messages must stay in the status bar instead.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { reactive, watch, nextTick } from "vue";

const status = readFileSync("src/components/layout/StatusBar.vue", "utf8");
assert.match(status, /<span v-if="layout\.msg" class="msg">/);
assert.doesNotMatch(status, /toast-pop/);
assert.doesNotMatch(status, /position:\s*fixed;\s*top:/);
const app = readFileSync("src/App.vue", "utf8");
const browser = readFileSync("src/stores/useBrowserStore.ts", "utf8");
const host = readFileSync("src/composables/useBrowserHost.ts", "utf8");
const css = readFileSync("src/styles/global.css", "utf8");
assert.match(app, /watch\(\(\) => session\.closeDialogOpen/);
assert.match(app, /layout\.webviewsSuspended = open/);
assert.match(app, /await bridge\.hideAllWebviews\(\)/);
assert.match(app, /browser\.forceGridRelayout\(\)/);
assert.match(host, /if \(layout\.webviewsSuspended\) return;/);
assert.doesNotMatch(browser, /showToast\([^\n]*已新建页签/);
assert.match(css, /html, body, #app\s*\{[^}]*overflow:\s*hidden/);

// Execute the actual App watcher with a deferred native hide completion.
const watcher = app.match(/watch\(\(\) => session\.closeDialogOpen,[\s\S]*?\}, \{ flush: "sync" \}\);/)[0];
for (const mainView of ["browser", "grid"]) {
  const session = reactive({ closeDialogOpen: false });
  const layout = reactive({ webviewsSuspended: false, mainView });
  const calls = [];
  let finishHide;
  const bridge = { hideAllWebviews() {
    calls.push("hide");
    return new Promise(resolve => { finishHide = resolve; });
  } };
  const browser = {
    relocate() { calls.push("restore-browser"); },
    forceGridRelayout() { calls.push("restore-grid"); },
  };
  const stop = new Function("watch", "nextTick", "session", "layout", "bridge", "browser", `return ${watcher}`)(watch, nextTick, session, layout, bridge, browser);
  session.closeDialogOpen = true;
  assert.equal(layout.webviewsSuspended, true);
  assert.deepEqual(calls, ["hide"]);
  session.closeDialogOpen = false;
  await nextTick();
  assert.equal(layout.webviewsSuspended, false);
  assert.equal(calls.at(-1), `restore-${mainView}`);
  finishHide();
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(calls.at(-1), `restore-${mainView}`);
  assert.equal(calls.length, 3, "late hide must be followed by restoration");
  stop();
}
console.log("NATIVE_WEBVIEW_OVERLAY_CHECK=PASS (watcher races + source guards; native desktop QA separate)");
