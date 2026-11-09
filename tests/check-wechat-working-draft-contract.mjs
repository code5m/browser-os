import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');
const native = read('../src-tauri/src/capabilities/workspace/weixin_drafts.rs');
const main = read('../src-tauri/src/main.rs');
const acl = read('../src-tauri/permissions/default-commands.toml');
const bridge = read('../src/bridge.ts');
const ui = read('../src/components/system/WechatDraftPanel.vue');

for (const name of ['wechat_working_draft_load', 'wechat_working_draft_save', 'wechat_working_draft_clear']) {
  assert.ok(main.includes('weixin_drafts::' + name), name + ' must be registered');
  assert.ok(acl.includes('"' + name + '"'), name + ' must be main-window-only');
  assert.ok(native.includes('fn ' + name + '('), name + ' must have a native implementation');
}
assert.match(native, /KeyringStore::save_token\(WORKING_DRAFT_KEY/);
assert.match(native, /KeyringStore::get_token_result\(WORKING_DRAFT_KEY/);
assert.match(native, /KeyringStore::delete_token\(WORKING_DRAFT_KEY\)/);
assert.match(native, /validate_working_draft\(&draft\)/);
assert.match(native, /MAX_WORKING_DRAFT_BYTES/);
assert.doesNotMatch(ui, /localStorage\.|sessionStorage\./);
assert.match(bridge, /wechatWorkingDraftLoad/);
assert.match(bridge, /wechatWorkingDraftSave/);
assert.match(ui, /await saveBeforeDraftAdd\(\)/);
const latch = ui.indexOf('outcomeUnknown.value = true;', ui.indexOf('function createDraft'));
const before = ui.indexOf('await saveBeforeDraftAdd();', latch);
const request = ui.indexOf('await bridge.wechatCreateDraft(', before);
assert.ok(latch > 0 && before > latch && request > before, 'durable duplicate guard must precede draft/add');

const clearBlock = ui.slice(ui.indexOf('function clearWorkingDraft()'), ui.indexOf('onMounted(() =>'));
assert.ok(clearBlock.indexOf('await saveQueue;') < clearBlock.indexOf('await bridge.wechatWorkingDraftClear()'),
  'clear must drain queued saves before keyring deletion');
assert.ok(clearBlock.includes('await nextTick();'), 'reactive clearing must flush watchers while persistence is suppressed');
assert.match(ui, /if \(!draftStorageAvailable\.value\)/);
console.log('WECHAT_WORKING_DRAFT_CONTRACT=PASS (keyring-only, durable no-blind-retry, safe clear ordering)');
