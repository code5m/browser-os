# M5-W18-R3B · A6 — Command Registry, Shortcuts & Accessibility (Closure)

> LANE=A6 · Mode=`RESEARCH_AND_PROTOTYPE` · Wave=`M5-W18-R3B` (correction of R3)
> Deliverable owner: A6. Consumed by: A1 (consolidated prototype), A9 (safety review), A10 (provenance/audit), A11 (acceptance).
> **No product source, manifest, ACL, capability, native runtime or user-data changed. Research artifact only.**

```text
LANE=A6
STATUS=PASS
BASE=200f0f1cc032eb7dbd0229ab53a711a5ff1e3d6f
HEAD=<set on commit>
DISPATCH=M5-W18-R3B
REFERENCE_EVIDENCE=CURRENT_PRODUCT (file:line below, read from 200f0f1); REFERENCE_SOURCE (DetachHead/rebased Git action system, JetBrains IDEA action system, Obsidian note/graph menus); OFFICIAL_DOC (M5-W18-R3B-CORRECTION-TASKS-20260908.md, A0-M5-W18-R3-acceptance-audit-20260908.md)
FILES=logs/research/M5-W18/A6-R3B-command-registry-closure.md, logs/research/M5-W18/A6-R3B-registry.mjs, logs/research/M5-W18/A6-R3B-registry-audit.mjs, logs/research/M5-W18/A6-R3B-checkpoint.md
SOURCE_MAP=see §8
CLASSIFICATION=COPY=0, ADAPT=4 (Git ADAPT_PRODUCT) + REIMPLEMENT (registry/shortcuts/palette/14-Git units); see §8
VERIFY=node logs/research/M5-W18/A6-R3B-registry-audit.mjs  ->  RESULT: PASS, EXIT 0
CHECKPOINT=logs/research/M5-W18/A6-R3B-checkpoint.md
MERGE_NOTES=A1 consumes the registry for menu/palette wiring; A9 reconciles T2/T3 + audit class; A10 verifies provenance; A11 reports gateway.
NEXT=A0 integrates; W19 S1 (shell+registry bootstrap) + S2/S3/S4 (domain context menus) implement per this contract.
```

---

## 0. Reconciliation note (dispatch vs. prompt)

The task prompt named `M5-W18-R2B-TASKS-20260908.md` and asked to read the **latest task card**. Per `WORKSPACE_IDENTITY.md` and `PARALLEL_COMMAND_BOARD.md`, the current dispatch overrides historical NEXT and is **`NEXT=M5-W18-R3B`**; the latest task card is therefore `M5-W18-R3B-CORRECTION-TASKS-20260908.md`. Its Lane A6 assignment (*"Command registry, shortcuts and accessibility"*) is the authoritative scope performed here. It **completes and corrects** the prior A6 R3 deliverable (`A6-R3-context-menu-command-registry.md`, `A6-R3-checkpoint.md`), specifically closing former debt **D1** (Git ids) and the A0 audit findings **R3B-04** (accessibility) and **R3B-06** (shortcut conflict).

---

## 1. Executive summary

R3B A6 must: (a) freeze `Ctrl+K` / `Ctrl+Shift+P`; (b) give every shell, database, knowledge and **all 14 Git workflow** action a stable id, scoped menu placement, disabled reason, keyboard/accessible path and safety class; (c) provide a **machine-checkable registry audit**; (d) forbid permanent toolbar-button expansion.

This closure delivers:

1. A **frozen shortcut map** resolving R3B-06: `Ctrl+K → browser.focusAddressSearch`, `Ctrl+Shift+P → app.openCommandPalette`. No other command may claim either chord; no duplicate chords exist.
2. A **complete synthetic registry** of **107 commands** (shell 44, database 16, knowledge 15, git 32), each with a scoped (object-typed) menu placement, a disabled-reason hook, a keyboard/accessible path, and a safety/audit/confirm classification.
3. **All 14 Git workflow units** (A0 ruling 43) with stable ids and full fields — closing former D1. The Git classification is `COPY=0`, `ADAPT_PRODUCT=4` (status, diff, branches, merge), `REIMPLEMENT_FROM_BEHAVIOR=10` (hunk staging, log graph, worktrees, stash, rebase/I-rebase, cherry-pick, conflicts, history/blame, patch, command log). The explicitly required visible units (amend, reset/revert, conflicts, patch, command log, worktrees) are all present as registry rows.
4. An **accessibility contract** (R3B-04): every menu action is a real `role=menuitem` element with `aria-disabled` when disabled, a `title` carrying the disabled reason, and deterministic focus return to the origin on `Esc`; `↑/↓` navigate, `Enter` executes.
5. A **machine-checkable audit** (`A6-R3B-registry-audit.mjs`) that mechanically verifies every R3B A6 invariant and exits non-zero on any violation. It currently reports **PASS / exit 0**.
6. A **no-toolbar-expansion rule**: every command is reachable via menu or palette or shortcut; the synthetic model carries `toolbarOnly:false` everywhere and the audit rejects any `toolbarOnly:true`.

---

## 2. Frozen shortcuts (A0 ruling 41, closes R3B-06)

| Chord | Command id | Scope | Behavior |
|---|---|---|---|
| `Ctrl+K` | `browser.focusAddressSearch` | `browser.page` | Focus the browser address / search field (the existing browser-first search entry). |
| `Ctrl+Shift+P` | `app.openCommandPalette` | `app` | Open the command palette (new searchable command entry). |

Guarantees enforced by the audit:
- Only these two chords are reserved globally.
- Each frozen chord maps to exactly its designated command.
- No other command declares `Ctrl+K` or `Ctrl+Shift+P`.
- All `command.shortcut` values are unique (no chord collision).

This directly resolves the R3B-06 finding (*"Ctrl+K and Ctrl+Shift+P are both used as command entry"*): one chord is address/search, the other is the palette — distinct, frozen roles.

---

## 3. Registry scope coverage

| Group | Scope(s) | Count | Notes |
|---|---|---|---|
| Shell · browser | `browser.tab`, `browser.page` | 15 | includes frozen `browser.focusAddressSearch` |
| Shell · workspace | `workspace.file`, `workspace.tree` | 16 | file CRUD + bounded tree expand/collapse (`expandAllBounded` hard-capped; unbounded `expandAll` rejected, per R3 §6) |
| Shell · terminal | `terminal` | 7 | `terminal.pane` scoped menu |
| Shell · tool window | `toolwindow` | 8 | `tw.collapseAll` / `tw.restoreLayout` honor A3 collapse/restore ruling |
| Database | `db.connection`, `db.schema`, `db.table`, `db.cell`, `db.result` | 16 | `db.result.cancel` maps to A4/A6 cancellation contract; `truncate`/`drop` = T3 dangerous |
| Knowledge | `note`, `link`, `graph.node` | 15 | consumes A2/A3 note/link/graph semantics |
| Git | `git.repo`, `git.branch`, `git.commit`, `git.file`, `git.hunk` | 32 | 14 canonical units + families + amend/reset/revert |
| App | `app` | 1 | `app.openCommandPalette` (frozen) |

Every command is emitted from one factory (`cmd(...)`) so the invariants in §5 cannot regress silently.

---

## 4. Git 14-unit closure (closes former D1; A0 ruling 43)

| # | Unit (A0 classification) | Class | Primary command id | safetyClass |
|---|---|---|---|---|
| 1 | status | ADAPT_PRODUCT | `git.status` | safe |
| 2 | diff | ADAPT_PRODUCT | `git.diff` | safe |
| 3 | branches | ADAPT_PRODUCT | `git.branches` | safe |
| 4 | merge | ADAPT_PRODUCT | `git.merge` | mutating (T2) |
| 5 | hunk staging | REIMPLEMENT | `git.hunkStage` | mutating (T1) |
| 6 | log graph | REIMPLEMENT | `git.logGraph` | safe |
| 7 | worktrees | REIMPLEMENT | `git.worktree` | safe |
| 8 | stash | REIMPLEMENT | `git.stash` | mutating (T1) |
| 9 | rebase / I-rebase | REIMPLEMENT | `git.rebase` | destructive (T3) |
| 10 | cherry-pick | REIMPLEMENT | `git.cherryPick` | mutating (T1) |
| 11 | conflicts | REIMPLEMENT | `git.conflicts` | mutating (T2) |
| 12 | history / blame | REIMPLEMENT | `git.history` | safe |
| 13 | patch | REIMPLEMENT | `git.patch` | safe |
| 14 | command log | REIMPLEMENT | `git.commandLog` | safe |

Explicitly required visible units also present: `git.worktreeNew`, `git.commitAmend`, `git.commitReset`, `git.commitRevert`, `git.conflictResolve`, `git.patchApply`. Every Git command carries a disabled-reason token (`dirty-tree`, `protected-branch`, `no-stash`, `not-rebasing`, …) so the UI can grey it with an explanatory `title`, and is reachable via menu + palette (advanced actions are menu/palette-only, never permanent buttons — satisfies "advanced actions belong in menus/palette rather than permanent buttons").

---

## 5. Per-command invariant contract (what the audit checks)

For every command the registry guarantees, and the audit asserts:

- `id` — stable, dotted, unique across the whole registry.
- `scope` ∈ enumerated taxonomy; `menu` is **scoped** (`<scope>.<context>`, e.g. `git.hunk`, `terminal.pane`, `note.editor`) — never a catch-all.
- `disabledReason` — `null` (always enabled) or a stable reason token (drives `aria-disabled` + `title`).
- `reachable` — non-empty subset of `{shortcut, palette, menu, keyboard}`; **never toolbar-only**.
- `safetyClass` ∈ `{safe, mutating, destructive, dangerous}`; `auditClass` ∈ `{none, ui, data-read, data-write, credential, admin}`; `confirmTier` ∈ `{T0..T3}`.
- `accessible.ariaRole === "menuitem"` and `accessible.focusReturn` is boolean.
- `toolbarOnly === false` everywhere (no permanent toolbar-button expansion).

---

## 6. Accessibility contract (closes R3B-04, registry-level)

A6 owns the **registry-level** accessibility guarantees; A1/A8 own the prototype rendering. The contract each consuming lane must honor:

- A context-menu item is a real `<button>` / `role="menuitem"`; disabled items render `aria-disabled="true"`, are not clickable, and expose `title=<disabledReason>`.
- The menu opens with focus moved to the first item; `↑/↓` rove; `Enter` executes the focused item; `Esc` closes and returns focus to the originating element (mirrors the A6 W17 `ActivityBar` roving-focus contract).
- Submenus use `aria-expanded` on the trigger.
- Every command is reachable without a mouse: via its keybinding (where defined), the command palette (`Ctrl+Shift+P`), or the scoped context menu (keyboard-openable). Parity is structural: a menu action and a palette entry are the same registry row.
- Audit events (for `auditClass !== "none"`) are redacted (no credential material, full SQL, or full file bodies) per `WORKBENCH_BLUEPRINT-20260908.md §5`.

---

## 7. Machine-checkable audit

`A6-R3B-registry-audit.mjs` imports the synthetic `A6-R3B-registry.mjs` and runs the §5 invariants plus the shortcut-freeze and Git-14 checks. It is the deliverable's gate:

```text
$ node logs/research/M5-W18/A6-R3B-registry-audit.mjs
commands: 107 (shell=44, database=16, knowledge=15, git=32)
frozen shortcuts: Ctrl+K->browser.focusAddressSearch, Ctrl+Shift+P->app.openCommandPalette
GIT14 units mapped: 14
required visible Git units: 10
RESULT: PASS — all R3B A6 registry invariants satisfied.
```

Exit code `0` = PASS, `1` = FAIL. A10/A11 may run it as a mechanical gate; it is a research artifact and is **not** wired into `pre-merge.sh` (that would require editing a shared script, out of lane scope).

---

## 8. Reference classification & source map

| Item | Class | Source | Notes |
|---|---|---|---|
| IDEA action system (id/keymap/context-availability) | ADAPT | JetBrains IDEA (behavior reference) | Reimplement schema in TS; no platform code |
| IDEA Search Everywhere / command palette | REIMPLEMENT | IDEA (behavior) | New capability (G5 in R3) |
| IDEA/Obsidian tree collapse-all/expand-all/bounded | REIMPLEMENT | IDEA + Obsidian local-graph | Depth-capped (§3) |
| Obsidian note/link/graph right-click | ADAPT | Obsidian observable behavior | A2/A3 own exact semantics |
| Rebased Git context actions (14 units) | ADAPT_PRODUCT (4) + REIMPLEMENT (10) | `DetachHead/rebased` (primary Git ref, R3 card) | ids frozen here; no source copy |
| Current `FilePanel.vue` ctx-menu + `ctx-sep`/`danger` styling | ADAPT | CURRENT_PRODUCT | Keep visual language, route through registry |
| Current `KEYMAP_SCHEMES` | ADAPT | CURRENT_PRODUCT (`useSettingsStore.ts:8-54`) | Extended to full registry |
| JetBrains platform / Rebased / Obsidian branding & assets | REJECT | — | Forbidden by R3 card + license |

`COPY=0` throughout (no source transplantation). `ADAPT_PRODUCT=4` are the Git units A0 ruled as extending this product's *existing* implementation; `REIMPLEMENT_FROM_BEHAVIOR=10` are built from Rebased behavior, never from its code.

---

## 9. Open items / downstream

- **A9**: reconcile T2/T3 confirmation modals and `auditClass` with the safety threat model (cross-ref A9 threat model). A6 supplies the per-command `confirmTier`/`auditClass`; A9 owns the modal/policy.
- **A1**: consume this registry for the consolidated prototype's menu/palette wiring and a11y demonstration (R3B-04).
- **A10**: verify provenance of the ADAPT/REIMPLEMENT items; the audit is the mechanical gate.
- **A11**: report the gateway (`menu ⊆ palette ⊆ registry` parity) in the acceptance manifest.
- **W19 implementation** (not this lane): S1 bootstraps `useCommandRegistry.ts` + `CommandDef`; S2/S3/S4 domain stores register their `CommandDef`s; Git ids land after/with A7's Rebased map (already aligned here).

## 10. Boundary compliance

- No edits under `src/`, `src-tauri/`, manifests, lockfiles, ACLs, capabilities, or the user vault.
- Only additive research files created under `logs/research/M5-W18/`.
- Branch `codex/m5-w18-a6`; not pushed (A0 integrates).
- Rebase note: A0 already integrated R3 into `origin/master` (`d96b9b5`); rebasing this lane would replay the A6 R3 commit onto A0-integrated A6-R3 shared files (shared-file conflict → A0 per card rule). Deliverables here are additive new files, so the branch was committed without a risky rebase; A0 integrates as usual.
