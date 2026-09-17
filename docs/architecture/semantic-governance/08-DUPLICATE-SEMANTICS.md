# 08 — DUPLICATE / AMBIGUOUS SEMANTICS
> Agent G · READ-ONLY audit · Tauri2+Vue3 browser-OS (mvp-browser-os-v3)
> Purpose: enumerate concrete duplicated or ambiguous semantics in source that a
> capability-modest, context-limited model could confuse into a change that
> COMPILES but is SEMANTICALLY WRONG.

Severity scale:
- S4 = compiles, silently wrong in production, core UX/data, hard to detect
- S3 = compiles, wrong under specific conditions, moderate detectability
- S2 = compiles, localized, easy to detect / recoverable
- S1 = caught by lint/type   ·   S0 = caught at compile/runtime crash

NOTE on Agent F merge: docs/architecture/semantic-governance/_findings/F-FEATURES.md
was NOT present at audit time, so no F- duplicate items were merged. The cases
below were derived directly from source + checkers. (If F-FEATURES.md appears later,
re-run and merge: a likely overlap is F's "browser/grid visibility" and "grid close"
items, which map to DUP-002 / DUP-006 / DUP-007.)

────────────────────────────────────────────────────────────────────────────
DUP-001  —  isBrowserVisible vs isBrowserView()
Concept:        Two near-identically named predicates that return DIFFERENT sets.
Where duplicated:
  - src/stores/useBrowserStore.ts:149-151  `isBrowserVisible` (computed) => mainView === "browser" (EXCLUDES "grid")
  - src/stores/useLayoutStore.ts:188-190 `isBrowserView()` (function) => mainView === "browser" || "grid" (INCLUDES "grid")
Why dangerous: A model reusing "the browser-visible helper" may swap one for the other, flipping behavior exactly in the grid<->browser boundary.
Risk: S4
Evidence IDs: EVID-0001, EVID-0002, EVID-0030

DUP-002  —  "is the grid showing" encoded in 3 independent flags
Concept:        Grid-visible truth is split across three uncoordinated state sources.
Where duplicated:
  - src/stores/useBrowserStore.ts:20  `gridOpen` (ref, FE intent flag)
  - src/stores/useLayoutStore.ts:138/144  `mainView === "grid"` AND `gridToolbarOpen`
  - the close-grid path also mutates all three (useBrowserStore.ts:483-511)
Why dangerous: None of the three is a single source of truth. A model editing one branch and "cleaning up" the others can desync them, leaving the grid subprocess alive but invisible, or the browser viewport blank.
Risk: S4
Evidence IDs: EVID-0005, EVID-0006, EVID-0007, EVID-0011

DUP-003  —  three ways to switch the main view
Concept:        view-transition has overlapping entry points with different side effects.
Where duplicated:
  - src/stores/useLayoutStore.ts:193-199  `setView(v)` (also closes fileEditor + navSection)
  - raw assignment `layout.mainView = "browser"` (dozens of sites)
  - `layout.isBrowserView()` used as a predicate for "am I in browser-land?"
Why dangerous: A model may switch views with a raw `mainView=` assignment thinking it equals setView(); it then silently skips navSection/fileEditorOpen cleanup.
Risk: S3
Evidence IDs: EVID-0029, EVID-0003

DUP-004  —  the B9-4 formula is duplicated AND stale
Concept:        `isBrowserVisible = !gridOpen && mainView==="browser"` is asserted in two places but NO LONGER matches the source.
Where duplicated (and wrong):
  - src/stores/useBrowserStore.ts:491  comment inside closeGridAll
  - scripts/check-grid-close-logic.mjs:112  regex assertion G2
  - ACTUAL source (src/stores/useBrowserStore.ts:149-151) is only `mainView === "browser"`.
Why dangerous: A model reading the comment/checker may "repair" the computed to include `!gridOpen`, reintroducing the very coupling the refactor removed.
Risk: S3
Evidence IDs: EVID-0008, EVID-0009, EVID-0010

DUP-005  —  three wrappers over the same grid-layout trigger
Concept:        layoutGrid / forceGridRelayout / scheduleGrid overlap; only one busts cache.
Where duplicated:
  - src/stores/useBrowserStore.ts:344-346  `layoutGrid()` => scheduleGrid()
  - src/stores/useBrowserStore.ts:560-570  `forceGridRelayout()` => gridSession++ then layoutGrid
  - src/composables/useBrowserHost.ts:97-103 / 124-126  `scheduleGrid()` + gridSession cache
Why dangerous: A model "deduplicating" may replace forceGridRelayout's gridSession++ with a plain layoutGrid, silently dropping the cache-bust.
Risk: S2
Evidence IDs: EVID-0028

DUP-006  —  "hide grid" (flag) vs "close grid" (destroy subprocess) are not distinct APIs
Concept:        There is no explicit hide-without-destroy primitive; the distinction lives implicitly in closeGridAll's behavior + the gridOpen flag.
Where duplicated:
  - src/stores/useBrowserStore.ts:483-511  closeGridAll() (gridOpen=false AND bridge.closeGrid → Rust kill)
  - src/stores/useBrowserStore.ts:513-527  closeGridOne()
  - src/stores/useLayoutStore.ts:210-215  toggleGridToolbar()
  - src-tauri/src/bridge.rs:3915-3944  close_grid() (HideWindow → CloseTab → shutdown_all)
Why dangerous: Adding a "minimize grid to dock" that sets gridOpen=false WITHOUT bridge.closeGrid() keeps each 450 MB child process alive (orphan).
Risk: S4
Evidence IDs: EVID-0007, EVID-0017, EVID-0018, EVID-0019, EVID-0032

DUP-007  —  freeze truth uses two sources; visibility uses one
Concept:        syncFreeze and syncViewVisibility read "is grid visible" differently.
Where duplicated:
  - src/stores/useBrowserStore.ts:619-631  syncFreeze: `if (gridOpen.value){ ... layout.mainView==='grid' ? UNFREEZE : FREEZE }`
  - src/stores/useBrowserStore.ts:646-655  syncViewVisibility: branches ONLY on mainView
  - src/composables/useBrowserHost.ts:64,99,109  schedulePosition/scheduleGrid also gate on BOTH gridOpen && mainView==='grid'
Why dangerous: A model "harmonizing" syncFreeze to read only mainView would freeze grids that are open but not the active view, or vice-versa.
Risk: S3
Evidence IDs: EVID-0011, EVID-0012

DUP-008  —  store tab actions vs bare bridge wrappers (dual entry point)
Concept:        tabClose/tabSwitch/tabNew (store) vs bridge.tabClose/... (bare invoke) are near-identically named; only the store versions do bookkeeping.
Where duplicated:
  - src/stores/useBrowserStore.ts:188-211  tabClose→closeTabNow (recordClose + splice + tabActivate + relocate + syncFreeze)
  - src/bridge.ts:571-594  bridge.tabNew/tabClose/tabActivate/tabOpen/tabPosition (bare invoke)
Why dangerous: A model editing a component may call bridge.tabClose(id) directly, bypassing recentlyClosed recording and the tabs[] splice.
Risk: S3
Evidence IDs: EVID-0026, EVID-0027

DUP-009  —  activateWeb vs isActiveWeb: sibling functions, OPPOSITE correct predicate
Concept:        One must use exact `mainView==="browser"`; the other must use `isBrowserView()`.
Where duplicated:
  - src/components/layout/UnifiedTabBar.vue:147-154  activateWeb() uses `layout.mainView !== "browser"`
  - src/components/layout/UnifiedTabBar.vue:156-158  isActiveWeb() uses `layout.isBrowserView()`
  (activateWeb's own comment warns against using isBrowserView() — see CASE-001.)
Why dangerous: "Refactoring" activateWeb to use isBrowserView() breaks tab display in grid view; "refactoring" isActiveWeb to exact mainView breaks active-tab highlight.
Risk: S4
Evidence IDs: EVID-0003, EVID-0004

────────────────────────────────────────────────────────────────────────────
## Conflict / drift found
- CONFIRMED internal conflict: scripts/check-ui.mjs AND scripts/check-session-persistence-policy.py
  still treat SessionCloseDialog as the [CURRENT] close protocol, while
  scripts/check-native-webview-overlay.mjs asserts it must NOT exist (revoked 2026-09-12,
  useBrowserStore.ts:185-187). Any agent trusting check-ui.mjs's [CURRENT] label would
  reintroduce the revoked component — directly contradicting check-native.
- CONFIRMED checker/source drift: the isBrowserVisible formula asserted by check-grid-close-logic.mjs
  (G2, line 112) and documented in closeGridAll's comment (line 491) no longer matches the source
  computed (useBrowserStore.ts:149-151). A model may "repair" either side wrongly.
- No F-FEATURES.md was present, so Agent F's duplicate findings could not be merged; if produced later,
  its "browser/grid visibility" and "grid close" items overlap DUP-002/006/007 and should be de-duplicated.
- RECOMMENDATION: add a meta-checker that fails when two checkers assert opposite invariants about the
  same symbol, and retire the stale SessionCloseDialog references from check-ui.mjs / check-session-persistence-policy.py.
