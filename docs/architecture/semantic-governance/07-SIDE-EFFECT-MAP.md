# 07-SIDE-EFFECT-MAP

> Agent E — Semantic Governance Audit (READ-ONLY)
> Project: `mvp-browser-os-v3` (Tauri2 + Vue3)
> Principle audited: **ONE SIDE-EFFECT EXIT** (one Native/resource side effect → one controlled exit).
> Caveats honored: VIEW SWITCH != DESTROY, HIDE != CLOSE, POSITION != SHOW.

---

## 1. Side-Effect Map (Summary Table)

| Side Effect | Entry Points | Canonical Exit | Hidden Effects | Multiple Exits | Risk |
|---|---|---|---|---|---|
| WebView **create** | `tab_new`/`create_tab` (bridge.rs:4266/723), `open_browser` (458), `tab_activate` hibernation rebuild (4663/4680→`spawn_child_window` 104), `recover_tab_webview` (163/206); grid `create_grid` (3850)→`get_or_spawn` (grid_process.rs:279)→`spawn` (236) | `spawn_child_window` (bridge.rs:104) for tabs; `get_or_spawn`/`spawn` for grids | create also navigates to url; child created 1×1 hidden | create converges to 2 factories | S2 |
| WebView **destroy** | `tab_close`→`close_tab` (4273/794), `close_browser` (465), `hibernate_tab` (4559), `close_grid` (3915), `grid_close_one` (4059), `kill_child` (689) | `close_tab` (manager.close_tab) tabs; `GridCmd::CloseTab`+`kill_child` grids | `close_tab` evicts broad metadata | single tab destroy; grid two-step intentional | S2 |
| WebView **show** | `apply_bounds_inner` `set_visible(true)` (555) via `tab_position`(4299)/`position_browser`(475); grid `GridCmd::UpdateRect`→`win.show()` (main.rs:423) | **IMPLICIT** — no dedicated `show_webview` command; show folded into position | POSITION_HAS_VISIBILITY_SIDE_EFFECT | 3 position entries all perform show | **S4** |
| WebView **hide** | `hide_webview`(608),`hide_all_webviews`(635),`tab_activate` internal `hide_bounds` loop (4709), `tabNew` `tabPosition(-30000)` (useBrowserStore.ts:170), grid `GridCmd::HideWindow`→`win.hide()` (main.rs:426) | `hide_bounds`→`update_rect(-30000)` offscreen (570) tabs; `HideWindow`→`win.hide()` grids | hide uses offscreen move, NOT `set_visible(false)` | `hide_webview`/`hide_all_webviews`/`tab_activate`/direct `tabPosition(-30000)` | **S3** |
| WebView **position** | `tab_position`(4299),`position_browser`(475),`grid_position`(4022)→`GridCmd::UpdateRect` | `apply_bounds_inner` (update_rect) tabs; `UpdateRect` grids | position ALSO sets size AND shows | `tab_position` vs `position_browser` thin variants | **S4** |
| WebView **resize** | folded into `position` (update_rect carries w/h); grid `set_size` in `UpdateRect` | `update_rect` | none beyond position coupling | none | S2 |
| **navigation** | `tab_open`(4279→`navigate_tab_webview`156),`tab_new`,`grid_open`(3949→`GridCmd::Navigate`),`tab_go_back/forward`(4630/4636) | `navigate_tab_webview` / `GridCmd::Navigate` | none | none | S2 |
| **reload** | `tab_reload`(4642→`plugin_eval` `location.reload`),`tabReload`(useBrowserStore.ts:256) | `plugin_eval` | none | none | S2 |
| **eval** | `eval_in_tab`(4649),`plugin_eval`(143),`tab_go_back/forward`,`syncFreeze` JS, AI inject | `plugin_eval`/`GridCmd::Eval` | grid eval returns `"ok"` without awaiting real result | tab vs grid semantics diverge | **S3** |
| grid child **spawn** | `create_grid`(3850)→`get_or_spawn`(279)→`spawn`(236) | `get_or_spawn`/`spawn` | create uses `visible:true` | single factory | S2 |
| grid **process kill** | `close_grid`(shutdown_all+kill),`grid_close_one`(4066),`kill_child`(689),create rollback (3877/3896) | `kill_child` (child.kill()+wait) (689) | none (waited) | `kill_child` + `shutdown_all` | S2 |
| **filesystem write** | `write_file`(3640),`create_file`(3704),`create_dir`(3736),`delete_path`(3761),`rename_path`(3780),`move_path`(fs_cmds.rs:21) | ALL route through `security_policy::check_path_within_roots`+`allowed_roots` then `std::fs::write` | guard is per-command | multiple commands, single guard | S2 |
| PTY **create** | `term_spawn`(4758),`term_spawn_channel`(4771) | `spawn_terminal` (terminal.rs:552) | two create commands (term_spawn legacy) | `term_spawn`+`term_spawn_channel` | S3 |
| PTY **write** | `term_write`(4787) | `session.writer.write_all` | none | single | S2 |
| PTY **resize** | `term_resize`(4801)→`terminal::resize`(648) | `master.resize` | none | single | S2 |
| PTY **kill** | `term_kill`(4817)→`terminate_session`(661)→`terminate_group`(SIGTERM→SIGKILL→wait)+thread join | `terminate_session` (waited) | PROPERLY WAITED | single | S2 |
| **keyring** | `KeyringStore::save_token`(keyring_store.rs:10) | `keyring::Entry::set_password` | token never reaches frontend JS | single exit | S2 |
| **session persistence** | `sessionSave`,`sessionDiscard`,`flushSessions`,`upsert_session_draft`(1618); `session.rs::save_session`(146 atomic_write) | `session.rs::atomic_write` + `workspace.rs` atomic writes | drafts in-memory, never auto-persisted | 2 persistence helpers | S2 |

---

## 2. CASE Verifications

### CASE-005 — `gridPosition` / `UpdateRect` also sets size/show? → **CONFIRMED POSITION_HAS_VISIBILITY_SIDE_EFFECT**
- **Tabs:** `apply_bounds_inner` (bridge.rs:531-564) calls `manager.set_visible(&id, true)` at **line 555**, immediately after `update_rect`. So `tab_position`/`position_browser` (named "position") perform an implicit **SHOW**.
- **Grids:** `dispatch_grid_cmd` for `GridCmd::UpdateRect` (main.rs:391-425) performs `set_position` + `set_size` + `win.show()` at **line 423**.
- ⇒ Both tab and grid "position" violate the POSITION != SHOW caveat. Classification: FACT.

### CASE-007 — BrowserHost visibility double-layer / native show-hide canonical exit
- **Double-layer confirmed:** `BrowserHost.vue:15` sets DOM `visibility: browser.isBrowserVisible ? 'visible':'hidden'` — hides only the DOM placeholder, NOT the actual floating GTK webview. The native webview visibility is governed separately by `tab_position`/`hide_webview`.
- **Canonical hide exit:** `hide_webview`(608)/`hide_all_webviews`(635) → `hide_bounds`(570) → `update_rect(-30000)` offscreen (NOT `set_visible(false)`, deadlock-avoidance per bridge.rs:567-569). Grids: `GridCmd::HideWindow`→`win.hide()` (main.rs:426).
- **Canonical show exit:** NONE dedicated — show is implicit via every `position` command.
- **Direct-native-bypass found:** `useBrowserStore.ts:170` calls `bridge.tabPosition(t.id, {x:-30000,...})` directly to hide a freshly created tab (bypassing `hide_webview`).

### CASE-009 — `sync_browser_scene`: exactly ONE adapter?
- **No backend `sync_browser_scene` exists** (searched src-tauri + src). The "browser scene sync" lives on the **frontend**: `useBrowserStore.ts::syncViewVisibility` (646) is the intended single adapter.
- **However multiple call sites perform equivalent native work:** `useBrowserHost.ts:172` `bridge.hideAllWebviews()` called directly in `onMounted`; `useBrowserStore.ts:170` direct `tabPosition(-30000)`; `relocate()`/`schedulePosition()` invoked from `tabNew`/`tabSwitch`/`closeTabNow`/`tabReload`/`tabNavigate` independently of `syncViewVisibility`.
- ⇒ Not exactly one clean adapter — scene-sync logic is duplicated across store + composable.

### CASE-010 — Cross-check with existing checkers

**Already enforced (GUARDED):**
- `check-native.mjs`: danger-zone file-edit gate (bridge.rs, `**/linux.rs`, capabilities, `tauri-browser-tabs/**`); deprecated patterns `set_size_request`, `queue_resize`, `webview.hide()` (raw `.hide()`), offscreen 1×1, `devicePixelRatio*`.
- `check-native-webview-overlay.mjs`: no toast-pop overlay, no `SessionCloseDialog`, no `webviewsSuspended`.
- `check-grid-close-logic.mjs`: `closeGridAll` resets `mainView="browser"`, calls `tabActivate`+`schedulePosition`+`closeGrid`, flips `gridOpen=false`, clears `gridRects`.
- `check-lifecycle-contract.py`: 6 legacy gaps; expects 0 today. `TERMINAL_KILL_NOT_WAITED` resolved (`terminate_group` calls `child.wait()`). `TAB_CLOSE_FAILURE_SHORT_CIRCUITS_CLEANUP` resolved (`close_tab` runs metadata eviction AFTER close). `GRID_PARTIAL_CREATE_ROLLBACK_MISSING` resolved (`create_grid` rolls back via `kill_child`).

**Unguarded (GAP / CASE-010 findings):**
- **G1:** `main.rs` is **NOT** in the `check-native` danger zone (regex matches only bridge.rs exactly, `linux.rs`, capabilities, `tauri-browser-tabs`). The child-process-side native calls `win.show()`(423)/`win.hide()`(430) and main-window calls are **never scanned**.
- **G2:** No checker verifies **POSITION_HAS_VISIBILITY_SIDE_EFFECT** (CASE-005).
- **G3:** No checker enforces "exactly one show exit" / single show-hide adapter (CASE-007/009).
- **G4:** `eval_in_tab` grid path returns `"ok"` immediately without the real eval result — unguarded.
- **G5:** FS writes / workspace / keyring / session persistence are outside the native danger-zone concept.

---

## 3. Evidence (EVID) Records

**EVID-005-TAB** · Claim: tab `position` command performs an implicit SHOW (set_visible true). · Classification: FACT · File: `src-tauri/src/bridge.rs` · Symbol: `apply_bounds_inner` · Current line: 555 · Confidence: HIGH

**EVID-005-GRID** · Claim: grid `grid_position`/UpdateRect also sets size and shows the window. · Classification: FACT · File: `src-tauri/src/main.rs` · Symbol: `dispatch_grid_cmd` (arm `GridCmd::UpdateRect`) · Current line: 391-425 (show at 423) · Confidence: HIGH

**EVID-SHOW-007** · Claim: there is no dedicated native "show webview" command; show is folded into 3 position entries. · Classification: FACT · File: `src-tauri/src/bridge.rs` + `src-tauri/src/main.rs` · Symbol: `apply_bounds_inner`(555)/`dispatch_grid_cmd`(423) · Confidence: HIGH

**EVID-HIDE-007** · Claim: webview hide has multiple overlapping exits (command + internal loop + offscreen-move convention). · Classification: INFERENCE · File: `src-tauri/src/bridge.rs` · Symbol: `hide_webview`(608)/`hide_all_webviews`(635)/`tab_activate`(4709) · Confidence: MEDIUM

**EVID-BYPASS-007** · Claim: `useBrowserStore.tabNew` hides a tab via `tabPosition(-30000)` directly, bypassing `hide_webview`. · Classification: FACT · File: `src/stores/useBrowserStore.ts` · Symbol: `tabNew` · Current line: 170 · Confidence: HIGH

**EVID-SYNC-009** · Claim: browser-scene sync is not centralized in one adapter; equivalent native work duplicated. · Classification: INFERENCE · File: `src/stores/useBrowserStore.ts` + `src/composables/useBrowserHost.ts` · Symbol: `syncViewVisibility`(646) / `onMounted`(172) · Confidence: MEDIUM

**EVID-ACTIVATE-007** · Claim: `tab_activate` silently CREATES a webview when the target is hibernated. · Classification: FACT · File: `src-tauri/src/bridge.rs` · Symbol: `tab_activate` · Current line: 4663-4682 · Confidence: HIGH

**EVID-TERM-CREATE** · Claim: two PTY-create commands exist (`term_spawn` + `term_spawn_channel`). · Classification: FACT · File: `src-tauri/src/bridge.rs` · Symbol: `term_spawn`(4758)/`term_spawn_channel`(4771) · Confidence: HIGH

**EVID-EVAL-RESULT** · Claim: grid `eval_in_tab` returns `"ok"` immediately without the real eval result; tab eval returns only success/failure. · Classification: FACT · File: `src-tauri/src/bridge.rs` · Symbol: `eval_in_tab`(4649) · Confidence: HIGH

**EVID-CLOSE-META** · Claim: `close_tab` performs broad metadata eviction beyond webview destruction. · Classification: FACT · File: `src-tauri/src/bridge.rs` · Symbol: `close_tab` · Current line: 804-817 · Confidence: HIGH

**EVID-EVALRESULT-MISSING** · Claim: a distinct `eval_result` native side effect does not exist in this codebase. · Classification: UNVERIFIED · Confidence: LOW

**EVID-FS-GUARD** · Claim: all FS writes funnel through one `security_policy` guard then `std::fs::write`. · Classification: FACT · Confidence: HIGH

**EVID-KEYRING** · Claim: keyring token write is a single exit and never reaches frontend JS. · Classification: FACT · Confidence: HIGH

---

## 4. Findings by Category

### HIDDEN_SIDE_EFFECT
- EVID-005-TAB (position→show); EVID-005-GRID (position→size+show); EVID-CLOSE-META (destroy→broad metadata eviction); EVID-EVAL-RESULT (grid eval result suppressed); EVID-ACTIVATE-007 (activate silently creates webview).

### MULTI_SEMANTIC_API
- EVID-005-TAB / EVID-005-GRID (position = position + show [+ size]); EVID-SHOW-007 (no dedicated show; 3 entries perform show); EVID-ACTIVATE-007 (activate = show + maybe create); EVID-EVAL-RESULT (same `eval_in_tab` API, divergent tab/grid semantics).

### SIDE_EFFECT_DUPLICATION
- EVID-HIDE-007 (hide replicated: command / activate-loop / offscreen convention); EVID-BYPASS-007 (`tabPosition(-30000)` position-as-hide); EVID-SYNC-009 (scene sync duplicated across store + composable + tabNew); EVID-TERM-CREATE (`term_spawn` + `term_spawn_channel`).

---

## 5. Severity Counts (S4 / S3 / S2)

- **S4 = 3**: EVID-005-TAB, EVID-005-GRID, EVID-SHOW-007
- **S3 = 6**: EVID-HIDE-007, EVID-BYPASS-007, EVID-SYNC-009, EVID-ACTIVATE-007, EVID-TERM-CREATE, EVID-EVAL-RESULT
- **S2 = 4**: WebView create/destroy/FS/PTY-kill+keyring+session clean band
- **UNVERIFIED = 1**: EVID-EVALRESULT-MISSING

---

## 6. Conflicts with Other Agents

- **vs Agent C (lifecycle):** No conflict on resolved gaps. Caveat: grid close uses deliberate two-step `HideWindow`→`CloseTab`→`kill_child`; if C assumes "kill == destroy" it should note the hide-first ordering is intentional.
- **vs Agent D (IPC):** **Conflict.** D models `tab_position`/`grid_position` as pure layout IPC. The side-effect map proves they carry a SHOW side effect (EVID-005) — the IPC surface has hidden visibility semantics. Also `eval_in_tab` returns `"ok"` regardless of grid result (EVID-EVAL-RESULT), contradicting any IPC contract that promises an eval result.
- **vs Agent B (actions):** **Conflict.** B's action model likely enumerates a discrete "show webview" action. Reality: there is **no** native show command — show is implicit in position (EVID-SHOW-007). Conversely B may under-specify that `tab_activate` can recreate a webview (EVID-ACTIVATE-007).
