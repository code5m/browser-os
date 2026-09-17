# 09 — SMALL-MODEL FAILURE MODES
> Agent G · READ-ONLY audit · Tauri2+Vue3 browser-OS (mvp-browser-os-v3)
> How a capability-modest, context-limited model could make a change that COMPILES
> but is SEMANTICALLY WRONG. Each mode = template + a re-verified historical case.

SeverITY scale:
- S4 = compiles, silently wrong in production, core UX/data, hard to detect
- S3 = compiles, wrong under specific conditions, moderate detectability
- S2 = compiles, localized, easy to detect / recoverable
- S1 = caught by lint/type   ·   S0 = caught at compile/runtime crash

Classification legend for evidence: FACT / INFERENCE / RECOMMENDATION / UNVERIFIED.

══════════════════════════════════════════════════════════════════════════════
SMF-001  —  CASE-001  activateWeb + isBrowserView() confusion   [AMBIGUOUS_NAME / CROSS_FILE]
Scenario: A model reviews UnifiedTabBar.vue and "simplifies" activateWeb to reuse the shared isBrowserView() predicate instead of the exact mainView check.
Local code: src/components/layout/UnifiedTabBar.vue:147-154 (activateWeb uses `layout.mainView !== "browser"`); src/stores/useLayoutStore.ts:188-190 (isBrowserView includes grid).
Wrong modification: Replace `if (layout.mainView !== "browser")` with `if (!layout.isBrowserView())`.
Why it compiles: Both are valid booleans; lint-clean.
Hidden contract violated: isBrowserView() INCLUDES "grid". In grid view the guard becomes false → setView("browser") skipped → schedulePosition takes the grid branch → clicked tab never positioned into view.
Runtime consequence: Clicking a browser tab while the grid is open does nothing visible.
Required context: The activateWeb comment (UnifiedTabBar.vue:149-151) already spells out the trap.
How architecture should prevent it: Expose a single `isBrowserViewStrict()`; checker asserts activateWeb does NOT reference isBrowserView().
Evidence: EVID-0002, EVID-0003, EVID-0004

SMF-002  —  CASE-006  create_grid FE/Rust contract half-change   [IPC_PARTIAL_CHANGE / CROSS_FILE]
Scenario: A model renames Rust create_grid → createGrid, or drops the FE `if (degraded) gridCount.value = created` line.
Local code: bridge.rs:3851 (create_grid(app, n, urls)); bridge.rs:3820 comment "前端契约不变"; bridge.ts:534-535 createGrid(n, urls); useBrowserStore.ts:314-317 gridCount reassignment.
Wrong modification: Rename Rust command OR delete the degraded-count reassignment.
Why it compiles: Rust + TS compile independently; mismatch only at IPC string boundary.
Hidden contract violated: (a) FE invokes literal "create_grid" string; (b) Rust returns MEMORY-BUDGETED count; if FE ignores it, gridCount diverges → gridRects/scheduleGrid over wrong N → 12×450MB ≈ 5.4 GB, swap thrash.
Runtime consequence: Grid never opens (IPC error swallowed) or FE gridCount=4 while Rust spawned 12 → OS stall.
How architecture should prevent it: Single registry for command names; checker asserting every FE invoke has matching Rust command.
Evidence: EVID-0013, EVID-0014, EVID-0015, EVID-0016, EVID-0031, EVID-0032

SMF-003  —  CASE-004  grid "hide" vs closeGridAll (destroy)   [HIDDEN_SIDE_EFFECT / LOCAL_FIX_GLOBAL_BREAKAGE]
Scenario: A model adds a "minimize grid to dock" that sets gridOpen=false but forgets bridge.closeGrid().
Local code: useBrowserStore.ts:20 (gridOpen); 483-511 (closeGridAll); bridge.rs:3915-3944 (close_grid kills children).
Wrong modification: New minimizeGrid() that only does `gridOpen.value = false`.
Why it compiles: Plain boolean + no IPC.
Hidden contract violated: gridOpen is a visibility/positioning gate, NOT lifecycle destroy. Subprocesses survive. No separate "hide" primitive.
Runtime consequence: Each minimized grid leaves a 450 MB WebKit child process running.
How architecture should prevent it: Single `destroyGrid()` that flips gridOpen AND calls bridge.closeGrid(); checker forbids `gridOpen.value = false` outside it.
Evidence: EVID-0005, EVID-0007, EVID-0017, EVID-0032

SMF-004  —  CASE-007  browser/grid visibility double state   [MULTIPLE_STATE_TRUTH]
Scenario: A model "cleans up" closeGridAll by removing the mainView reset, reasoning "gridOpen=false is the real signal".
Local code: useBrowserStore.ts:483-511 (closeGridAll resets mainView to "browser"); :491 comment; :149-151 isBrowserVisible = mainView==="browser"; BrowserHost.vue:15 visibility; useBrowserHost.ts:68 schedulePosition early-returns unless mainView==="browser".
Wrong modification: Delete the `if (layout.mainView === "grid") layout.mainView = "browser"` block.
Why it compiles: Removing a guarded assignment is syntactically fine.
Hidden contract violated: BrowserHost visibility and schedulePosition BOTH key off mainView==="browser", not gridOpen. After closeGridAll with mainView still "grid": isBrowserVisible=false → BrowserHost hidden AND schedulePosition returns early → active tab webview stays off-screen.
Runtime consequence: Browser area BLANK after closing the grid (exact B9-4 P1 bug).
How architecture should prevent it: Collapse grid-visibility to ONE derived selector consumed by BrowserHost/schedulePosition/syncFreeze.
Evidence: EVID-0001, EVID-0007, EVID-0008, EVID-0011, EVID-0030

SMF-005  —  CASE-010a  SessionCloseDialog checker conflict   [CHECKER_BLIND_SPOT]
Scenario: A model reads scripts/check-ui.mjs, sees SessionCloseDialog listed as "[CURRENT]" protocol, and re-adds the revoked component.
Local code: check-ui.mjs:37,53,196,222-243,409; check-session-persistence-policy.py:21,230,573-574; check-native-webview-overlay.mjs:6,25,26 (asserts must NOT exist); useBrowserStore.ts:185-187 (revoked 2026-09-12).
Wrong modification: Re-create SessionCloseDialog.vue and mount in App.vue.
Why it compiles: A new component + mount compiles and passes check-ui.mjs.
Hidden contract violated: Two checkers disagree → CI contradictory; revived component re-introduces banned "webviewsSuspended" overlay.
How architecture should prevent it: Single source of truth for protocol state; remove stale SessionCloseDialog references.
Evidence: EVID-0020, EVID-0021, EVID-0022, EVID-0023

SMF-006  —  CASE-010b  FilePanel destructive ops have no semantic checker   [CHECKER_BLIND_SPOT]
Scenario: A model adds a file-mutating action in FilePanel.vue calling bridge.writeFile/deletePath directly.
Local code: FilePanel.vue (file manager); bridge.ts:497-523 (writeFile/createFile/createDir/deletePath/renamePath/movePath bare invoke).
Wrong modification: Add delete/rename handler calling bridge.deletePath without store guards.
Why it compiles: Bridge signatures typed; any path string compiles.
Hidden contract violated: No checker guards FilePanel destructive semantics → local "fix" becomes global data-loss path.
How architecture should prevent it: Checker forbidding `bridge.deletePath|writeFile|renamePath|movePath` outside useWorkspaceStore.ts.
Evidence: EVID-0024, EVID-0025

SMF-007  —  DUP-004 drift: isBrowserVisible formula mismatch   [CHECKER_BLIND_SPOT]
Scenario: A model runs checkers, sees check-grid-close-logic.mjs "fail" on G2, and "fixes" the source computed to match `!gridOpen && mainView==="browser"`.
Local code: useBrowserStore.ts:149-151 (isBrowserVisible = mainView==="browser"); check-grid-close-logic.mjs:112 asserts formula CONTAINS `!gridOpen.value`.
Wrong modification: Change computed to `() => !gridOpen.value && layout.mainView === "browser"`.
Why it compiles: Trivial valid computed.
Hidden contract violated: Current refactor intentionally keys BrowserHost visibility on mainView only. Re-adding !gridOpen can hide browser webview while a grid is merely flagged.
How architecture should prevent it: The checker is the one to update; add comment noting "do NOT add !gridOpen".
Evidence: EVID-0001, EVID-0008, EVID-0009, EVID-0010

SMF-008  —  DUP-008  tab store-action vs bare bridge   [LOCAL_FIX_GLOBAL_BREAKAGE / IPC_PARTIAL_CHANGE]
Scenario: A model wires a new "close this tab" affordance calling bridge.tabClose(id) directly.
Local code: useBrowserStore.ts:188-211 (closeTabNow: recordClose + splice + tabActivate + relocate + syncFreeze); bridge.ts:573 (tabClose bare invoke).
Wrong modification: Component calls `bridge.tabClose(id)` instead of `browser.tabClose(id)`.
Why it compiles: Both accept string id; typed.
Hidden contract violated: Store action maintains tabs[] and recentlyClosed[] and triggers relocate/syncFreeze. Bare call leaves tabs[] stale and restore stack broken.
How architecture should prevent it: Forbid `bridge.tabClose|tabNew` outside useBrowserStore.ts via grep checker.
Evidence: EVID-0026, EVID-0027

SMF-009  —  DUP-007  freeze truth source mismatch   [MULTIPLE_STATE_TRUTH / CROSS_FILE]
Scenario: A model "harmonizes" syncFreeze to read only mainView (matching syncViewVisibility).
Local code: useBrowserStore.ts:619-631 (syncFreeze reads gridOpen AND mainView); :646-655 (syncViewVisibility branches only on mainView).
Wrong modification: Replace `if (gridOpen.value)` with `if (layout.mainView === 'grid')` in syncFreeze.
Why it compiles: Boolean refactor, lint-clean.
Hidden contract violated: When grid open but view is "browser" (mid-transition), grids must stay UNFROZEN; collapsing to mainView alone either freezes visible grids or iterates dead grid set.
How architecture should prevent it: Single `isGridLive()` selector feeding both syncFreeze and scheduleGrid.
Evidence: EVID-0011, EVID-0012

SMF-010  —  DUP-003  view-switch entrypoint confusion   [DUPLICATE_ENTRYPOINT]
Scenario: A model adds a new place that switches to browser using raw `layout.mainView = "browser"` (not setView()).
Local code: useLayoutStore.ts:193-199 (setView also clears fileEditorOpen + navSection); raw assignments elsewhere.
Wrong modification: New entry point does `layout.mainView = "browser"` without navSection/fileEditorOpen cleanup.
Why it compiles: Direct ref assignment.
Hidden contract violated: setView guarantees navSection and fileEditorOpen reset. Raw assignment leaves stale menu/editor overlay.
How architecture should prevent it: Make mainView write-only via setView(); checker forbids `layout.mainView =` outside setView.
Evidence: EVID-0029, EVID-0003

SMF-011  —  ORDER_DEPENDENT_API  grid count range coupling   [ORDER_DEPENDENT_API]
Scenario: A model bumps default grid count or MAX_GRID without coordinating gridUrls length and gridCols() mapping.
Local code: useBrowserStore.ts:27-33 (gridUrls default length 12); :291-299 (gridCols 2..12); bridge.rs:3822 (MAX_GRID=12); :3852 (n.clamp(2,MAX_GRID)).
Wrong modification: Change gridCount default or MAX_GRID without extending gridUrls / gridCols.
Why it compiles: Independent numeric constants.
Hidden contract violated: buildGrid builds urls from gridUrls[i] for i in 0..n; if n>12 Rust clamps to 12 and gridCols must have a branch for new n.
How architecture should prevent it: Derive gridCols and MAX_GRID from one shared constant; checker asserts gridUrls length >= MAX_GRID.
Evidence: EVID-0013, EVID-0031, EVID-0014

SMF-012  —  LIFECYCLE_CONFUSION  gridSession cache-bust   [LIFECYCLE_CONFUSION]
Scenario: A model "removes redundant increments" of gridSession, thinking it's unused state.
Local code: useBrowserStore.ts:23 (gridSession), :302 (buildGrid++), :560-570 (forceGridRelayout++); useBrowserHost.ts:124-126 (gridCache.syncSession).
Wrong modification: Delete gridSession++ in buildGrid or forceGridRelayout.
Why it compiles: Removing a counter is trivially valid.
Hidden contract violated: After rebuild, rect may coincide with prior state, so 50ms de-dup SKIPS re-sending. gridSession is the explicit cache-buster.
How architecture should prevent it: Rename to `gridRebuildEpoch`; checker asserts syncSession fed incremented epoch on buildGrid/forceGridRelayout.
Evidence: EVID-0028

══════════════════════════════════════════════════════════════════════════════
CASE VERIFICATION SUMMARY (re-verified against source, not assumed)
- CASE-001 activateWeb + isBrowserView: PROVEN (UnifiedTabBar.vue:147-154; comment documents trap).
- CASE-006 create_grid FE/Rust contract half-change: PROVEN (bridge.rs:3851 + bridge.ts:534-535 + memory-budget return + FE gridCount reassignment).
- CASE-004 grid hide vs closeGridAll: PROVEN (no distinct hide primitive; closeGridAll only destroyer).
- CASE-007 browser/grid visibility double state: PROVEN (gridOpen vs mainView==="grid" vs gridToolbarOpen).

SEVERITY COUNTS (this document's SMF modes)
  S4 = 4   (SMF-001, SMF-002, SMF-003, SMF-004)
  S3 = 8   (SMF-005, SMF-006, SMF-007, SMF-008, SMF-009, SMF-010, SMF-011, SMF-012)
  S2 = 0

TOP 5 MOST DANGEROUS FAILURE MODES (ranked by blast radius + silence)
  1. SMF-002  create_grid FE/Rust contract half-change  → system-wide memory blow-up, silent.
  2. SMF-003  grid hide vs closeGridAll                 → 450 MB orphan subprocesses per grid, silent.
  3. SMF-004  browser/grid visibility double state       → blank browser viewport (B9-4 regression).
  4. SMF-001  activateWeb + isBrowserView() confusion    → tabs silently invisible in grid view.
  5. SMF-008  tab store-action vs bare bridge            → restore stack + tabs[] desync, silent.
