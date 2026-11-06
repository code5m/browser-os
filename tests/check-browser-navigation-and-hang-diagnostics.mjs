import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const src = (name) => readFileSync(new URL('../' + name, import.meta.url), 'utf8');
const plugin = src('tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/commands.rs');
const main = src('src-tauri/src/main.rs');
const browser = src('src/capabilities/browser/state/useBrowserStore.ts');
const bar = src('src/components/layout/ActivityBar.vue');
const app = src('src/App.vue');
const shellDiagnostics = src('src/application/shellDiagnostics.ts');
const bridge = src('src/bridge.ts');
const watchdog = src('src-tauri/src/hang_diagnostics.rs');
const gridManager = src('src-tauri/src/grid_process.rs');
const codingOwner = src('src-tauri/src/capabilities/agent/execution_owner.rs');
const aiBackend = src('src-tauri/src/framework/ai.rs');
const collector = src('scripts/collect-browseros-hang.sh');

// GTK URI is a top-level document property. The general on_navigation
// callback is invoked for iframe srcdoc/about:blank as well.
assert.match(plugin, /connect_uri_notify\(move \|wv\|/);
assert.match(plugin, /wv\.uri\(\)/);
assert.match(plugin, /let last_uri = std::cell::RefCell::new\(String::new\(\)\)/);
assert.match(plugin, /#\[cfg\(not\(target_os = "linux"\)\)\]/);
assert.match(plugin, /#\[cfg\(target_os = "linux"\)\]\s*let _ = next_url;/);
assert.match(plugin, /BrowserTabEvent::NavigationFinished/);

// Frontend must not apply late events for a closed tab, and omnibox must
// release stale focus-time display state on blur.
assert.match(browser, /if \(!t \|\| !navUrl\.trim\(\)\) return;/);
assert.match(bar, /function blurAddress\(\)\s*\{[\s\S]{0,320}addressText\.value=contextualAddress\.value;/);
assert.match(main, /\[navigation\] main_forward seq=/);
assert.match(shellDiagnostics, /\[navigation\] frontend_applied seq=/);
assert.match(app, /recordNavigationApplied\(/);
assert.doesNotMatch(main, /emit tab-navigated id=\{\} url=\{\}/);

// A stuck GUI must not accumulate unbounded callbacks; the watchdog runs on
// an independent thread and logs only process and thread metadata.
assert.match(main, /hang_diagnostics::start\(app\.handle\(\)\.clone\(\)\)/);
assert.match(watchdog, /pending_callback\.swap\(true, Ordering::SeqCst\)/);
assert.match(watchdog, /gtk_unresponsive_suspected/);
assert.match(watchdog, /gtk_responsive_again/);
assert.match(watchdog, /DIAGNOSTIC_MAX_BYTES: u64 = 1024 \* 1024/);
assert.doesNotMatch(watchdog, /document\.cookie|localStorage|html_body|request_body/);
assert.match(shellDiagnostics, /\[health\] shell-pulse unix_ms=/);
assert.match(app, /recordShellHeartbeat\(/);
assert.match(bridge, /\[ipc\] command=/);
assert.match(bridge, /command === "debug_log"/);

// Independent collector is read-only, excludes WebKit cache and redacts
// source session URLs before creating a user-reviewable local support archive.
assert.match(collector, /umask 077/);
assert.match(collector, /session-\*\.log/);
assert.match(collector, /\[URL\]/);
assert.match(collector, /No running BrowserOS process was stopped/);
assert.doesNotMatch(collector, /kill -|killall|pkill|gdb -p|strace -p|rm -rf/);
// Native event callbacks must only enqueue Grid positioning. The worker
// coalesces fast GTK events and replays the newest generation after slow I/O.
assert.match(main, /WindowEvent::Moved\(_\) \| WindowEvent::Resized\(_\)[\s\S]{0,250}request_reposition_async\(\)/);
assert.match(gridManager, /fn request_reposition_async\(&self\)/);
assert.match(gridManager, /fn finish_reposition_iteration\(&self, observed: u64\)/);
assert.match(gridManager, /grid_reposition_background_slow/);
// Graceful exit is a single background cleanup transaction, not a GTK wait.
assert.match(main, /WindowEvent::CloseRequested \{ api, \.\. \}[\s\S]{0,330}api\.prevent_close\(\)/);
assert.match(main, /RunEvent::ExitRequested \{ api, \.\. \}[\s\S]{0,250}api\.prevent_exit\(\)/);
assert.match(main, /\.name\("browseros-shutdown"\.into\(\)\)/);
assert.match(main, /fn begin_graceful_shutdown\(/);
// Existing coding work binds to the actual session identity, not a random UUID.
assert.match(codingOwner, /fn coding_scope\(root: &Path, session_id: Option<&str>\)/);
assert.match(codingOwner, /Self::begin_scoped\("coding", workspace, session, vec!\[\]\)/);
assert.match(aiBackend, /ProductionExecution::begin_coding\(/);
console.log('BROWSER_NAVIGATION_HANG_DIAGNOSTICS_CONTRACT=PASS');
