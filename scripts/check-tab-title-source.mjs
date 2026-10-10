#!/usr/bin/env node
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
const rust=readFileSync("src-tauri/src/bridge.rs","utf8");
const browser=readFileSync("src/capabilities/browser/state/useBrowserStore.ts","utf8");
const app=readFileSync("src/App.vue","utf8");
assert.match(rust,/let id = webview\.label\(\)\.to_string\(\)/);
assert.match(rust,/let page_url = webview\.url\(\)/);
assert.doesNotMatch(rust,/fn report_title_inner[\s\S]{0,350}active_tab\.lock/);
assert.match(rust,/"tab-title",[\s\S]*?"url": page_url/);
assert.match(browser,/canonical\(existing\.url\) !== canonical\(t\.url\)/);
assert.match(browser,/existing\.title = t\.title/);
assert.match(browser,/t\.title = navUrl/);
assert.match(browser,/current\.title = target/);
assert.match(browser,/t\.title = t\.url/);
assert.match(app,/onTabTitle\(\(t\) => browser\.setTitle\(t\)\)/);
console.log("TAB_TITLE_SOURCE_AND_NAVIGATION=PASS");
