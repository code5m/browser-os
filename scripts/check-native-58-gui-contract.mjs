#!/usr/bin/env node
// This gate proves the installed-GUI driver covers the exact frozen 58-command
// set and cannot silently replace real installed IPC with a mocked fast PASS.
// It does NOT substitute for the installed GUI run's actual side-effect proof.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
const ROOT=resolve(fileURLToPath(new URL('../',import.meta.url)));
const source=(path)=>readFileSync(resolve(ROOT,path),'utf8');
const debt=JSON.parse(source('docs/architecture/native-boundary/native-permission-debt-baseline.json')).pending_side_effect_reviews;
const js=source('scripts/native-58-gui-review.js');
const driver=source('scripts/native-58-gui-regression.py');
const rust=source('src-tauri/src/native_58_gui_review.rs');
const main=source('src-tauri/src/main.rs');
const workflow=source('.github/workflows/capability-v2-validation.yml');
const block=/const expected=\[([\s\S]*?)\];/.exec(js);
assert.ok(block,'Native 58 JS driver must have an exact expected-command ledger');
const expected=Array.from(block[1].matchAll(/'([a-z][a-z0-9_]*)'/g),m=>m[1]);
assert.equal(debt.length,58,'legacy debt must contain exactly 58 distinct commands');
assert.equal(new Set(debt).size,58);
assert.deepEqual([...expected].sort(),[...debt].sort(),'driver expected list must match the governed Native 58 inventory');
assert.equal(new Set(expected).size,58);
const invoked=Array.from(js.matchAll(/\brun\('([a-z][a-z0-9_]*)'\s*,/g),m=>m[1]);
const eventDriven=Array.from(js.matchAll(/\brecord\('(report_title|report_resources)'\s*,/g),m=>m[1]);
const actual=[...invoked,...eventDriven];
assert.deepEqual(actual.slice().sort(),debt.slice().sort(),'every declared Native 58 command needs one real invocation or remote WebView event observation');
assert.equal(new Set(actual).size,58,'duplicate/missing Native 58 outcomes are not permitted');
assert.match(js,/const invoke = tauri\?\.core\?\.invoke;/,'must use the actual installed Tauri IPC');
assert.match(js,/tauri\.event\.emit\('native-58-review-finished'/,'must emit real completed command outcomes');
assert.match(js,/resourceRecordsBeforeClear/,'must include before/after WebKit resource evidence');
assert.match(rust,/items\.len\(\) == 58/,'native must verify exact 58 results');
assert.match(rust,/nativePostconditionsStatus/,'native side effects require postcondition proof');
assert.match(main,/NATIVE_58_GUI_REVIEW/,'installed app must provide an opt-in test entry');
assert.match(driver,/tempfile\.TemporaryDirectory/,'test must isolate user home and XDG writes');
assert.match(driver,/"XDG_DATA_HOME"/);
assert.match(driver,/"NATIVE_58_XDG_LOG"/);
assert.match(driver,/"DISPLAY"/);
assert.match(driver,/Xvfb\\s/);
assert.match(driver,/eval_event/,'WebView JS side-effect must be checked by loopback server');
assert.match(driver,/remote_origin_allow_and_deny/,'remote allow/deny results must be checked');
assert.match(driver,/authorizationDenied/,'remote rejection must prove actual ACL denial');
assert.match(workflow,/Native 58 side-effect review with the SAME installed Release/);
assert.match(workflow,/native-58-gui-regression\.py --binary \/usr\/bin\/mvp-browser-os/);
console.log('NATIVE_58_GUI_CONTRACT=PASS baseline=58 real-ipc=56 remote-event=2 native-postconditions=required installed-release=required');
