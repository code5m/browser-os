# A4 M5-W18-R3 Checkpoint

```text
LANE=A4
DISPATCH=M5-W18-R3-UX (RESEARCH_AND_PROTOTYPE)
CARD=A4 - Shell state and persistence contract
WORKDIR=/home/ainfinit/.codex/worktrees/m5-w18-a4/mvp-browser-os-v3
BRANCH=codex/m5-w18-a4
BASE=origin/master @ 200f0f1 (docs(M5-W18): redirect UX research around Rebased)
REBASE=branch reset to origin/master (dropped 2 prior R2B commits already integrated upstream as
       A4-R2B-backend-evidence-closure.md / A4-dbx-backend-architecture-map.md); now exactly at 200f0f1
STATUS=READY_FOR_REVIEW
MODE=research-only; NO product source / dependency / ACL / capability / native runtime / user-data changes
```

## Deliverables (this lane, research files only)

1. `logs/research/M5-W18/A4-R3-shell-state-persistence-contract.md`
   - Current-state inventory (verified against `useLayoutStore.ts`, `useBrowserStore.ts`,
     `useWorkspaceStore.ts`, `App.vue`, `src-tauri/src/session.rs`).
   - Target state model: `OpenDocument` (unified tab), `ToolWindow` (IDEA-style edge panels),
     `LayoutState`, and the `ShellStateV1` aggregate with **global vs per-workspace** partition.
   - Exact DTOs (TS interfaces) for W19.
   - Migration algorithm from current `modTabs` + `useBrowserStore.tabs`.
   - Crash-safe persistence plan reusing `session::atomic_write` (the project's only atomic primitive).
   - Unknown-view fallback hard rules (no throw / no blank panel on restore).
   - Store ownership refactor (`useShellStore` as sole owner; `useLayoutStore`/`useBrowserStore` yield).
   - Pure-function spec + 15 proposed W19 tests.
2. `logs/research/M5-W18/A4-R3-shell-state-prototype.mjs`
   - Self-contained reference impl of the pure functions (no product import).
   - Runs under `node`, prints `A4-R3 PURE TESTS: 15/15 PASS`.
3. `logs/research/M5-W18/A4-R3-checkpoint.md` (this file).

## Verification

```bash
$ node logs/research/M5-W18/A4-R3-shell-state-prototype.mjs
  ok  T1 migrate empty
  ok  T2 migrate browser+module
  ok  T3 de-dup duplicate url
  ok  T4 unknown main_view -> home
  ok  T5 doc unknown conn -> dropped + active reset
  ok  T6 tool view unavailable -> hidden
  ok  T7 schema too new -> fresh
  ok  T8 corrupt/missing schema -> fresh
  ok  T9 edge invariant: pinned wins
  ok  T10 edge invariant: highest activated wins
  ok  T11 open on empty edge
  ok  T12 open replaces non-pinned primary
  ok  T13 open does NOT replace pinned primary
  ok  T14 collapseAll keeps pinned
  ok  T15 capacity cap drops oldest non-pinned
A4-R3 PURE TESTS: 15/15 PASS
exit=0
```

Worktree state: `git status --short` clean except the three new research files. No other lane
files, no product code, no ACL/capability, no push.

## How this meets the R3 global acceptance constraints (relevant subset)

- **Per-workspace/global boundaries**: `ShellStateV1.global` vs `ShellStateV1.workspaces[id]` partition
  is explicit (§2.4 table). Single-workspace today, schema-ready for multi later.
- **Side/bottom tools: open/close/collapse-all/restore/expand-active/pin/unpin/auto-hide/resize**:
  covered by `ToolWindow` fields + `openToolWindow`/`collapseAll`/`restorePreviousLayout` (geometry
  limits delegated to A3, state machine owned here).
- **Only one primary tool window per edge; opening another replaces it unless pinned**: encoded in
  `openToolWindow` + `enforceEdgeInvariant`, tested T11–T13.
- **No large dashboard / no duplicated navigation**: documents unify the two existing tab lists
  (browser + module) — removes the current dual-tab duplication rather than adding chrome.
- **Self-contained synthetic prototype, no product changes**: satisfied (`.mjs` only, no imports).

## Dependencies / coordination (hand-off, not blocking)

- **A2** density audit: `global.density_pref="auto"` resolves via A2's measured breakpoints
  (already `NAV_DENSITY_FULL_PX=1180` / `NAV_DENSITY_COMPACT_PX=900` in `useLayoutStore`).
- **A3** progressive-disclosure geometry: rides on `ToolWindow.size_px/autoHide/maximized/primary`.
- **A1** consolidated prototype + **A6** command registry: tool-window actions get registry identity.
- **A5/A7/A8** register panel `view` ids into the `ctx.availableToolViews` set consumed by
  `normalizeShellState` for the unknown-view fallback.
- **A9** safety: `beforeunload` flush + dirty-tree guards; A4 guarantees the save primitive is atomic.
