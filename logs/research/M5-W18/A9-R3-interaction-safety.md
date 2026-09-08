# A9 — R3 Interaction-Safety Review (M5-W18-R3-UX)

> Lane **A9** (`RESEARCH_AND_PROTOTYPE`, no product-code changes). Primary product
> reference: the R3 task cards (`M5-W18-R3-UX-TASKS-20260908.md`), the rejected-shell
> review (`M5-W18-PROTOTYPE-REVIEW-20260908.md`), the workbench blueprint
> (`WORKBENCH_BLUEPRINT-20260908.md`), and the A0 R2B integration audit. Git behavior
> reference: `DetachHead/rebased` (fork of JetBrains/intelliJ-community, Apache-2.0) as
> the primary Git-workflow model per the R3 dispatch; `sourcegit` / `sliogit` as secondary.

## 0. Scope, method, and non-goals

- This is **interaction-safety design evidence for the R3 workbench shell**, not a zvec-grep
  trust-boundary audit (that was the R2B A9 deliverable, `A9-threat-model.md`). The two
  reports are complementary: R2B covered *engine/daemon/network* trust; R3 covers
  *user-facing UI action* safety inside the browser-first shell.
- **Static design + behavior analysis only.** No product source, ACL, capability, native
  runtime, dependency, or user vault is changed. All findings are proposed contracts for
  W19 slices, not implementations.
- "Measured" = read from the cited R3/blueprint/R2B sources or from Rebased's documented
  behavior. "Proposed" = a design rule this lane recommends. "Inferred" = reasoned but not
  dynamically verified (flagged).
- The R3 global acceptance constraints that gate this review: right-click menus are scoped
  to the object under the pointer (no catch-all); **every context-menu action also has a
  command-registry identity, disabled reason, keyboard/accessibility path, and safety
  class**; prototypes cover 1920×1080 / 1440×900 / 1366×768 / 1024×720 plus
  empty/loading/error/disabled/context-menu states.

### Surfaces in scope (R3 task card, A9 line)

1. **Context menus** — right-click on tab/page, tree, note/link/graph node, DB
   connection/schema/table/cell/result, Git repo/branch/commit/file/hunk, terminal, tool window.
2. **Drag/drop** — file→editor open, tab reorder, tree node move, drop into DB/graph/git targets.
3. **Git history rewriting** — amend, interactive rebase/squash, reset (soft/mixed/hard),
   force push, branch delete, cherry-pick, revert, conflict resolution.
4. **Database writes** — SQL console `DELETE/UPDATE/INSERT/DROP/TRUNCATE`, schema mutation,
   result-grid cell edits.
5. **External navigation** — address-bar URL navigation, link clicks leaving the workspace,
   remote/web content in webviews, `tauri://` vs `http://localhost:1421` origins.

### Lane dependencies (consumed, not edited)

- **A6** owns the context-menu *inventory and command-registry parity*. A9 threat-models
  A6's menus but does not redefine their grouping. Where A9 adds a safety class, A6's
  registry is the consumer.
- **A4** owns the *shell-state / persistence* contract (dirty flags, crash-safe write,
  unknown-view fallback). A9's dirty-tree/dirty-document checks depend on A4's DTO shape.
- **A5** owns the *database daily loop* and (with A4/A6) the *cancellation backend
  contract*. A9's DB-write safety builds on A0 ruling #8 (cancellation is a real backend
  contract, not a flag-only button).
- **A3** owns *progressive-disclosure / tool-window* behavior; A9's drag/drop and
  focus-return safety align with A3's edge/collapse rules.
- **A10** will review this report for contradictory ownership and impossible transitions.

---

## 1. Safety framework (proposed, single source of truth)

All five surfaces share one model. Define it once; apply per surface.

### 1.1 Safety class (attached to every command-registry entry)

Every action — whether invoked from a context menu, a toolbar button, a keyboard shortcut,
or the command palette — carries exactly one `safety_class`:

| Class | Meaning | Default UI treatment |
|---|---|---|
| `SAFE` | Read-only / reversible / no external effect | Direct execute, no prompt |
| `MUTATES_LOCAL` | Changes local doc/state, undoable | Direct execute; undo available |
| `DESTRUCTIVE_SOFT` | Deletes/overwrites but recoverable (trash, soft-delete, branch still reflog-able) | Confirm (T2) unless recently confirmed for the same object |
| `DESTRUCTIVE_HARD` | Irreversible or high-blast-radius (force push, `reset --hard`, `DROP`, `DELETE` without WHERE, branch delete of unmerged) | Hard-confirm (T3/T4) with explicit target echo |
| `EXTERNAL_EGRESS` | Leaves the machine or crosses trust boundary (remote navigation, upload, remote-embedding) | Consent gate (T4) + audit, fail-closed |

> A context-menu action with no `safety_class` is **not renderable** — A6's registry must
> reject it (parity rule from R3 global constraints).

### 1.2 Confirmation tiers

| Tier | Trigger | UI | Bypass rule |
|---|---|---|---|
| T0 | `SAFE` | none | — |
| T1 informational | action has side effects user may not expect but is reversible | toast + undo | undo within window |
| T2 confirm | `DESTRUCTIVE_SOFT` | dialog "Are you sure?" + object name + reversible note | explicit Confirm button; Esc/click-outside = cancel |
| T3 hard-confirm | `DESTRUCTIVE_HARD` | dialog echoing exact target (branch name, SQL, row count) + irreversible warning | type/check the target token or tick checkbox; no default focus on Confirm |
| T4 egress-gate | `EXTERNAL_EGRESS` | consent with disclosed scope (what leaves, where) + audit note | explicit opt-in; default = cancel; persistence optional, keyring-only |

**Focus rule (from R3 a11y + A6):** Confirm buttons must never be the default-focused control
for T3/T4; focus starts on Cancel. This prevents the "Enter-to-destroy" trap.

### 1.3 Dirty checks (dirty-tree / dirty-document)

- **Dirty-document**: any open document (file, SQL console, note, terminal buffer with
  uncommitted input) exposes `dirty:boolean` via A4's document DTO. Closing/resetting/
  navigating-away from a dirty document triggers T2 ("unsaved changes — save / discard /
  cancel").
- **Dirty-tree / working tree**: before any Git history-rewriting or branch-switching
  operation, the lane must read the **working-tree status** (staged + unstaged + untracked).
  If dirty and the operation would lose/rewrite work, block or require T3 with the explicit
  list of affected paths (bounded, paginated for large trees — never freeze the UI).
- **Dirty-result**: a query result grid with pending cell edits must block `DROP`/schema
  mutation on the same table until edits are committed or discarded (A5 loop).
- Safe default: **never auto-discard**. "Discard" is `DESTRUCTIVE_HARD`, not a default
  button.

### 1.4 Protected-branch policy (proposed)

- A branch is **protected** if: (a) it is the repo's configured mainline (e.g. `master`/
  `main`/`trunk` per repo config), or (b) it carries a `protected` marker in workspace
  settings, or (c) it is tracked on a remote and the remote marks it protected (read from
  remote if available; otherwise default to mainline-only protection).
- Protected-branch effects:
  - `push --force` / `--force-with-lease` → **T4**, requires typing the branch name, and is
    disabled entirely unless an explicit workspace policy enables force-push.
  - `reset --hard` / `branch -D` on a protected branch → **T3**, with reflog-preservation
    note (local reflog still recoverable; remote is not).
  - Direct commit/amend on a protected branch → discouraged; recommend branch + PR flow,
    but not hard-blocked locally if the user insists (T2).
- Safe default: **protection ON for mainline; force-push OFF by default.**

### 1.5 Undo / abort paths

- **Soft undo stack**: local document edits and reversible tree operations get an in-memory
  undo (bounded, e.g. last 100 ops per document) — `MUTATES_LOCAL`.
- **Abortable operations**: long-running or externally-visible ops (Git push, DB query,
  remote navigation) expose a cancel token (reusing A5/A4 cancellation contract). Abort =
  stop + leave system in a consistent state, never partial-commit.
- **Irreversible operations** (`DESTRUCTIVE_HARD`, `EXTERNAL_EGRESS`) have **no in-app undo**;
  safety comes from T3/T4 confirmation + (where possible) a recoverable tombstone
  (reflog for Git, trash for files, soft-delete for DB rows where the schema allows).
- **Git recovery aid**: after any history rewrite, surface the recovery path in the result
  (e.g. "previous HEAD was `<sha>`; recover via `git reflog` / Rebased's reflog view").

### 1.6 Source checks (command provenance)

- Every command carries a `source` (`context_menu` | `toolbar` | `keyboard` | `command_palette`
  | `script` | `remote`). The **same safety class applies regardless of source** — a
  `DESTRUCTIVE_HARD` action triggered by a keyboard shortcut is still T3, not silent.
- **Remote/script source**: actions arriving over IPC from a remote webview or a script must
  pass the existing `check_invocation_source` + ACL gate (per `WORKSPACE_IDENTITY.md`). A
  remote source may never raise its own privilege tier; egress from a remote source requires
  the T4 gate plus the explicit remote-embedding/authorization pattern borrowed from R2B B1.
- **Provenance echo**: T3/T4 dialogs show *how* the action was triggered (so the user can
  spot an unexpected trigger).

### 1.7 Audit redaction & secrets boundaries (building on R2B A9 B1/B3/B6)

- **Audit log** records: action identity, safety class, source, object id/type, timestamp,
  outcome, and (for egress) disclosed scope. It **never** records:
  - secrets (API keys, tokens, passwords) — keyed redaction regex `/token|api.?key|authorization|secret|password/i`;
  - full SQL text for writes **when it contains bound parameter values that are secret** —
    log the statement shape + row-affected count, redact literal secret values (aligns with
    A0 ruling #9: credentials keyring-only; never persist secret-bearing URLs/SQL with values);
  - full file/workspace content — only path + size + hash.
- **Secrets boundaries** (reaffirm R2B + blueprint §5): credentials live in OS keyring /
  Tauri secure storage only; they may transit JS/Tauri params transiently but must not be
  persisted to disk, to layout state, or to any audit record. URLs with embedded secrets
  (e.g. `https://user:pass@host`) are stripped before persistence/logging.
- **Redaction is fail-closed**: if a value matches the secret pattern it is dropped, never
  logged "for debugging".

---

## 2. Surface 1 — Context menus

**R3 constraint:** right-click menus are scoped to the object under the pointer; they do not
become unstructured catch-all menus. Every action also has a command-registry identity,
disabled reason, keyboard path, and safety class (consumed from A6).

| Object scope | Representative actions | Proposed safety class | Notes |
|---|---|---|---|
| Browser tab/page | Reload, Duplicate, Pin, Close, Close others, Mute | `MUTATES_LOCAL` / `DESTRUCTIVE_SOFT` (Close others) | "Close others" = T2 (reversible-ish via session restore) |
| Workspace/file tree | Open, Reveal, Copy path, Rename, Delete, New file | `DESTRUCTIVE_SOFT` (Delete/Rename) | Delete → trash, T2; Rename → undoable |
| Note / link / graph node | Open, Backlinks, Local graph, Delete, Rename | `DESTRUCTIVE_SOFT` | Note delete → T2 + confirm target title |
| DB connection / schema / table / cell / result | Connect, Refresh, Run, Edit cell, Delete row, Drop table | `MUTATES_LOCAL` (cell edit) → `DESTRUCTIVE_HARD` (Drop/Delete row w/o WHERE) | See Surface 4 |
| Git repo / branch / commit / file / hunk | Checkout, Branch, Stash, Reset, Amend, Force-push, Delete branch | per Surface 3 | — |
| Terminal | Copy, Paste, Clear, Close | `DESTRUCTIVE_SOFT` (Clear/Close) | Clear scrollback = T2 |
| Tool window | Collapse all, Restore, Pin, Auto-hide, Close | `SAFE`/`MUTATES_LOCAL` | never destructive |

**Proposed rules:**
- A disabled menu item shows a **disabled reason** (greyed + tooltip), e.g. "Amend disabled:
  working tree dirty" or "Force-push disabled: protected branch". This satisfies the R3
  "disabled reason" requirement and aids accessibility.
- Sub-menus are grouped (A6), not flat — prevents the catch-all anti-pattern the review
  rejected.
- Keyboard/command-palette parity: the same `safety_class` + tier fires whether invoked by
  right-click or `Ctrl+Shift+P`. No silent fast-path for shortcuts.

---

## 3. Surface 2 — Drag / drop

| Drag source → target | Risk | Proposed rule |
|---|---|---|
| File in tree → editor area | low | open document (`MUTATES_LOCAL`); if target is an existing dirty doc, T2 |
| Tab reorder within strip | low | reorder (`MUTATES_LOCAL`); persisted via A4 layout state |
| Tree node → different folder (move) | `DESTRUCTIVE_SOFT` | move = rename-on-disk; T2 + undo; never cross workspace root without T3 |
| File → DB table / graph node / git commit | `EXTERNAL_EGRESS`-like | **rejected by default** — dropping arbitrary content into a DB/graph/git object is out of scope for R3; show "not a valid drop target" |
| External file from OS → webview | `EXTERNAL_EGRESS` | only allowed into explicitly designated drop zones; path is validated against workspace root (no path escape) before any read |
| Text selection → terminal | low | paste; subject to terminal confirm-if-destructive (Surface 5 external nav adjacency) |

**Proposed rules:**
- Every drop target declares an `accepts` contract; an invalid drop is a no-op with a
  briefly shown reason (no silent data move).
- Cross-root moves (outside the trusted workspace root) are `DESTRUCTIVE_HARD` → T3, because
  they can move files out of the user's authorized scope (trust-boundary echo of R2B B1:
  workspace root is the trust boundary, not a permission grant).
- Drag must be **cancelable** (Esc / drop outside target = abort, no partial move).

---

## 4. Surface 3 — Git history rewriting

Primary model: Rebased/IntelliJ Git log & VCS actions. Map each to a safety class and tier.

| Operation | Rebased/IDEA analog | Safety class | Tier | Protected-branch effect |
|---|---|---|---|---|
| Commit (new) | Commit | `MUTATES_LOCAL` | T0/T1 | normal |
| Amend last commit | Amend | `DESTRUCTIVE_SOFT` (rewrites SHA) | T2 | mainline: T3 + "branch from here?" suggestion |
| Interactive rebase / squash | Rebase (--interactive) | `DESTRUCTIVE_HARD` (rewrites history) | T3 | protected: T4 + force-push consequence note |
| Reset soft/mixed | Reset (Soft/Mixed) | `DESTRUCTIVE_SOFT` | T2 | mainline: T3 |
| Reset hard | Reset (Hard) | `DESTRUCTIVE_HARD` | T3 | protected: T3 + reflog-preservation note |
| Push (normal) | Push | `EXTERNAL_EGRESS` (remote) | T1/T4 if remote is non-default | — |
| Push --force / --force-with-lease | Force Push | `DESTRUCTIVE_HARD` + `EXTERNAL_EGRESS` | T4 (type branch name) | **disabled by default**; needs workspace policy |
| Branch delete (-d/-D) | Delete branch | `DESTRUCTIVE_HARD` | T3 | unmerged on protected: T4; reflog note |
| Cherry-pick | Cherry-pick | `MUTATES_LOCAL` | T1 | — |
| Revert | Revert | `MUTATES_LOCAL` (safe alternative to reset) | T0/T1 | **preferred** over reset where possible |
| Stash / pop | Stash | `MUTATES_LOCAL` | T1 | stash list is recoverable |
| Conflict resolution | Merge/Rebase conflict | `MUTATES_LOCAL` | T1 | must complete or abort; no half-merge persisted |

**Proposed rules:**
- **Dirty-tree gate**: any operation that rewrites history or switches branches is blocked
  (or forces T3) when the working tree is dirty, listing the affected paths (bounded UI,
  paginated for huge trees — never freeze).
- **Recoverability echo**: after amend/rebase/reset, show prior HEAD SHA and the reflog
  recovery path (Rebased exposes reflog; mirror that).
- **Force-push off by default** (safe default from §1.4). When enabled, T4 requires typing
  the exact branch name; `--force-with-lease` is preferred over `--force`.
- **Revert over reset**: the UI should surface `Revert` as the safe first-choice for
  "undo a commit" and demote `Reset --hard` behind a warning (matches IDEA's pattern).
- **No silent remote write**: push is `EXTERNAL_EGRESS`; non-default remotes are T4.
- **Source parity**: `git reset --hard` from a keyboard shortcut or a script is still T3 —
  shortcuts do not lower the tier (§1.6).

---

## 5. Surface 4 — Database writes

Builds on A0 ruling #8 (cancellation is a real backend contract) and blueprint J2/S2. The
SQL console is the primary write surface; result-grid cell edits are a secondary one.

| Statement / op | Safety class | Tier | Required controls |
|---|---|---|---|
| `SELECT` / read | `SAFE` | T0 | cancel token (A5 contract) |
| `INSERT` / `UPDATE` (keyed) | `MUTATES_LOCAL` | T1 | transaction + cancel + affected-row echo |
| `UPDATE`/`DELETE` **without WHERE** | `DESTRUCTIVE_HARD` | T3 | echo row count estimate; require explicit confirm; prefer transaction w/ preview |
| `TRUNCATE` / `DROP` / `ALTER` | `DESTRUCTIVE_HARD` | T3/T4 | T4 for `DROP`/`TRUNCATE` on non-empty; production DB → A0 ruling #9 production-signals gating |
| Result-grid cell edit | `MUTATES_LOCAL` | T1 | commit/rollback; blocks `DROP` on same table until resolved |
| Schema mutation via UI | `DESTRUCTIVE_HARD` | T4 | production DB blocked unless policy allows; online-DDL algorithm echoed |

**Proposed rules:**
- **Production guard**: reuse the R2B/blueprint production-signal detection (A0 ruling #9).
  Writes against a detected production DB require an explicit, typed confirmation and are
  audit-logged with redacted statement shape.
- **Cancel is real**: a running `DELETE`/`UPDATE` must be abortable via the A4/A5 running
  query registry + backend cancellation (idempotent, bounded reclamation) — never a
  flag-only button (A0 ruling #8).
- **WHERE-less guard**: the console must parse/estimate and warn before executing a
  whole-table `DELETE`/`UPDATE`; T3 with the estimated row count.
- **Secrets in SQL**: bound-parameter values that are secrets are redacted from audit (§1.7);
  the statement **shape** + affected count is logged instead.
- **Dirty-result interaction**: pending cell edits block `DROP`/schema change on the same
  table until committed or discarded (consistency with A5 loop).

---

## 6. Surface 5 — External navigation

Trust boundary echoes R2B B1/B5 and `WORKSPACE_IDENTITY.md` debug/release origin rules.

| Navigation | Risk | Proposed rule |
|---|---|---|
| Address-bar URL (user-typed http/https) | `EXTERNAL_EGRESS` (remote content) | allowed; remote content rendered in a sandboxed webview; no local IPC privilege escalation; remote scripts cannot invoke native commands without `check_invocation_source` + ACL |
| Link click to external site | `EXTERNAL_EGRESS` | T1 notice "leaving workspace" for the first external nav per session; no auto-download |
| `tauri://localhost` (release) vs `http://localhost:1421` (debug) | origin integrity | must match `WORKSPACE_IDENTITY.md`; debug IPC only via `dev-capabilities/main.json` under `#[cfg(debug_assertions)]`; never broaden default capability with `remote.urls` |
| Remote embedding / upload (future zvec) | `EXTERNAL_EGRESS` | T4 using the R2B B1 HMAC-signed per-workspace grant + consent + anti-replay; disabled by default |
| `file://` / workspace-root escape | trust boundary | navigation outside the trusted workspace root is **denied** unless explicitly granted; path validated (no `..` escape) |
| Deep link / `invoke` from remote webview | privilege | passes `check_invocation_source` + ACL; remote source cannot raise its tier (§1.6) |

**Proposed rules:**
- External content is **read, not trusted**: a remote page cannot silently gain native
  command access. Native IPC requires the existing source+ACL gate.
- Workspace root is the trust boundary (not a permission grant). Cross-root navigation needs
  explicit, persisted grant (keyring-scoped, not plaintext — R2B B3/A0 #9).
- Egress consent is **fail-closed**: default = cancel; the disclosed scope (what leaves,
  where) is shown before any transfer.

---

## 7. Threat model (consolidated)

| # | Threat | Surface | Likelihood | Impact | Proposed control | Class/Tier |
|---|---|---|---|---|---|---|
| T1 | Right-click catch-all leaks destructive action | Menu | Med | Med | scoped menus + registry parity (A6) | SAFE/T0 |
| T2 | Disabled action reachable via shortcut | Menu | Med | High | tier independent of source (§1.6) | per action |
| T3 | "Enter" confirms destructive dialog | All | High | High | focus starts on Cancel (§1.2) | T3/T4 |
| T4 | History rewrite loses uncommitted work | Git | Med | High | dirty-tree gate (§1.3) | T3 |
| T5 | Force-push to protected/mainline | Git | Low | High | off by default + T4 (§1.4) | T4 |
| T6 | `reset --hard` unrecoverable surprise | Git | Med | High | reflog echo + T3 | T3 |
| T7 | Whole-table DELETE without WHERE | DB | Med | High | WHERE-less guard + T3 | T3 |
| T8 | Cancel button is fake (flag-only) | DB | Low | High | real backend cancel (A0 #8) | T1 |
| T9 | Secret in SQL/URL persisted to audit/disk | All | Med | High | redaction fail-closed (§1.7) | audit |
| T10 | Drop moves file out of workspace root | DnD | Low | Med | cross-root = T3 (§3) | T3 |
| T11 | Remote page gains native command access | Nav | Low | High | source+ACL gate; no tier raise | T4 |
| T12 | External navigation escapes workspace root | Nav | Low | Med | root validation + grant | T4 |

---

## 8. Alignment with A0 R2B rulings & other lanes

- **Ruling #8 (DB cancellation is real):** Surface 4 requires a backend cancellation
  contract (A4/A5), not a flag-only button. ✅ adopted.
- **Ruling #9 (credentials keyring-only):** §1.7 + Surface 5 affirm OS keyring / Tauri
  secure storage; no plaintext, no secret-bearing URL/SQL persistence. ✅ adopted.
- **Ruling #3 (DbValue defect):** interaction safety is independent of the wire-type fix, but
  DB-write audit logging must wait for the unified `DbValue` contract (S0) so row counts and
  redaction are correct. ✅ noted as dependency.
- **A6 (context menus):** A9's safety classes are consumed by A6's registry; no duplication
  of menu inventory. ✅ dependency.
- **A4 (persistence):** dirty flags / crash-safe write / unknown-view fallback are the
  substrate for §1.3 dirty checks. ✅ dependency.
- **A3 (progressive disclosure):** drag/drop focus-return and edge rules align with A3's
  tool-window behavior. ✅ alignment.
- **A10 (review):** this report is the input for A10's contradiction/ownership check. ✅ hands off.

---

## 9. R3 acceptance-safety mapping

Every R3 global acceptance constraint that touches safety is satisfied by the framework:
- *Every context-menu action has a command-registry identity, disabled reason, keyboard/
  accessibility path and safety class* → §1.1 + Surface 1.
- *Right-click menus scoped to object under pointer, not catch-all* → Surface 1 table + §7 T1.
- *Prototypes cover disabled/context-menu/error states* → tiers T0–T4 + disabled-reason
  rendering are prototype-required states.
- *No product source / ACL / capability / native runtime change* → this report is design
  evidence only; no files outside `logs/research/M5-W18/` are touched.

---

## 10. Open questions / W19 dependencies

1. **Force-push default**: recommend OFF; needs a workspace-policy schema (A4) to persist the
   opt-in. Blocks T4 force-push UI in S-whenever Git slice lands.
2. **Dirty-tree status source**: depends on A4's document/working-tree DTO and the actual Git
   backend (S0/S2). The *gate logic* is specified here; the *data source* is W19.
3. **Reflog recovery surfacing**: requires the Git backend to expose reflog (Rebased does);
   R3 prototype should show the recovery path even if backend is synthetic.
4. **DB production-signal detection**: reuse R2B/A0 production-signals; needs the DB backend
   (A4/A5) to surface `is_production_database`. Specified here; data in W19.
5. **Redaction regex scope**: keep R2B B6 key set; extend with `password` and
   secret-bearing-URL stripping. Single source in A10's ledger.
6. **Cross-root navigation grant**: persistence mechanism = OS keyring (A0 #9); UI state in
   A4. Specified here.

## 11. Deliverables of this lane

- `logs/research/M5-W18/A9-R3-interaction-safety.md` — this report.
- `logs/research/M5-W18/A9-R3-checkpoint.md` — lane checkpoint.
- No product code, ACL, capability, manifest, or user vault changed. Not pushed (A0
  integrates). STATUS = PASS (research complete; W19 implementation deferred per board).
