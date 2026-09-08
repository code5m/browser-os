# A4 M5-W18-R3B Persistence Contract Reconciliation

- Lane: **A4**
- Dispatch: `M5-W18-R3B` (`REVISE_TARGETED` wave after A0 `REVISE_TARGETED` verdict)
- Card: `M5-W18-R3B-CORRECTION-TASKS-20260908.md` §5 — *Persistence contract reconciliation*
- Workdir: `/home/ainfinit/.codex/worktrees/m5-w18-a4/mvp-browser-os-v3`
- Branch: `codex/m5-w18-a4`
- Base: `origin/master` @ `d96b9b5` (post R3 integration; A4 rebased onto it; prior R3 commit was already integrated upstream)
- Mode: research-only. **No product source / dependency / ACL / capability / native runtime / user-data changes.**
- Companion pure-test file: `A4-R3B-shell-state-prototype.mjs` (29/29 PASS).

---

## 0. Corrections log (R3 → R3B)

| # | R3 claim (now superseded) | R3B correction | Source |
|---|---|---|---|
| C1 | `collapseAll(toolWindows)` kept pinned windows open (A4-R3 prototype T14) | Collapse All hides **every** tool window including pinned; pinning affects ordinary replacement / auto-hide only. One restorable snapshot is recorded. | A0 ruling (R3B-03): `Collapse All hides every tool window, including pinned windows, and records one restorable layout snapshot.` |
| C2 | Collapse All + Restore were only partially specified; no snapshot durability | `collapsed_snapshot` is a **per-workspace, persisted** slot; `Restore Layout` is deterministic and restores visibility+size+pin. | A0 ruling (R3B-03) + A3 `restorePrevious` |
| C3 | Chrome numbers reused A3's R3 constants (title 32 / top rows 80 / status 26) | A4 adopts the **A0 SSOT**: top chrome 60px (two 30px rows), status bar 24px, 28px activity strip in collapsed mode. Note the live `StatusBar.vue:136` is still `26px` — that is a R3B-05 geometry gap for A2/A3 to close, not a persistence concern. | A0 ruling (R3B-05); `CURRENT_PRODUCT` `src/components/layout/StatusBar.vue:136` |
| C4 | Transient UI state (hover/overlay/drag) not explicitly excluded from persistence | `serializeShellState` strips a fixed `TRANSIENT_KEYS` set, `SESSION_ONLY_KEYS` (focus/focusStack/snapshots), and `SECRET_KEY_RE` credential keys; output is key-order-deterministic. | R3B card §5 ("do not persist temporary hover/overlay state") + A7 privacy precedent |
| C5 | Crash recovery described only as "reuse `session::atomic_write`" | Concretely: atomic tmp+rename write, debounced coalesced save, parse-error → quarantine + `freshState`, corrupt/oversized-schema → `freshState`, no replay of external side effects. | `CURRENT_PRODUCT` `src-tauri/src/session.rs:30 atomic_write`, `:146 save_session`, `:154 load_session`, `:172 list_sessions` (skips corrupt) |

**Cross-lane dependency note:** A3 had not committed an R3B deliverable at the time of writing (`codex/m5-w18-a3` HEAD `0258c5b`, R3 only). A4 therefore reconciles to the **authoritative A0 ruling** and to A3's R3 `restorePrevious`/`collapseAll` shape (A3 `A3-R3-toolwindow-state-machine.mjs`), and explicitly flags the two concrete divergences A3 must fix in its own R3B pass: (i) chrome constants (C3) and (ii) keep tree collapse/expand separate from tool-window Collapse All (A0: "Keep tree collapse/expand behavior separate"). A4 does **not** alter A3's files.

---

## 1. Evidence tags

- `CURRENT_PRODUCT` — `src/stores/useLayoutStore.ts`, `src/stores/useBrowserStore.ts`, `src/stores/useWorkspaceStore.ts`, `src/stores/useSettingsStore.ts` (R2B/ R3 already inventoried); **confirmed `useLayoutStore.ts` holds NO `localStorage` today** → layout is currently in-memory only, i.e. a crash loses the whole layout. This is the gap the persisted `ShellStateV1` closes.
- `CURRENT_PRODUCT` — `src-tauri/src/session.rs:30` `atomic_write` (tmp+rename, the project's only atomic primitive), `:146 save_session`, `:154 load_session` (returns `Err` on missing/corrupt, caller decides), `:172 list_sessions` (skips corrupt files instead of failing the panel).
- `CURRENT_PRODUCT` — `src/components/layout/StatusBar.vue:136` status height `26px` (≠ A0 SSOT 24px; R3B-05, A2/A3 to reconcile).
- `OFFICIAL_DOC` — A0 `logs/checkpoints/A0-M5-W18-R3-acceptance-audit-20260908.md` rulings (REVISE_TARGETED, R3B-01…09).
- `INFERENCE` — where a design choice is A4's, it is labelled and tagged `PROPOSED_NOT_AUTHORIZED` (§7).
- `OBSERVED_BEHAVIOR` — A3 `A3-R3-toolwindow-state-machine.mjs` `collapseAll`/`restorePrevious` semantics (58 assertions PASS) as the reference shape.

---

## 2. State model (carried from R3, refined)

`ShellStateV1` aggregate:

```
{
  schema_version: 1,
  saved_at_ms: number,                 // set by caller at save time; EXCLUDED from equality
  global: {                            // GLOBAL ownership (shared across workspaces)
    theme, focus_mode_default, density_pref, keymap_scheme, addr_mode_default,
    sidebar_default_open, window: { x, y, w, h, maximized }
  },
  active_workspace_id: string,
  workspaces: {
    [id]: {
      documents: [{ id, kind, ref, title, meta?, pinned? }],
      active_document_id,
      tool_windows: [{ id, edge, view, state, primary, pinned, autoHide, maximized, size_px, last_activated_ms }],
      layout: { main_view, nav_section, density, collapsed, focus_mode, addr_mode, sidebar_open, sidebar_width },
      splitters: { left, right, bottom },
      collapsed_snapshot: null | { tool_windows, layout, splitters, main_view }
    }
  }
}
```

### 2.1 Workspace vs global ownership (R3B explicit)

| Field | Owner | Persist? | Rationale |
|---|---|---|---|
| `global.theme`/`density_pref`/`keymap_scheme`/`addr_mode_default`/`sidebar_default_open` | global (app) | yes | user-wide preference, applies to every workspace |
| `global.window` (x/y/w/h/maximized) | global (app window) | yes | window geometry is per-app, not per-workspace |
| `workspaces[id].documents` | per-workspace | yes | open tabs are a workspace concern |
| `workspaces[id].tool_windows` | per-workspace | yes (state/visibility/pin/size) | panel layout per workspace |
| `workspaces[id].layout` (main_view, collapsed, focus_mode, nav_section, …) | per-workspace | yes | active surface + collapse flag per workspace |
| `workspaces[id].splitters` | per-workspace | yes | panel widths per workspace |
| `workspaces[id].collapsed_snapshot` | per-workspace | yes | the one restorable snapshot must survive restart |
| transient `focus`/`focusStack` | session-only | **no** | restored to active document surface on load |
| `snapshots` (expand-active/focus-mode undo LIFO, max 3) | session-only | **no** | transient undo stack, never replayed after restart |
| DB connection credentials | registry (`keyring`) | **never in JSON** | A6/B-secret: credentials stay in `db:<conn_id>` keyring, not the layout file |

### 2.2 Unknown-window fallback (hard rules, no throw / no blank panel)

1. `main_view` not in `availableViews` → reset to `home`, record `{reason:"unknown-view"}`.
2. Document `kind==="db-console"` whose `connId` not in `availableConnIds` → drop document, reset `active_document_id`; record `{reason:"unknown-conn"}`.
3. Tool window `view` not in current registry:
   - if currently open → set `state:"hidden"`, `primary:false`, record `{reason:"unknown-tool-view"}`;
   - if already hidden → drop from the persisted list, record reason.
   - **Pruned from `collapsed_snapshot` too**, so `Restore Layout` can never resurrect a ghost (proven by test T25).
4. `schema_version > MAX` or missing/unparseable → `freshState()` (quarantine corrupt file, see §3).
5. `collapsed_snapshot` present but its inner `tool_windows` contain an unknown view → pruned (same rule as #3).

---

## 3. Snapshot lifecycle & crash recovery

### 3.1 Snapshot lifecycle
- **Write:** `collapseAllWorkspace` pushes exactly one `collapsed_snapshot` (deep copy of pre-collapse `tool_windows`/ `layout`/ `splitters`/ `main_view`). A second Collapse All **overwrites** it; an already-collapsed state (nothing open) is a **no-op** that preserves the existing snapshot (test T17 — must not overwrite with an empty state).
- **Restore:** `Restore Layout` reads `collapsed_snapshot` and rebuilds visibility/size/pin; then **clears** the slot (`null`). Empty slot → documented default layout (files tool open on the left).
- **Capacity:** the dedicated `collapsed_snapshot` is a single slot (A0: "one restorable layout snapshot"). The separate A3 `snapshots` undo stack (expand-active / focus-mode) is session-only, capped at 3, and is **not** persisted.

### 3.2 Crash recovery algorithm (W19 port, 1:1 from prototype)
1. **Serialize** `serializeShellState(state)` → pretty JSON (key-order deterministic, transient/secret/session stripped).
2. **Save** `session::atomic_write(path, json)` (tmp file + atomic rename; mirrors `save_session`). Caller debounces (last-write-wins, `MIN_SAVE_INTERVAL_MS` proposed 250ms) and coalesces rapid layout changes.
3. **Load** `loadShellState(raw, ctx)`:
   - file missing / unparseable → quarantine `<name>.corrupt-<ts>.json`, return `freshState()` + `{quarantined:true}`; **never throw**.
   - `schema_version > MAX` → `freshState()` + `{reason:"schema-too-new"}`.
   - otherwise `normalizeShellState` (unknown-view fallback, capacity cap, edge invariant, snapshot pruning).
4. **No replay of external side effects:** the persisted blob is declarative only. DB connection refs are ids resolved at runtime against `keyring`; restoring a `db` document does **not** re-execute any query. Restart never re-runs a query (mirrors A6 "restart does not replay execution").
5. **Corrupt-window isolation:** a single bad workspace file never blanks the app — `list_sessions` precedent skips corrupt files (test T26 proven at the shell-state layer).

### 3.3 Determinism proof (R3B card "prove Collapse All and Restore Layout are deterministic")
- Every transition is a **pure function** (clones input, no `Date.now` inside; caller supplies `nowMs`). No `Math.random`, no wall-clock inside `collapseAllWorkspace`/`restoreWorkspaceLayout`/`normalizeShellState`.
- **Idempotent:** `collapseAllWorkspace(collapseAllWorkspace(ws))` is byte-identical to a single call for the persisted snapshot (T17).
- **Round-trip stable:** `normalize(normalize(raw))` yields identical serialized state (T24).
- **Key-order independent:** `serializeShellState` sorts object keys and canonicalizes `tool_windows` array order by `id`, so two equivalent states built with different key/array insertion order serialize identically (T28).

---

## 4. Transient-state exclusion (R3B card "do not persist temporary hover/overlay state")

`TRANSIENT_KEYS = {hover_id, overlay, drag_preview, context_menu, tooltip, resize_active, splitter_drag, focus_ring}`
`SESSION_ONLY_KEYS = {focus, focusStack, snapshots}`
`SECRET_KEY_RE = /(token|password|secret|passwd|api[_-]?key)/i`

`serializeShellState` removes all of the above recursively and **strips** any string-valued secret key. Proof (T21/T22/T23): a state seeded with `hover_id`/`overlay`/`drag_preview`/`tooltip`/`focus`/`focusStack`/`snapshots`/a `db_password` meta serializes **identically** to the same state without those keys, and the secret value never appears in the output.

---

## 5. Geometry (A0 SSOT, informational for the persistence layer)

`collapsedShares(vw,vh) = { w:(vw-28)/vw, h:(vh-60-24)/vh }`. A4 only persists `layout.collapsed` and `splitters`; the activity-strip/chrome/status constants live in the geometry layer (A2/A3 R3B). At `900x600` the collapsed height share is `(600-84)/600 = 86%` (≥85%), width `(900-28)/900 = 96.9%` (T29, formula deterministic).

---

## 6. Verification

```bash
$ node logs/research/M5-W18/A4-R3B-shell-state-prototype.mjs
  ... 29 ok lines ...
A4-R3B PURE TESTS: 29/29 PASS
exit=0
```

Unrun / NOT_RUN: real Tauri desktop persistence round-trip (needs the native client); A11/A9 native-GUI evidence reserved for A0/user review (R3B-09). The pure model is fully exercised; no native call is made.

---

## 7. Candidate implementation card (PROPOSED_NOT_AUTHORIZED)

**Scope (W19, single slice S0-shell-state):** add a `useShellStore` as the sole owner of `ShellStateV1`; yield layout responsibilities from `useLayoutStore`/`useBrowserStore`. Add `src-tauri/src/commands` `save_shell_state` / `load_shell_state` backed by `session::atomic_write`; frontend debounced-save on layout mutation; `beforeunload` flush (A9 safety hook). Reuse `session::atomic_write` + the corrupt-skip pattern from `list_sessions`.

**Preconditions:** A3 R3B closes chrome-constant divergence (C3) and tree/tool-window Collapse All separation; A6 delivers credential handling so `db:<conn_id>` keyring is the only credential source.

**Tests:** port the 29 pure assertions into `vitest`; add a `scripts/check-shell-persistence-logic.mjs` (policy gate, like A4's existing pattern) that loads the real store + bridge stub. Add a corrupt-file integration test driving `load_shell_state` through the Tauri command.

**Completion criteria:** 29/29 pure tests + policy gate PASS; `npm run build` exit 0; `git diff --check` clean; no credential reaches disk (T23 gate).

**Not authorized:** this lane does not write any of the above product code; card is for A0/W19 coding dispatch.
