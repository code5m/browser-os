#!/usr/bin/env node
// Native browser tabs are child WebViews on Linux. HTML z-index cannot place a
// toast above them, so global messages must stay in the status bar instead of
// floating overlays.
//
// Owner 最终裁决（2026-09-12）：普通 Tab 关闭不再弹确认框，原 SessionCloseDialog 已
// 撤销，不再有"关闭确认期间暂停 WebView / 遮罩"的 watcher。本检查只守住通用边界：
// 状态栏承载消息、无覆盖原生网页的浮层、成功操作不显示 Toast、文档根不滚动。
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const status = readFileSync("src/components/layout/StatusBar.vue", "utf8");
assert.match(status, /<span v-if="layout\.msg" class="msg">/);
assert.doesNotMatch(status, /toast-pop/);
assert.doesNotMatch(status, /position:\s*fixed;\s*top:/);

const app = readFileSync("src/App.vue", "utf8");
const browser = readFileSync("src/stores/useBrowserStore.ts", "utf8");
const host = readFileSync("src/composables/useBrowserHost.ts", "utf8");
const css = readFileSync("src/styles/global.css", "utf8");

// 成功操作不显示 Toast（不浮层盖住原生网页）；普通关闭不得出现成功 Toast
assert.doesNotMatch(browser, /showToast\([^\n]*已新建页签/);
assert.doesNotMatch(browser, /showToast\([^\n]*已关闭页签/);
// 普通关闭不再有任何"关闭确认遮罩"组件（SessionCloseDialog 已撤销）
assert.doesNotMatch(app, /SessionCloseDialog/);

assert.match(css, /html, body, #app\s*\{[^}]*overflow:\s*hidden/);

// WebView 可见性仍受保护（modal 期间不被 JS 误隐藏）
assert.match(host, /if \(layout\.webviewsSuspended\) return;/);

console.log("NATIVE_WEBVIEW_OVERLAY_CHECK=PASS (overlay guards hold; native desktop QA separate)");
