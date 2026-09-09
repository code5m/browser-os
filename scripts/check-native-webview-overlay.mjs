#!/usr/bin/env node
// Native browser tabs are child WebViews on Linux. HTML z-index cannot place a
// toast above them, so global messages must stay in the status bar instead.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const status = readFileSync("src/components/layout/StatusBar.vue", "utf8");
assert.match(status, /<span v-if="layout\.msg" class="msg">/);
assert.doesNotMatch(status, /toast-pop/);
assert.doesNotMatch(status, /position:\s*fixed;\s*top:/);
console.log("NATIVE_WEBVIEW_OVERLAY_CHECK=PASS");
