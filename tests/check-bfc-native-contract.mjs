import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');
const main = read('../src-tauri/src/main.rs');
const defaults = read('../src-tauri/permissions/default-commands.toml');
const remote = read('../src-tauri/permissions/remote-collect.toml');
const bfcAcl = read('../src-tauri/permissions/bfc-readback.toml');
const bfcCapability = JSON.parse(read('../src-tauri/capabilities/bfc-readback.json'));
const registry = read('../docs/architecture/native-boundary/native-commands.yaml');
const commands = read('../src-tauri/src/capabilities/bfc/commands.rs');
const runtime = read('../src-tauri/src/capabilities/bfc/runtime.rs');
const store = read('../src/capabilities/bfc/state/useBfcStore.ts');
const panel = read('../src/capabilities/bfc/ui/BfcProjectPanel.vue');

const mainCommands = [
  'bfc_readback_begin',
  'bfc_readback_status',
  'bfc_readback_cancel',
  'bfc_readback_finish',
  'bfc_service_status',
];
for (const name of mainCommands) {
  assert.ok(main.includes(`capabilities::bfc::commands::${name}`), `main handler missing ${name}`);
  assert.ok(defaults.includes(`"${name}"`), `main permission missing ${name}`);
  assert.ok(registry.includes(`  ${name}:`), `native registry missing ${name}`);
}
assert.ok(main.includes('capabilities::bfc::commands::bfc_readback_submit'));
assert.doesNotMatch(remote, /"bfc_readback_submit"/);
assert.match(bfcAcl, /commands.allow = \["bfc_readback_submit"\]/);
assert.deepEqual(bfcCapability.webviews, ["tab-*"]);
assert.deepEqual(bfcCapability.windows, ["main"]);
assert.deepEqual(bfcCapability.remote.urls, ["https://www.bilibili.com/*","https://space.bilibili.com/*"]);
assert.deepEqual(bfcCapability.permissions, ["bfc-readback"]);
assert.doesNotMatch(defaults, /"bfc_readback_submit"/);
assert.ok(registry.includes('  bfc_readback_submit:'));

assert.match(commands, /http:\/\/127\.0\.0\.1:4198/);
assert.match(commands, /\.no_proxy\(\)/);
assert.match(commands, /redirect\(reqwest::redirect::Policy::none\(\)\)/);
assert.match(commands, /returned_scan != Some\(payload\.scan_id\.as_str\(\)\)/);
assert.match(commands, /MAX_REMOTE_READBACK_BYTES/);
assert.match(runtime, /session\.tab_id != tab_id/);
assert.match(runtime, /url\.host_str\(\)/);
assert.match(runtime, /BFC_BILIBILI_HTTP_412/);
assert.match(runtime, /global_ids\.insert/);
assert.match(runtime, /SESSION_TTL/);
// The optional original-folder plan needs provenance, but not an account
// token or write-capable Tauri command.
assert.match(runtime, /serde\(rename = "folderId"\)/);
assert.match(runtime, /serde\(rename = "folderName"\)/);
assert.match(runtime, /folder_id: capture\.folder_id\.clone\(\)/);
assert.match(runtime, /folder_name: capture\.folder_name\.clone\(\)/);
assert.doesNotMatch(commands, /x\/v3\/fav\/resource\/(?:copy|move)/);
assert.doesNotMatch(runtime, /x\/v3\/fav\/resource\/(?:copy|move)/);
assert.match(runtime, /BFC_IMPORT_CANCEL_UNSAFE/);
assert.match(commands, /state::<BfcRuntime>\(\)\.cancel\(&ticket\)/);
assert.match(store, /JSON\.stringify\(currentTicket\)/);
assert.match(store, /if \(cancelRequested\.value\) return;/);
assert.match(store, /service\.local_only !== true/);
assert.match(store, /service\.read_snapshot_only !== true/);
assert.match(store, /complete_snapshot === false/);
assert.match(panel, /complete_snapshot === false/);
assert.match(panel, /不读取 Cookie/);
assert.doesNotMatch(commands, /api\.bilibili\.com|passport\.bilibili\.com/);

console.log('BFC_NATIVE_CONTRACT=PASS (ACL split, tab-bound ticket, loopback-only import, incomplete snapshot warning)');
