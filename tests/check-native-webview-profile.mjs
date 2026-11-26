import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const native = readFileSync(new URL('../tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/commands.rs', import.meta.url),'utf8');
const login = readFileSync(new URL('../src/capabilities/bfc/model/readbackScripts.mjs', import.meta.url),'utf8');
assert.match(native,/\.path\(\)\s*\.app_data_dir\(\)/);
assert.match(native,/app_data_dir\.join\("browser-webview-profile"\)/);
assert.match(native,/builder\s*=\s*builder\.data_directory\(profile\)/);
assert.match(native,/std::fs::Permissions::from_mode\(0o700\)/);
assert.match(native,/canonical_profile != canonical_base\.join\("browser-webview-profile"\)/);
assert.doesNotMatch(native,/get_cookies|cookies\s*\(\)|cookie_manager\s*\.|document\.cookie|localStorage\./i);
assert.match(login,/\[class\*="login"\]/);
assert.match(login,/loginControlSeen/);
// Native host membership and actual directory identity are separate from the
// browser engine's own session data. Retain the original compatibility checks.
const owner = readFileSync(new URL('../src-tauri/src/browser_profile_owner.rs', import.meta.url),'utf8');
const startup = readFileSync(new URL('../src-tauri/src/startup.rs', import.meta.url),'utf8');
const grid = readFileSync(new URL('../src-tauri/src/grid_process.rs', import.meta.url),'utf8');
const gridWindows = readFileSync(new URL('../src-tauri/src/grid_process_windows.rs', import.meta.url),'utf8');
assert.match(native,/struct PreparedLinuxProfile/);
assert.match(native,/let profile = self\.prepared_browser_profile\(\)\?/);
assert.match(native,/prepared\.check_root\(\)\?/);
assert.match(owner,/static HOST_PROFILE: OnceLock<ProfileLease>/);
assert.match(owner,/try_lock\(directory, MEMBERS, true\)/);
assert.match(owner,/try_lock\(directory, MEMBERS, false\)/);
assert.match(owner,/BROWSER_PROFILE_PREVIOUS_HOSTS_ACTIVE/);
assert.match(owner,/BROWSER_PROFILE_STALE_EPOCH/);
assert.match(owner,/BROWSER_PROFILE_ROOT_IDENTITY_CHANGED/);
assert.match(owner,/directory_identity\(path\)/);
assert.match(owner,/HOST_PROFILE_BOUND\.get\(\)\.is_some\(\)/);
assert.match(startup,/browser_profile_owner::prepare\([\s\S]*?tabs\.prepare_storage\(\)[\s\S]*?browser_profile_owner::bind_prepared\(/);
for (const platform of [grid,gridWindows]) {
  assert.match(platform,/browser_profile_owner::grid_epoch\(\)\?/);
  assert.match(platform,/\.env\(crate::browser_profile_owner::GRID_EPOCH_ENV, profile_epoch\)/);
}
console.log('BROWSER_NATIVE_PROFILE_CONTRACT=PASS (stable shared profile, native host epoch and root identity, no credential exports)');
