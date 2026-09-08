# A4 · R3 Shell State & Persistence Contract

> **R3B SUPERSEDED (2026-09-08):** This R3 report's collapse/restore semantics are overridden by
> `A4-R3B-persistence-contract.md` per the A0 R3 acceptance ruling (R3B-03). In particular the
> claim "Collapse All keeps pinned windows open" and the per-workspace snapshot lifecycle /
> crash recovery / transient-state exclusion are corrected there. The companion `A4-R3-shell-state-prototype.mjs`
> still runs 15/15 but its **T14 is VOID** (it asserts the pre-R3B behavior); the authoritative suite is
> `A4-R3B-shell-state-prototype.mjs` (29/29 PASS).

```text
LANE=A4
DISPATCH=M5-W18-R3-UX (RESEARCH_AND_PROTOTYPE)
TASK=CARD "A4 - Shell state and persistence contract"
WORKDIR=/home/ainfinit/.codex/worktrees/m5-w18-a4/mvp-browser-os-v3
BRANCH=codex/m5-w18-a4
BASE=origin/master @ 200f0f1 (docs(M5-W18): redirect UX research around Rebased)
MODE=research-only; no product source / ACL / capability / native runtime / user-data changes
REFERENCE_PROTOTYPE=logs/research/M5-W18/A4-R3-shell-state-prototype.mjs (self-contained, runs under `node`)
STATUS=READY_FOR_REVIEW
```

## 0. Scope & non-goals

This card owns **only** the unified shell-state *model* and its *crash-safe persistence
contract*. It does **not** own:

- visual density numbers / before-after budgets → **A2** (current-shell density audit)
- tool-window edge geometry, splitter limits, tree collapse-all/expand-one-level/bounded-expand-all,
  keyboard focus, focus-return → **A3** (IDEA progressive-disclosure behavior)
- context-menu / command-registry identity for every action → **A6**
- database / Git / graph placement inside the shell → **A5 / A7 / A8**
- viewport measurement frames / "大气" visual system → **A8**
- interaction safety (dirty-tree, protected-branch, undo) → **A9**

A4 proposes the **data structures, ownership boundaries, migration path, persistence primitive,
unknown-view fallback, and the pure functions** that the other lanes' behavior rides on. The pure
functions in §7 are the contract surface; A4 ships a working reference implementation + tests in
`A4-R3-shell-state-prototype.mjs` so W19 can port them 1:1 into `src/stores`/`scripts`.

## 1. Current-state inventory (verified, not assumed)

Read from `src/stores/useLayoutStore.ts`, `src/stores/useBrowserStore.ts`,
`src/stores/useWorkspaceStore.ts`, `src/App.vue`, and `src-tauri/src/session.rs`.

| Concern | Today | Persisted? | Mechanism |
|---|---|---|---|
| Active module view `mainView` (27 `MainView` variants) | `useLayoutStore.mainView` | **No** | in-memory only |
| Module tabs `modTabs: ModTab[]` + `activeModTab` | `useLayoutStore` | **No** | in-memory only |
| Browser tabs `tabs: TabInfo[]` (`{id,url,title}`) + `activeTabId` | `useBrowserStore` | **No** (live list) | in-memory; only the *archived* session (closed-tab resources) hits disk via `session.rs` |
| Sidebar `sidebarOpen`/`sidebarWidth` | `useLayoutStore` | **No** | in-memory |
| Right dock `browserDockOpen`/`browserDockTab` (`files|term|net|session`) | `useLayoutStore` | **No** | in-memory (single hardcoded right dock) |
| `compactMode`, `addrMode`, `navSection`, `windowWidth` | `useLayoutStore` | **No** | in-memory |
| Recents (url/file) | `useWorkspaceStore.recents` | **Yes** | `localStorage["browser-os-recents"]` (last 30) |
| Clipboard history | was `localStorage` | **Removed** | A6 BUG-HUNT → in-memory only (B11-1 red-line) |

**Key gaps this contract closes**

1. **No shell-state durability.** Reload loses every open tab, the active view, dock/open-panel
   state, sidebar width, compact mode. Users re-build their workspace every launch.
2. **Two parallel "tab" lists.** `modTabs` (module views) and `useBrowserStore.tabs` (web pages)
   are disjoint data structures with overlapping UX (both render as tabs in a strip). R3 must unify
   them into one document model.
3. **No per-workspace boundary.** Everything is global to the app process; there is no concept of
   "this set of open documents belongs to workspace X".
4. **No unknown-view fallback.** If a persisted view id disappears (plugin removed, db-connection
   gone, feature flag off), naive restore would show a blank/empty panel or throw.
5. **localStorage is the only durable store for shell data**, and it is *not* the project's
   crash-safe primitive (Rust `session::atomic_write` is). Secrets red-line (B11-1) already pushed
   clipboard out of localStorage; shell state should follow the same trusted path.

## 2. Target state model

Three layers, each owned by one store (see §8):

```
ShellStateV1
 ├─ schema_version: 1
 ├─ saved_at_ms: number
 ├─ global:  GlobalShellState     ← shared across workspaces
 ├─ active_workspace_id: string
 └─ workspaces: Record<string, WorkspaceShellState>   ← one per workspace
```

### 2.1 Document (unified "tab")

```ts
type DocumentKind = "browser" | "module" | "editor" | "db-console" | "git" | "graph-node" | "note";

interface OpenDocument {
  id: string;            // stable uuid
  kind: DocumentKind;
  ref: string;           // browser:url | module:<MainView> | editor:<path> | db-console:<connId> | ...
  title: string;
  pinned?: boolean;      // pinned docs survive "close others" / workspace switch
  meta?: Record<string, unknown>;  // e.g. {connId, db, table} for db-console; {path} for editor
}
```

Migration maps `useBrowserStore.tabs` → `kind:"browser", ref:"browser:<url>"`, and
`useLayoutStore.modTabs` → `kind:"module", ref:"module:<view>"`. See §4.

### 2.2 Tool window (IDEA-style, new)

```ts
type Edge = "left" | "right" | "bottom";
type ToolWindowState = "open" | "stashed" | "hidden";   // stashed = button on edge strip, not expanded

interface ToolWindow {
  id: string;            // stable, e.g. "files", "terminal", "git-log", "problems", "db-tree"
  edge: Edge;
  view: string;          // which panel renders when open
  state: ToolWindowState;
  primary: boolean;      // exactly one `primary:true` per edge (invariant, enforced §7.3)
  pinned: boolean;       // excluded from collapseAll / autoHide / closeAll
  autoHide: boolean;
  maximized: boolean;    // open + takes full edge zone (focus on one tool)
  size_px: number;       // splitter size, clamped per edge
  last_activated_ms: number;
}
```

Operations (signatures only; A3 owns geometry/limits, A4 owns the state transition purity):
`openToolWindow`, `closeToolWindow`, `collapseAll(edge|global)`, `restorePreviousLayout`,
`expandActive(edge)`, `pin/unpin`, `setAutoHide`, `resize(edge, px)`.

### 2.3 Layout (top chrome + density + focus)

```ts
interface LayoutState {
  main_view: string;        // the "home/base" view when no document dock is foregrounded
  nav_section: "" | "grid" | "more" | "omni";
  density: "full" | "compact" | "icon";   // derived from windowWidth (already in useLayoutStore)
  compact_mode: boolean;    // immersive browser (hide addr+tab bars)
  addr_mode: "url" | "dir";
  sidebar_open: boolean;
  sidebar_width: number;    // clamp 180..560 (today) — keep
  focus_mode: boolean;      // hide ALL tool chrome, one return action (Esc / dedicated button)
}
```

### 2.4 Per-workspace vs global boundary (the partition)

| Field | Boundary | Rationale |
|---|---|---|
| `global.density`, `global.theme`, `global.focus_mode_default`, `global.sidebar_open/width`, `global.addr_mode`, `global.compact_mode` | **global** | user-chrome preference, independent of what they are working on |
| `workspaces[id].documents` + `active_document_id` | **per-workspace** | open pages belong to a task/workspace |
| `workspaces[id].tool_windows` | **per-workspace** | which panels are open for this workspace |
| `workspaces[id].layout.main_view/nav_section` | **per-workspace** | the foreground base view per workspace |
| `workspaces[id].splitters` (`{left,right,bottom}`) | **per-workspace** | panel sizes per workspace |

Today there is a single implicit workspace; the model seeds `workspaces["default"]` and is ready for
multi-workspace later without a schema change (just add keys to the `workspaces` map). This honors
R3's "per-workspace/global boundaries" without over-building a workspace switcher.

## 3. DTO (exact shape proposed for W19)

```ts
// logs/research/M5-W18/A4-R3-shell-state-prototype.mjs carries a JS port of these.
interface ShellStateV1 {
  schema_version: 1;
  saved_at_ms: number;
  global: GlobalShellState;
  active_workspace_id: string;
  workspaces: Record<string, WorkspaceShellState>;
}

interface GlobalShellState {
  theme: "light" | "dark";
  focus_mode_default: boolean;
  density_pref: "full" | "compact" | "icon" | "auto"; // "auto" = derive from windowWidth
  sidebar_open: boolean;
  sidebar_width: number;
  addr_mode: "url" | "dir";
  compact_mode: boolean;
}

interface WorkspaceShellState {
  documents: OpenDocument[];
  active_document_id: string | null;
  tool_windows: ToolWindow[];
  layout: LayoutState;
  splitters: { left: number; right: number; bottom: number };
}
```

## 4. Migration from current tabs

`migrateFromLegacy(legacy, opts) -> { documents, active_document_id }`

Input (read-only snapshots of today's stores):
- `legacy.modTabs: {id, view, title, path?}[]`
- `legacy.browserTabs: {id, url, title}[]`
- `legacy.activeModTab: string` and `legacy.activeTabId: string`
- `legacy.mainView: string`

Algorithm:
1. Browser tabs → `OpenDocument{kind:"browser", ref:"browser:"+url, title}`.
2. Module tabs → `OpenDocument{kind:"module", ref:"module:"+view, title}` (preserve `path` into `meta.path`).
3. If `legacy.activeTabId` set → `active_document_id` = that browser doc id; else if `legacy.activeModTab`
   set → that module doc id; else `null`.
4. De-duplicate by `ref` (same url / same module view collapses to one doc) — mirrors today's
   `openModule`/`openDirTab` same-view de-dup.
5. Preserve `pinned` where the source already marks it (today none, so default `false`).

Edge cases covered by tests (§7.6): empty input → `documents:[]`, `active:null`; duplicate urls
collapse; a module tab whose `view` is no longer a valid `MainView` is dropped *during migration*
(because migration runs before normalize, the unknown-view rule in §5 still applies defensively).

## 5. Unknown-view fallback (hard rule)

`normalizeShellState(raw, ctx) -> { state: ShellStateV1, dropped: DroppedRef[] }`

`ctx` supplies the *currently available* view/connection sets (from a registry owned by A1/A5/A7
for their respective panels). For every persisted reference that is not in `ctx.available`:

- `document.ref` whose kind/view/connId is unavailable → **drop the document**, record
  `{reason:"unknown-view"|"unknown-conn"|"feature-off", ref}`.
- `workspace.layout.main_view` not in `ctx.availableViews` → reset to `"home"`.
- `tool_windows[].view` not in `ctx.availableToolViews` → set that window `state:"hidden"`
  (kept in the list so a future enabling can restore it, but never rendered open).
- `active_document_id` pointing to a dropped doc → reset to first remaining doc id or `null`.
- `schema_version` mismatch (> supported) → **discard entire file**, return fresh default state
  (no partial apply). `schema_version` < supported → run `migrate()` chain if present, else default.
- JSON parse error / IO error → discard, return fresh default (the previous good file is untouched
  because persistence is atomic — see §6).

No exception escapes normalize; the UI never shows a blank panel or throws on restore.

## 6. Crash-safe persistence

Reuse the **only** atomic primitive: `crate::session::atomic_write` (tmp + rename) already used by
`session.rs`, `workspace.rs`, `tasks.rs`, `script_runner.rs`, `plugin.rs`, `images.rs`.

Proposed new native commands (W19, NOT this lane):
- `shell_state_save(state: ShellStateV1) -> Result<(), String>`
  → `session::atomic_write(shell_state_path(), serde_json::to_string_pretty(state)?)`.
- `shell_state_load() -> Option<ShellStateV1>`
  → read file; on missing/corrupt → `None` (frontend then calls `normalizeShellState` with fresh default).

Frontend discipline:
- **Debounce** saves 400–600 ms (coalesce rapid tab/resize churn); write only on idle + on
  `beforeunload`/`tauri::ExitRequested` (best-effort synchronous flush via a small final write).
- **Single writer**: only `useShellStore` serializes the aggregate. No other store writes the file.
- **No localStorage for shell state** (secrets red-line consistency with B11-1; localStorage is also
  not crash-safe). Recents may stay in localStorage *or* move into `workspaces[id]` — out of scope,
  noted as a follow-up.
- **Capacity**: cap `documents` (e.g. 200) and `tool_windows` (e.g. 32); drop oldest non-pinned if
  exceeded (mirrors `SESSION_MAX_*` pattern in `session.rs`).

## 7. Pure state functions (contract surface + reference impl)

All pure, no `bridge`/DOM, Node-loadable (the prototype imports nothing from the app).

### 7.1 `migrateFromLegacy(legacy)` → `{documents, active_document_id}`
§4.

### 7.2 `normalizeShellState(raw, ctx)` → `{state, dropped}`
§5. Also enforces the per-edge `primary` invariant (§7.3) and value clamps.

### 7.3 `enforceEdgeInvariant(toolWindows)` → `ToolWindow[]`
Per edge, at most one `primary:true`. Tie-break: keep the **pinned** one; if multiple pinned, keep the
highest `last_activated_ms`; the rest forced `primary:false` (stashed). Returns a new array.

### 7.4 `openToolWindow(state, edge, id, opts?)` → `ToolWindow[]`
Resolves the "one primary per edge, replace unless pinned" rule:
- Find current `primary` window on `edge`, call it `P`.
- If `P` exists, is **pinned**, and `P.id !== id` → `P` stays primary; `id` is opened as
  `primary:false` but `state:"stashed"` (available on the edge strip, not expanded). *This is the
  "replaces it unless pinned" interpretation A4 adopts and recommends; it keeps the pinned window
  foreground while still letting the user reach the new one.*
- Else → all windows on `edge` → `primary:false`; `id` → `primary:true, state:"open"`.
Returns a new array (immutable update).

### 7.5 `collapseAll(toolWindows, scope)` / `restorePreviousLayout(snapshots, ...)` 
- `collapseAll` sets every tool window, including `pinned`, to `state:"hidden"` (per A0 global acceptance).
- `restorePreviousLayout` pops a bounded (≤10) snapshot stack of `tool_windows` arrays kept in memory
  (not persisted as a separate file; it lives inside `ShellStateV1.workspaces[id]` as
  `layout_snapshots?: ToolWindow[][]` — optional, capped).

### 7.6 Pure tests proposed for W19 (all encoded + run in the prototype)

| # | Function | Assertion |
|---|---|---|
| T1 | migrateFromLegacy | empty input → `documents:[]`, `active:null` |
| T2 | migrateFromLegacy | 2 browser tabs + 1 module tab → 3 docs, kinds correct |
| T3 | migrateFromLegacy | duplicate url collapses to 1 doc |
| T4 | normalizeShellState | unknown `main_view` → reset to `"home"` + dropped recorded |
| T5 | normalizeShellState | doc ref to missing connId → dropped, active reset |
| T6 | normalizeShellState | tool window `view` unavailable → `state:"hidden"` |
| T7 | normalizeShellState | `schema_version` too new → fresh default, no apply |
| T8 | normalizeShellState | corrupt JSON path (simulated) → fresh default |
| T9 | enforceEdgeInvariant | two `primary` on same edge → one remains, tie-break pinned |
| T10 | enforceEdgeInvariant | non-pinned dup primary → highest `last_activated_ms` wins |
| T11 | openToolWindow | open on empty edge → that window primary |
| T12 | openToolWindow | open replaces non-pinned primary on edge |
| T13 | openToolWindow | open does NOT replace pinned primary (stays primary, new stashed) |
| T14 | collapseAll | every tool window hidden, pinned included |
| T15 | capacityCap | >200 docs → oldest non-pinned dropped |

The prototype runs all 15 and prints `A4-R3 PURE TESTS: 15/15 PASS`.

## 8. Store ownership (W19 proposed refactor)

| Store | After R3 | Notes |
|---|---|---|
| `useShellStore` **(NEW)** | **Sole owner** of `ShellStateV1`. Holds `documents`, `tool_windows`, `layout`, `global`, `workspaces`. Exposes the pure functions. Writes via `shell_state_save` (debounced). | Replaces the shell-state *fields* now scattered in `useLayoutStore` + `useBrowserStore.tabs`. |
| `useLayoutStore` | Becomes a **thin read accessor** delegating to `useShellStore` for `mainView`/`navSection`/`density`/`sidebar`. Keeps pure nav helpers (already policy-tested by `check-client-navigation-logic.mjs`). | No new persistence here. |
| `useBrowserStore` | Keeps webview lifecycle (`tabGoBack`, grid, AI adapters) but **yields `tabs`/`activeTabId`** to `useShellStore.documents` (`kind:"browser"`). | Avoids duplicated tab ownership. |
| `useWorkspaceStore` | Keeps artifact/repo/file CRUD; may later move `recents` into the shell state (follow-up, not this lane). | |
| `App.vue` | On mount: `await bridge.shell_state_load()` → `normalizeShellState` → hydrate `useShellStore`; subscribe to changes → debounced save. Replaces the current `loadRecents()`-only restore. | |

Single-writer principle prevents the classic "two stores both persist, last-write-wins clobber" bug.

## 9. Dependencies / coordination

- **A2** density audit: `global.density_pref="auto"` must resolve via A2's measured breakpoints
  (`NAV_DENSITY_FULL_PX=1180`, `NAV_DENSITY_COMPACT_PX=900` already in `useLayoutStore`).
- **A3** progressive disclosure: owns splitter min/max, auto-hide timing, focus-return; A4's
  `ToolWindow` fields (`size_px`, `autoHide`, `maximized`, `primary`) are the data A3's geometry rides on.
- **A6** command registry: every tool-window action (open/collapse/pin/...) gets a command-registry
  identity + disabled-reason; A4 provides the state mutation, A6 the registry entry.
- **A5/A7/A8** register their panel `view` ids into the `ctx.availableToolViews` set that
  `normalizeShellState` consumes for the unknown-view fallback.
- **A9** safety: `beforeunload` flush + dirty-tree guards for editors belong to A9; A4 only guarantees
  the *save primitive* is atomic and that a mid-write crash leaves the prior good file.

## 10. Deliverables in this lane

- `logs/research/M5-W18/A4-R3-shell-state-persistence-contract.md` (this file)
- `logs/research/M5-W18/A4-R3-shell-state-prototype.mjs` (self-contained pure impl + 15 tests, runs on `node`)
- `logs/research/M5-W18/A4-R3-checkpoint.md`

No product code, no ACL/capability, no other lane files, no push.
