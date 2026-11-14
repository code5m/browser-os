import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

// Static source-contract smoke checks only. They cannot establish Windows
// compilation, WebView2 reliability, native filesystem safety or GUI PASS.
const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');
const cas = read('../src-tauri/src/capabilities/workspace/cas.rs');
const moves = read('../src-tauri/src/capabilities/workspace/fs_cmds.rs');
const session = read('../src-tauri/src/session.rs');
const tabs = read('../tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/commands.rs');

assert.match(cas, /FILE_SAVE_PLATFORM_UNSUPPORTED/);
assert.doesNotMatch(cas, /save_if_unchanged_path\s*\(/);
assert.match(cas, /non_linux_save_is_denied_without_mutating_the_file/);
assert.match(moves, /FILE_MOVE_PLATFORM_UNSUPPORTED/);
assert.match(session, /\.create_new\(true\)/);
assert.match(session, /\.sync_all\(\)/);
assert.match(session, /\.json\.tmp/);
assert.match(tabs, /PageLoadEvent::Started/);
assert.match(tabs, /PageLoadEvent::Finished/);
assert.match(tabs, /browser_profile_path/);
assert.match(tabs, /#\[cfg\(target_os = "windows"\)\]\s*fn ensure_private_browser_profile/);
assert.match(tabs, /builder\.data_directory\(profile\)/);
assert.match(tabs, /GRID_PLATFORM_UNSUPPORTED/, 'script result remains blocked until native Windows POC');
assert.doesNotMatch(tabs, /on_navigation\(move \|next_url\|/);

console.log('WINDOWS_LINUX_V2_SOURCE_CONTRACT=PASS (static-only, Windows GUI pending)');
