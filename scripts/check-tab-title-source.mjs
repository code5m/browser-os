#!/usr/bin/env node
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
const rust=readFileSync("src-tauri/src/bridge/resource_commands.rs","utf8");
const browser=readFileSync("src/capabilities/browser/state/useBrowserStore.ts","utf8");
const app=readFileSync("src/App.vue","utf8");
assert.match(rust,/let id = webview\.label\(\)\.to_string\(\)/);
assert.match(rust,/let page_url = webview\.url\(\)/);
assert.doesNotMatch(rust,/fn report_title_inner[\s\S]{0,350}active_tab\.lock/);
assert.match(rust,/"tab-title",[\s\S]*?"url": page_url/);
// Native events are invalidations, not authoritative metadata publications.
assert.match(browser,/function setTitle\(t: TabInfo\)\s*\{\s*if \(t && typeof t\.id === "string"\) queueNativeTabMetadata\(t\.id\);\s*\}/);
assert.match(browser,/function setNavigated\(id: string, _navUrl: string, applied\?: MetadataApplied\)\s*\{\s*queueNativeTabMetadata\(id, applied\);\s*\}/);
assert.match(browser,/if \(typeof id !== "string" \|\| !tabs\.some\(tab => tab\.id === id\)\) return;/);
assert.match(browser,/const snapshot = await bridge\.tabList\(\);[\s\S]*?if \(epoch !== metadataEpoch \|\| runningMutation/);
assert.match(browser,/!tabs\.some\(item => item === tab\) \|\| metadataDirty\.has\(row\.id\)[\s\S]*?!native\.url\.trim\(\)[\s\S]*?continue;/);
assert.match(browser,/tab\.url = native\.url;\s*tab\.title = native\.title \|\| native\.url;/);
assert.match(browser,/current\.title = target/);
assert.match(browser,/t\.title = t\.url/);
assert.match(app,/onTabTitle\(\(t\) => browser\.setTitle\(t\)\)/);
console.log("TAB_TITLE_SOURCE_AND_NAVIGATION=PASS");
