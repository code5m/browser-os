#!/usr/bin/env node
import assert from "node:assert/strict";
import {readFileSync} from "node:fs";
const rust=readFileSync("src-tauri/src/bridge.rs","utf8");
const grid=readFileSync("src-tauri/src/grid_process.rs","utf8");
const ui=readFileSync("src/capabilities/grid/ui/GridRows.vue","utf8");
assert.match(rust,/CORE_MEMORY_RESERVE_MB:\s*u64\s*=\s*1800/);
assert.match(rust,/if affordable < MIN_GRID\s*\{\s*return Err/);
assert.doesNotMatch(rust,/affordable\.clamp\(MIN_GRID/);
assert.match(grid,/crash_history:\s*Mutex<HashMap<u32, VecDeque<std::time::Instant>>>/);
assert.match(grid,/events\.len\(\) <= 3/);
assert.match(grid,/if !should_restart/);
assert.match(ui,/watch\(\(\) => \[layout\.mainView, layout\.navSection\]/);
assert.match(ui,/onBeforeUnmount\(stopResourcePolling\)/);
console.log("CORE_STABILITY_GUARDS=PASS (static contracts; native GUI evidence separate)");
