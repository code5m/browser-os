#!/usr/bin/env node
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const read = (path) => readFileSync(path, "utf8").replace(/\r\n/g, "\n");
const rust = read("src-tauri/src/bridge/grid_runtime.rs");
const grid = read("src-tauri/src/grid_process.rs");
const windowsGrid = read("src-tauri/src/grid_process_windows.rs");
const lifecycle = read("src-tauri/src/grid_lifecycle.rs");
const ui = read("src/capabilities/grid/ui/GridRows.vue");

const region = (source, start, end) => {
  const from = source.indexOf(start);
  const to = source.indexOf(end, from + start.length);
  assert.ok(from >= 0 && to > from, "missing Grid owner region: " + start);
  return source.slice(from, to);
};

function assertRestartSafety(source, windows) {
  const monitor = region(source, "fn start_monitor(", windows ? "fn replay(" : "fn restart_ready_children(");
  const ready = windows ? monitor : region(source, "fn restart_ready_children(", "fn replay(");
  const owner = windows ? "manager" : "self";
  const spawnName = windows ? "spawn_with_state_locked" : "spawn_locked";
  const spawn = region(source, "fn " + spawnName + "(", "pub fn get_or_spawn(");
  const close = region(source, windows ? "fn kill_child_locked(" : "pub fn kill_child(",
    windows ? "pub fn kill_child(" : "pub fn restart(");
  const closeAll = region(source, "pub fn shutdown_all(",
    windows ? "/// Native ownership stays intact" : "pub fn request_shutdown(");

  assert.match(source, /crash_history:\s*Mutex<HashMap<u32, VecDeque<std::time::Instant>>>/);
  // The monitor admits at most three crashes in the existing 120-second window.
  // Only that admitted branch queues a restart; no direct spawn bypasses it.
  assert.match(monitor, /while events\.front\(\)\.is_some_and\([\s\S]*?\.as_secs\(\) > 120\)/);
  assert.match(monitor, /events\.push_back\(now\);\s*\(events\.len\(\) <= 3,\s*events\.len\(\)\)/);
  assert.match(monitor, /if allowed\s*\{\s*(?:eprintln!\("(?:\\.|[^"\\])*"\);\s*)?manager\.pending_restarts\.lock\(\)\.unwrap\(\)\.insert\(index,\s*\(/);
  assert.match(monitor, /from_millis\(350 \* attempts as u64\)/);
  assert.match(monitor, /manager\.lifecycle_lock\.lock\(\)\.unwrap\(\);\s*if manager\.lifecycle\.is_stopping\(\)\s*\{\s*break;/);

  // A ready restart waits for both its backoff and the old process-tree owner.
  assert.match(ready, new RegExp("\\*when <= std::time::Instant::now\\(\\)\\s*&& !" + owner + "\\.lifecycle\\.is_retiring\\(\\*\\*index\\)"));
  assert.match(ready, new RegExp("let Some\\(\\(_, saved\\)\\) = " + owner + "\\.pending_restarts\\.lock\\(\\)\\.unwrap\\(\\)\\.remove\\(&index\\) else\\s*\\{\\s*continue;"));
  assert.match(ready, new RegExp("if " + owner + "\\.children\\.lock\\(\\)\\.unwrap\\(\\)\\.contains_key\\(&index\\)\\s*\\{\\s*continue;\\s*\\}\\s*if let Err\\(error\\) = " + owner + "\\." + spawnName + "\\(index, Some\\(saved\\.clone\\(\\)\\)\\)"));
  if (!windows) {
    assert.match(ready, /let Ok\(_operation\) = self\.lifecycle\.enter\(\) else\s*\{\s*return;/);
    assert.match(ready, /self\.lifecycle_lock\.lock\(\)\.unwrap\(\);\s*if self\.lifecycle\.is_stopping\(\)\s*\{\s*return;/);
  } else {
    assert.match(spawn, /let _operation = self\.lifecycle\.enter\(\)\?;/);
  }
  assert.match(spawn, /self\.lifecycle\.check\(\)\?;/);
  assert.match(spawn, /if self\.lifecycle\.is_retiring\(index\)\s*\{\s*return Err/);

  // Explicit closes cancel deferred restarts before transferring native owners.
  assert.match(close, /self\.pending_restarts\.lock\(\)\.unwrap\(\)\.remove\(&index\);[\s\S]*?let removed = self\.children\.lock\(\)\.unwrap\(\)\.remove\(&index\)/);
  // Grid shutdown may not retire unrelated isolated browser tabs.
  assert.match(closeAll, /self\.pending_restarts\.lock\(\)\.unwrap\(\)\.retain\(\|id, _\| crate::grid_ipc::is_tab_renderer\(\*id\)\);[\s\S]*?children\.keys\(\)\.copied\(\)\s*\.filter\(\|id\| !crate::grid_ipc::is_tab_renderer\(\*id\)\)\.collect\(\);[\s\S]*?for index in indices\s*\{[\s\S]*?children\.remove\(&index\)[\s\S]*?self\.lifecycle\.retire\(index, owner\)/);
  assert.doesNotMatch(closeAll, /children\.drain\(\)/);
}

assert.match(rust, /CORE_MEMORY_RESERVE_MB:\s*u64\s*=\s*1800/);
assert.match(rust, /if affordable < MIN_GRID\s*\{\s*return Err/);
assert.doesNotMatch(rust, /affordable\.clamp\(MIN_GRID/);
assertRestartSafety(grid, false);
assertRestartSafety(windowsGrid, true);

const admission = region(lifecycle, "pub(super) fn enter(", "pub(super) fn request_shutdown(");
assert.match(admission, /self\.check\(\)\?;[\s\S]*?self\.operations\.fetch_add\(1, Ordering::SeqCst\)/);
assert.match(admission, /let permit = GridOperation\(Arc::clone\(self\)\);[\s\S]*?self\.check\(\)\?;\s*Ok\(permit\)/);
const stopCheck = region(lifecycle, "pub(super) fn check(", "pub(super) fn is_stopping(");
assert.match(stopCheck, /if self\.is_stopping\(\)\s*\{\s*Err\("GRID_SHUTDOWN_IN_PROGRESS"\.into\(\)\)/);

// Negative source fixtures make each rejection meaningful. They do not run
// native processes or replace the Rust lifecycle/process tests in CI.
for (const [source, windows] of [[grid, false], [windowsGrid, true]]) {
  for (const [name, pattern, replacement] of [
    ["unbounded crash loop", /events\.len\(\) <= 3/, "true"],
    ["unconditional restart admission", /if allowed\s*\{/, "if true {"],
    ["restart before native retirement", /&& !(?:self|manager)\.lifecycle\.is_retiring\(\*\*index\)/, "&& true"],
    ["restart during shutdown", /if manager\.lifecycle\.is_stopping\(\)\s*\{\s*break;/, "if false { break;"],
    ["closed cell keeps deferred restart", /self\.pending_restarts\.lock\(\)\.unwrap\(\)\.remove\(&index\);/g, "self.pending_restarts.lock().unwrap().get(&index);"],
    ["closed view keeps deferred restarts", /self\.pending_restarts\.lock\(\)\.unwrap\(\)\.retain\(\|id, _\| crate::grid_ipc::is_tab_renderer\(\*id\)\);/, "self.pending_restarts.lock().unwrap().len();"],
  ]) {
    const mutated = source.replace(pattern, replacement);
    assert.notEqual(mutated, source, "ineffective fixture: " + name);
    assert.throws(() => assertRestartSafety(mutated, windows), { name: "AssertionError" }, name);
  }
}

// Resource polling follows the actually visible advanced panel, not a stale global grid row.
assert.match(ui, /watch\(\(\) => \[layout\.mainView, advancedOpen\.value\]/);
assert.match(ui, /view !== "grid" \|\| !advancedOpen\.value/);
assert.match(ui, /stopResourcePolling\(\)/);
assert.match(ui, /onBeforeUnmount\(stopResourcePolling\)/);
console.log("CORE_STABILITY_GUARDS=PASS (static contracts; native GUI evidence separate)");
