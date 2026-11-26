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
const layout = src('src-tauri/src/bridge/browser_commands.rs');
const crashlog = src('src-tauri/src/crashlog.rs');
const recovery = src('src-tauri/src/bridge.rs');
const gridManager = src('src-tauri/src/grid_process.rs');
const codingOwner = src('src-tauri/src/capabilities/agent/execution_owner.rs');
const aiBackend = src('src-tauri/src/framework/ai.rs');
const codingFacade = src('src-tauri/src/capabilities/agent/public_api.rs');
const coding = src('src-tauri/src/capabilities/agent/coding.rs');
const collector = src('scripts/collect-browseros-hang.sh');

// Linux keeps the proven WebKit main-frame URI observer. Windows uses
// Tauri's page-load lifecycle; the navigation permission callback is not a
// completion event and must never be used to fabricate NavigationFinished.
assert.match(plugin, /#\[cfg\(target_os = "linux"\)\][\s\S]{0,1800}connect_uri_notify\(move \|wv\|/);
assert.match(plugin, /wv\.uri\(\)/);
assert.match(plugin, /let last_uri = std::cell::RefCell::new\(String::new\(\)\)/);
assert.match(plugin, /#\[cfg\(not\(target_os = "linux"\)\)\][\s\S]{0,900}on_page_load\(move \|_webview, payload\|/);
assert.match(plugin, /PageLoadEvent::Started\s*=>\s*BrowserTabEvent::NavigationStarted/);
assert.match(plugin, /PageLoadEvent::Finished\s*=>\s*BrowserTabEvent::NavigationFinished/);
// Normal tabs remain permissive; an isolated screenshot Worker must
// invalidate the document and restrict every redirect to credential-free HTTPS.
assert.match(plugin, /\.on_navigation\(move \|url\| \{\s*if let Some\(ready\) = &nav_ready \{[\s\S]*?invalidate_capture_document\(&nav_revision\);[\s\S]*?url\.scheme\(\) == "https" && url\.host_str\(\)\.is_some\(\)[\s\S]*?url\.username\(\)\.is_empty\(\) && url\.password\(\)\.is_none\(\)\s*\} else \{ true \}/);
assert.doesNotMatch(plugin, /on_navigation\(move \|url, navigate\| \{ callback\(navigate\); \}\)/);

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
// A queued GTK sweep is coalesced; native locks and a slow secondary
// stderr sink must not keep the global registry or drain path blocked.
assert.match(layout, /LAYOUT_SWEEP_IN_FLIGHT\.swap\(true, Ordering::AcqRel\)/);
assert.match(layout, /state\.child_layouts\.try_lock\(\)/);
assert.match(watchdog, /"main_thread":\s*\{/);
assert.match(watchdog, /"io_pressure": read_pressure\("\/proc\/pressure\/io"\)/);
assert.match(watchdog, /"swap_counters": swap_counters\(\)/);
assert.match(watchdog, /"layout_sweep": crate::bridge::layout_enforcer_status\(\)/);
assert.match(plugin, /fn reserve_lifecycle\(/);
assert.match(plugin, /fn offscreen_rect\(rect: LogicalRect\)[\s\S]{0,180}LogicalRect::new\(-30000\.0, rect\.y, rect\.width, rect\.height\)/);
assert.match(recovery, /source_error\.contains\("TAB_LIFECYCLE_BUSY"\)/);
assert.match(crashlog, /\.spawn\(move \|\| pump\(reader, session_tx, echo_tx\)\)\?;[\s\S]{0,400}libc::dup2\(writer\.as_raw_fd\(\), 2\)/);
assert.match(crashlog, /session\.try_send\(Arc::clone\(&chunk\)\)/);
assert.match(crashlog, /echo\.try_send\(chunk\)/);
const pump = crashlog.slice(crashlog.indexOf('fn pump('), crashlog.indexOf('/// 致命信号处理器'));
assert.doesNotMatch(pump, /write_all\(|\.flush\(|\b(?:session|echo)\.send\(/);
assert.match(crashlog, /SESSION_QUEUE_CHUNKS: usize = 128/);
assert.match(crashlog, /ECHO_QUEUE_CHUNKS: usize = 32/);
assert.match(watchdog, /browseros-ui-probe-submit/);
assert.match(watchdog, /probe_tx\.try_send\(\(\)\)/);
assert.match(watchdog, /stderr_session_dropped_bytes/);
assert.match(plugin, /fn reserve_tab_access(?:<'a>)?\(/);
assert.match(plugin, /TabAccessKind::Lifecycle if access\.lifecycle \|\| access\.operations > 0/);
assert.match(plugin, /let \(_operation, tab\) = self\.get_tab\(id, TabAccessKind::Operation\)\?/);
assert.match(plugin, /fn reconcile_rect\(/);
assert.match(plugin, /if current\.rect == rect \{\s*return platform::linux::ensure_size_allocated/);
assert.match(layout, /manager\.reconcile_rect\(&id, \|\|/);
assert.match(layout, /manager\.position_visible\(/);
assert.match(layout, /if result\.is_ok\(\) \{[\s\S]{0,250}last\.insert/);
assert.match(plugin, /pub fn position_visible\(/);
assert.match(layout, /impl Drop for LayoutSweepGuard/);
assert.match(recovery, /source_error\.contains\("TAB_LAYOUT_BUSY"\)/);
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
assert.match(codingOwner, /fn resume_coding\([\s\S]*?coding_scope\(root, Some\(session_id\)\)[\s\S]*?Self::begin_scoped\("coding", workspace, session, context\)/);
assert.match(codingOwner, /record\.workspace_id != workspace \|\| record\.session_id != session/);
assert.match(codingOwner, /tasks\.restore_paused_record\(restored\)/);
assert.match(aiBackend, /public_api::dispatch_coding_request\(/);
assert.match(codingFacade, /super::coding::execute_with_admission_adapter\(/);
assert.match(coding, /ProductionExecution::resume_coding\(\s*&root, &session\.session_id, session\.foundation_task\.as_ref\(\)/);
assert.match(coding, /execution\.acquire_lease\(\)\?;[\s\S]*?engine::drive_owned_function_batches\([\s\S]*?&execution_lease, 8, 8/);
console.log('BROWSER_NAVIGATION_HANG_DIAGNOSTICS_CONTRACT=PASS');
