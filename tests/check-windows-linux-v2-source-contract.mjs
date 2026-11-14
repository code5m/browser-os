import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

// Static source-contract smoke checks only. They cannot establish Windows
// compilation, WebView2 reliability, native filesystem safety or GUI PASS.
const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');
const cas = read('../src-tauri/src/capabilities/workspace/cas.rs');
const moves = read('../src-tauri/src/capabilities/workspace/fs_cmds.rs');
const session = read('../src-tauri/src/session.rs');
const tabs = read('../tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/commands.rs');
const cargo = read('../src-tauri/Cargo.toml');
const lock = read('../src-tauri/Cargo.lock');
const process = read('../src-tauri/src/windows_process.rs');
const terminal = read('../src-tauri/src/capabilities/terminal/terminal.rs');
const windowsPipe = read('../src-tauri/src/windows_pipe.rs');
const gridIpc = read('../src-tauri/src/grid_ipc.rs');

assert.match(cargo, /keyring = \{ version = "3", features = \["sync-secret-service", "windows-native"\] \}/);
assert.match(lock, /name = "keyring"[\s\S]*?"windows-sys 0\.60\.2"/);
assert.match(cargo, /Win32_System_JobObjects/);
assert.match(process, /JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE/);
assert.match(windowsPipe, /PIPE_REJECT_REMOTE_CLIENTS/);
assert.match(windowsPipe, /GetNamedPipeClientProcessId/);
assert.match(gridIpc, /MAX_GRID_WIRE_BYTES/);
assert.match(process, /AssignProcessToJobObject/);
assert.match(terminal, /WINDOWS_TERMINAL_JOB_FAILED/);
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
assert.match(tabs, /webview\.eval_with_callback\(js, move \|raw_json\|/);
assert.match(tabs, /decode_native_eval_result\(&raw_json\)/);
assert.doesNotMatch(tabs, /GRID_PLATFORM_UNSUPPORTED/);
assert.doesNotMatch(tabs, /on_navigation\(move \|next_url\|/);

console.log('WINDOWS_LINUX_V2_SOURCE_CONTRACT=PASS (static-only, Windows GUI pending)');
