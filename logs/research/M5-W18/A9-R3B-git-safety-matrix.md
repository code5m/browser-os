# A9 — R3B Git Interaction-Safety Matrix (all 14 workflow units)

> Lane **A9** (`RESEARCH_AND_PROTOTYPE`, no product-code changes).
> Wave: **M5-W18-R3B** (Targeted Prototype Closure; correction wave — no product source / ACL / capability / dependency / native runtime / user-vault change).
> Date: 2026-09-08.
> Extends `A9-R3-interaction-safety.md` (R3 surface-level review) to the **canonical 14 Git workflow units** per `A7-R3-git-workflow-reference-map.md` (pinned `DetachHead/rebased` v1.1.15 `@cee14e9`; git4idea Apache-2.0).
> Operative NEXT = `M5-W18-R3B` (per `WORKSPACE_IDENTITY.md` + `PARALLEL_COMMAND_BOARD.md`, which supersede the older R2B dispatch). R2B domain research "remains accepted"; this lane executes the R3B §A9 correction: *extend the safety review across all 14 Git units, freeze dirty-tree / protected-branch / confirmation / undo-abort / audit-redaction / credential boundaries, and verify context menus do not bypass command policy.*

## 0. Scope, method, non-goals

- **This is design evidence for W19 Git slices, not an implementation.** No file outside `logs/research/M5-W18/` is changed; no product source / ACL / capability / dependency / user vault is touched.
- **Measured** = read from the cited product source (`src-tauri/src/{bridge,domain,sync}.rs`) at the lines noted. **Proposed** = a safety rule this lane recommends for W19. **Inferred** = reasoned but not dynamically verified (flagged; real GUI verification is reserved for A0/user per board).
- The R3B global constraints bind: no donor-source transplant; prototype must be self-contained; native desktop evidence is reserved for A0/user review.
- The R3B §A9 "especially" operations — **amend, reset/revert, rebase continue/abort, cherry-pick continue/abort, conflict resolution, patch apply, command log** — receive dedicated treatment in §2.3 and §3.

### Lane dependencies (consumed, not edited)

- **A7** owns the canonical 14-unit matrix + classification (ADAPT=5 / REIMPLEMENT=9, COPY=0, pinned `cee14e9`). This report consumes that matrix and adds the safety layer.
- **A6** owns the command-registry inventory + per-action identity / disabled-reason / keyboard parity. A9's frozen tier map (§2.3) is the **source of truth A6 consumes**; A9 does not redefine menu grouping.
- **A4** owns the shell-state / persistence contract (dirty flags, crash-safe write). A9's dirty-tree gate (§2.1) depends on A4's working-tree DTO.
- **A10** reviews this report for contradictory ownership and verifies the §4 context-menu claim against real frontend wiring.
- **A1** consumes the tier map for the prototype's Git-mode rendering (correct safety classes at 6 sizes + failure/conflict/empty states).

---

## 1. Measured product Git safety substrate (the frozen assets)

The following product facts are the **non-negotiable foundation** the 14-unit safety matrix must preserve and extend. All citations are from the current canonical source (HEAD `200f0f1`).

| # | Product safety asset | Measured location | What it freezes |
|---|---|---|---|
| S1 | Two-phase confirmable-job gate `request_git_write`/`confirm_git_write` | `bridge.rs:2150-2151`, `2366-2377` (request), `2479-2486` (confirm) | Any Git write is staged, previewed, then confirmed; never executes in phase 1. |
| S2 | Source-agnostic gate: `check_invocation_source(&webview, …)` in **both** phases | `bridge.rs:2377`, `2486` | Context menu / keyboard / palette / script / remote all pass the same source+ACL gate at the IPC layer. |
| S3 | Op whitelist `GitWriteOp` = {Stage, Unstage, Discard, Commit, CreateBranch, CheckoutBranch, Push} | `domain.rs:234-266` (`enum` + `from_op_str`); non-whitelist → `Err("操作禁止")` at `bridge.rs:2378-2379` | History-rewriting ops (reset/merge/rebase/cherry-pick/stash/amend/revert/branch-delete/patch) are **currently rejected by the backend**, fail-closed. |
| S4 | Dangerous double-confirm: `Discard \| Push` require `confirmed_dangerous=true` | `domain.rs:274`; enforced `bridge.rs:2189` (`take_confirmable_git_job`) | Destructive ops cannot be confirmed without the explicit second-confirm payload. |
| S5 | Job TTL 300s, one-shot, expired purged | `bridge.rs:2169` (`GIT_WRITE_JOB_TTL_SECS`); `2185-2188` expiry; `2202` (`purge_expired_git_jobs`) | No stale/half-confirmed job lingers; confirm is single-use. |
| S6 | Audit redaction by construction: `git_write_audit_detail` signature excludes paths/diff/credentials/remote URL | `bridge.rs:2211-2214` | Audit detail **cannot** carry secrets by type signature (fail-closed). |
| S7 | Push is non-force and single-remote by construction: `GIT_PUSH_REMOTE="origin"` + `build_push_refspec` forbids `+` (force) and leading `:` (remote delete) | `sync.rs:919`, `921-937` | Force-push and remote-branch-deletion are **structurally impossible** at the refspec level even if a UI button existed. |
| S8 | Credentials read from OS keyring only at push time | `sync.rs:15-16` (`push_artifacts` reads system keychain; `cred_cb` at `241-244`) | No credential persisted to disk / layout / audit; keyring-only. |
| S9 | Dirty-tree precheck exists for checkout | `sync.rs:835` (`checkout_conflict` precheck vs dirty working tree) | Branch switch already blocks on dirty tree. |

**Conclusion of §1:** the product already enforces a fail-closed Git write boundary at the IPC layer (S1–S2), restricts the write surface to 7 whitelisted ops (S3), requires double-confirm for the two dangerous ones (S4), redacts audit (S6), and makes force-push/remote-delete impossible at the wire (S7). The 14-unit R3B extension is therefore a **spec for W19** that must *preserve S1–S8* when the remaining 12 ops are whitelisted.

---

## 2. Frozen boundaries (the six the R3B card demands)

Each boundary is defined, anchored to a measured product fact, stated as frozen, and flagged for W19 extension where applicable.

### 2.1 Dirty-tree boundary

- **Definition:** before any history-rewriting or branch-switching op, read the working-tree status (staged + unstaged + untracked). If dirty and the op would lose/overwrite uncommitted work → block, or require T3 with a **bounded** list of affected paths (never freeze the UI on a huge tree).
- **Product anchor:** `read_status` (`sync.rs:354`) exists; `checkout_conflict` precheck (`sync.rs:835`) already gates `CheckoutBranch`. All other history-rewriting ops are currently **not whitelisted** (S3), so no dirty-tree exposure yet.
- **Frozen rule:** every future history-rewriting op (reset / merge / rebase / cherry-pick / stash pop·apply / amend) **must** call a dirty-tree precheck reusing `read_status`/`checkout_conflict`. The affected-path list is capped at `GIT_WRITE_PREVIEW_MAX_PATHS=20` with pagination beyond. Read-only units (status, diff, log-graph, history/blame, branch-list, command-log view) are **exempt**.
- **W19:** extend the `request_git_write` precheck (currently only checkout-aware) to cover the new whitelisted ops. Until then the backend already rejects them (S3), so the dirty-tree risk is contained.

### 2.2 Protected-branch boundary

- **Definition:** a branch is **protected** if (a) it is the configured mainline (`master`/`main`/`trunk` per repo config), or (b) it carries a workspace `protected` marker, or (c) the remote marks it protected. Effects:
  - force-push → **T4**, requires typing the exact branch name, and is disabled entirely unless a workspace policy enables it.
  - `reset --hard` / `branch -D` on a protected branch → **T3** with a reflog-preservation note.
  - direct commit/amend on a protected branch → **T2** + recommend branch-then-PR.
- **Product anchor (structural guarantee):** `build_push_refspec` (`sync.rs:921-937`) emits only `refs/heads/X:refs/heads/X` — **no `+` prefix (force) and no leading `:` (remote delete)**. `GIT_PUSH_REMOTE="origin"` is hardcoded (`sync.rs:919`). Therefore force-push and remote-branch-deletion are **impossible at the wire regardless of UI** (S7).
- **Frozen rule:** force-push stays impossible (no `+` refspec) even if a UI control is added; `branch -D` / `reset --hard` on protected branches require T3 + reflog note; the product already prevents remote force/delete, so the push side of this boundary is structurally guaranteed. Local destructive ops on protected branches use the T3 gate + warning.
- **W19:** add the workspace `protected` marker DTO (A4) and the T3/T4 dialogs echoing the branch name + reflog-recovery hint.

### 2.3 Confirmation-tier boundary (T0–T4 map for all 14 units + the "especially" ops)

Tier vocabulary (from `A9-R3-interaction-safety.md` §1.2): T0 none · T1 toast+undo · T2 confirm (reversible) · T3 hard-confirm (irreversible, target echoed, Cancel-focused) · T4 egress-gate (typed target, default=cancel). Safety classes: `SAFE` / `MUTATES_LOCAL` / `DESTRUCTIVE_SOFT` / `DESTRUCTIVE_HARD` / `EXTERNAL_EGRESS`.

**Canonical 14-unit tier map** (unit names per `A7-R3-git-workflow-reference-map.md` §3):

| # | Unit | Repr. ops | Safety class | Tier | Protected effect | Product state |
|---|---|---|---|---|---|---|
| 1 | status | read statuses | `SAFE` | T0 | read-only | IMPLEMENTED (`read_status`) |
| 2 | hunk staging | stage/unstage hunk | `MUTATES_LOCAL` | T1 | — | ABSENT (path-only `write_stage`) |
| 3 | diff | read diff | `SAFE` | T0 | read-only | IMPLEMENTED (`read_diff`) |
| 4 | log graph | render DAG | `SAFE` | T0 | read-only | ABSENT |
| 5 | branches | list / create / checkout / **delete** | create·checkout `MUTATES_LOCAL` T1; delete `-d` `DESTRUCTIVE_HARD` T3; `-D` unmerged/protected **T4** | T1 / T3 / T4 | delete on protected → T4 | list/create/checkout IMPLEMENTED; **delete NOT whitelisted** (rejected, S3) |
| 6 | worktrees | add / **remove** | add `MUTATES_LOCAL` T1; remove `DESTRUCTIVE_SOFT` T2 (trash-like) | T1 / T2 | — | ABSENT |
| 7 | stash | push T1; pop/apply T1 (overwrite-conflict risk); **drop** `DESTRUCTIVE_HARD` **T3** | T1 / T3 | drop on protected stash ref → T4 | ABSENT |
| 8 | merge | `git merge` | `MUTATES_LOCAL` (conflictable) | T2 | conflict → resolve flow | PARTIAL (auto `three_way_merge` only) |
| 9 | rebase / interactive | replay / todo / **continue / abort** | replay `DESTRUCTIVE_HARD` T3; **continue `MUTATES_LOCAL` T1**; **abort `MUTATES_LOCAL` T1 (easy, reflog-recoverable)** | T3; protected → T4 + force-push consequence | ABSENT |
| 10 | cherry-pick | pick commit(s) / **continue / abort** | pick `MUTATES_LOCAL` T1; **continue T1**; **abort T1 (easy)** | T1 | protected target → T2 | ABSENT |
| 11 | conflicts | resolve (3-way) | `MUTATES_LOCAL` T1 | T1 | must complete-or-abort; no half-merge persisted | DETECT-only (`read_status=conflicted`) |
| 12 | history / blame | annotate / file history | `SAFE` | T0 | read-only | ABSENT |
| 13 | patch | `format-patch` (export) / `git apply` (import) | export `SAFE` T0; **apply `MUTATES_LOCAL` T2 (overwrites)** | T0 / T2 | dirty-tree gate applies to apply | ABSENT |
| 14 | command log | view executed-command console | `SAFE` T0 (view); **raw output must NOT contain credentials** | T0 | — | PARTIAL (audit only) |

**"Especially" operations called out by R3B §A9** (these are operations *across* the units above; listed explicitly so the freeze is unambiguous):

| Operation | Safety class | Tier | Dirty-tree | Protected | Undo/abort | Notes |
|---|---|---|---|---|---|---|
| **amend** last commit | `DESTRUCTIVE_SOFT` (rewrites SHA) | T2 (mainline → **T3**) | required (blocks if dirty) | mainline T3 + "branch from here?" suggestion | recovery: prior HEAD + `git reflog` | ABSENT (not whitelisted) |
| **reset soft/mixed** | `DESTRUCTIVE_SOFT` | T2 (mainline → T3) | required | mainline T3 | reflog recovery | ABSENT |
| **reset --hard** | `DESTRUCTIVE_HARD` | **T3** | required (lists affected paths) | protected → T3 + reflog-preservation note | reflog recovery (local only; remote not) | ABSENT |
| **revert** | `SAFE`/`MUTATES_LOCAL` | T0/T1 | none | — | preferred safe alternative to reset; UI must surface first | ABSENT |
| **rebase continue** | `MUTATES_LOCAL` | T1 | in-progress (resume) | — | — | resumes; paired with abort |
| **rebase abort** | `MUTATES_LOCAL` | **T1 (easy, prominent)** | restores pre-rebase | — | reflog-recoverable | must be reachable, not buried |
| **cherry-pick continue** | `MUTATES_LOCAL` | T1 | resume | — | — | paired with abort |
| **cherry-pick abort** | `MUTATES_LOCAL` | **T1 (easy)** | restores pre-pick | — | reflog-recoverable | must be reachable |
| **conflict resolution** | `MUTATES_LOCAL` | T1 | during merge/rebase/cherry | — | must complete-or-abort; no half-state persisted | DETECT-only today |
| **patch apply** | `MUTATES_LOCAL` | **T2** | required (overwrites WT) | — | `git apply --reverse` if clean | ABSENT |
| **command log** (view) | `SAFE` | T0 | — | — | **redact credentials in remote URLs** | partial (audit) |

**Frozen rule:** the tier/safety-class in this table is the **single source of truth** A6's registry consumes. When W19 whitelists a new op, its tier is taken from here; a context menu, keyboard shortcut, or palette entry for that op inherits the same tier (no source may lower it — see §4). T3/T4 dialogs are **Cancel-focused** (focus starts on Cancel) to prevent Enter-to-destroy.

### 2.4 Undo / abort boundary

- **Soft undo:** local document edits and reversible tree ops get an in-memory undo (bounded, e.g. last 100 ops/document) — `MUTATES_LOCAL`.
- **Abortable ops (must be easy to reach, T1, never buried):** `rebase --abort`, `cherry-pick --abort`, `merge --abort`. These restore the pre-op state via reflog. The UI **must expose Abort prominently** during an in-progress rebase/cherry-pick/merge; it is NOT hidden behind a destructive confirm.
- **Irreversible ops (no in-app undo; always surface recovery path):** `reset --hard`, `branch -D`, `stash drop`, `push` (already non-force), `patch apply` overwrite. Recovery via reflog / tombstone. Every such op echoes the **prior HEAD SHA** + `git reflog` recovery hint (Rebased exposes a reflog view — mirror that).
- **Frozen rule:** every irreversible op shows its recovery path; abort actions are T1 and are never behind a destructive confirm. The product's `GitWriteJob` already enforces one-shot + TTL (S5) → no half-committed state for implemented ops; this property must be preserved when new ops land.

### 2.5 Audit-redaction boundary

- `git_write_audit_detail` (`bridge.rs:2214`) signature **deliberately excludes** paths/diff/credentials/remote URL → redaction by construction (fail-closed).
- **Frozen rule:** any new Git op added to the whitelist **must** route through `git_write_audit_detail` and **must not** add credential/path-bearing parameters. Affected-row counts and reflog echoes are allowed (no secrets).
- **Command log (unit 14):** must NOT log credentials. Any remote URL containing embedded credentials (`https://user:pass@host`) is stripped before display/persistence, per `A9-R3-interaction-safety.md` §1.7 secrets boundary.
- **W19:** when rebase/cherry-pick/merge/conflict units land, their audit events reuse the existing `git_write_*` audit names + `git_write_audit_detail` redaction (do not invent a new, unredacted audit path).

### 2.6 Credential boundary

- Git remote credentials live in **OS keyring / Tauri secure storage**, read only at push time (`sync.rs:15-16`); never persisted to disk / layout / audit.
- Push uses the hardcoded `origin` (`sync.rs:919`) → bounded egress surface (no arbitrary remote). `cred_cb` is invoked **only inside push** (`sync.rs:241-244`).
- **Frozen rule:** no credential in any log / audit / command-log; keyring-only; origin-only push; embedded credentials stripped from displayed URLs. This aligns with A0 ruling #9 (credentials keyring-only).

---

## 3. Per-unit safety matrix (all 14, consolidated)

Combines A7's classification (§3) with the safety layer from §2. `W19 class` per A7 (ADAPT / REIMPLEMENT). `Safety`/`Tier` per §2.3.

| # | Unit | Product state | W19 class (A7) | Safety class | Tier | Dirty gate | Protected | Undo/abort | Audit | Credential |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | status | IMPLEMENTED | ADAPT | `SAFE` | T0 | exempt | — | — | read-only | — |
| 2 | hunk staging | ABSENT | REIMPLEMENT | `MUTATES_LOCAL` | T1 | path subset | — | soft undo | `git_write_*` | — |
| 3 | diff | IMPLEMENTED | ADAPT | `SAFE` | T0 | exempt | — | — | read-only | — |
| 4 | log graph | ABSENT | REIMPLEMENT | `SAFE` | T0 | exempt | — | — | read-only | — |
| 5 | branches | list/create/checkout IMPLEMENTED; delete rejected | ADAPT | mixed (see §2.3) | T1/T3/T4 | delete: required | delete protected → T4 | T3 recoverable | `git_write_*` | branch name only (not secret) |
| 6 | worktrees | ABSENT | REIMPLEMENT | mixed | T1/T2 | remove: required | — | trash-like | `git_write_*` | — |
| 7 | stash | ABSENT | REIMPLEMENT | mixed | T1/T3 | push: yes | drop protected → T4 | drop T3 (extend double-confirm S4) | `git_write_*` | — |
| 8 | merge | PARTIAL | ADAPT | `MUTATES_LOCAL` | T2 | yes | conflict→resolve | abort T1 | `git_write_*` | — |
| 9 | rebase / interactive | ABSENT | REIMPLEMENT | `DESTRUCTIVE_HARD` | T3 | yes | protected → T4 | continue T1 / abort T1 (easy) | `git_write_*` | — |
| 10 | cherry-pick | ABSENT | REIMPLEMENT | `MUTATES_LOCAL` | T1 | yes | protected target → T2 | continue T1 / abort T1 (easy) | `git_write_*` | — |
| 11 | conflicts | DETECT-only | REIMPLEMENT | `MUTATES_LOCAL` | T1 | during op | — | complete-or-abort | `git_write_*` | — |
| 12 | history / blame | ABSENT | REIMPLEMENT | `SAFE` | T0 | exempt | — | — | read-only | — |
| 13 | patch | ABSENT | REIMPLEMENT | mixed | T0/T2 | apply: required | — | `apply --reverse` if clean | `git_write_*` | — |
| 14 | command log | PARTIAL (audit) | REIMPLEMENT | `SAFE` | T0 | — | — | view only | **redact creds** (S6) | **strip URL creds** (S8) |

---

## 4. Context-menu policy verification (R3B §A9 requirement)

**Claim to verify:** *context menus do not bypass command policy.*

**Verification (static, structural — measured from product source):**

1. **The safety gate is at the IPC layer, not the UI.** `request_git_write` (`bridge.rs:2366`) and `confirm_git_write` (`bridge.rs:2479`) are `#[tauri::command]` handlers; **both** call `check_invocation_source(&webview, …)` (`bridge.rs:2377`, `2486`). Therefore, regardless of whether the trigger is a right-click context menu, a keyboard shortcut, the command palette, a script, or a remote webview, the **same handler** runs with the **same source+ACL gate**. A context menu cannot create a silent fast-path that skips the gate. ✅
2. **The write surface is server-frozen.** `from_op_str` (`domain.rs:258-266`) returns `None` for any op outside the 7-entry whitelist → `request_git_write` returns `Err("操作禁止")` (`bridge.rs:2378-2379`). A context-menu entry that maps to a non-whitelisted op (e.g. "Reset --hard") is **rejected by the backend even if the frontend rendered it**. The menu cannot expand the write surface. ✅
3. **Tiers / dangerous flags are server-enforced.** `take_confirmable_git_job` (`bridge.rs:2189`) refuses `Discard|Push` without `confirmed_dangerous=true` (`domain.rs:274`). A context-menu "Discard" still requires the explicit second-confirm payload; the frontend cannot lower the tier. ✅
4. **Registry parity (R3 global constraint + A6):** every context-menu action must carry a command-registry identity + disabled reason + keyboard/accessibility path + safety class (owned by A6). Because the safety class/tier is attached to the **registry entry**, a right-click and `Ctrl+Shift+P` invoke the **same registry command** → same class/tier. No silent shortcut. ✅

**Conclusion:** context menus **cannot** bypass command policy because (a) the policy is enforced in the Rust IPC handler irrespective of source, (b) the op set is server-frozen, (c) tiers/dangerous flags are server-enforced, and (d) registry parity makes the trigger source irrelevant to the class/tier.

**Residual risk / dependency (NOT a product-gate gap):**
- The **frontend must still wire** each Git context-menu item to the correct registry command (A6) and must not invent a client-only destructive shortcut. This is a W19 implementation + A6 machine-checkable registry-audit item (R3B §A6: "machine-checkable registry audit").
- **Dynamic GUI verification** (launching the Tauri window and clicking through every Git context menu) is reserved for A0/user per board (native desktop evidence). This lane's verification is **static/structural = PASS**; the dynamic GUI pass is marked **NOT_RUN** below.
- **A10** must confirm A6's registry includes all 14 units' destructive actions with tiers matching §2.3; A9 supplies §2.3 as the authoritative tier map A6 consumes.

---

## 5. Threat model delta (Git-specific, extends R3 T1–T12)

| # | Threat | Unit(s) | Control | Class/Tier |
|---|---|---|---|---|
| G1 | History rewrite loses uncommitted work | reset/rebase/cherry/merge/stash/amend | dirty-tree gate (§2.1) | T3 |
| G2 | Force-push to protected/mainline | branches/push | structurally impossible via refspec (S7) + T4 if ever enabled | T4 |
| G3 | `reset --hard` surprise | reset | reflog echo + T3 (§2.3) | T3 |
| G4 | Rebase/cherry-pick abandoned mid-flight (no abort) | rebase/cherry | prominent Abort T1 (§2.4) | T1 |
| G5 | `stash drop` unrecoverable | stash | extend double-confirm (S4) → T3 | T3 |
| G6 | Command-log leaks credential in remote URL | command log | redaction (§2.5/§2.6) | audit |
| G7 | Context-menu "fast path" bypasses confirm | all | refuted by IPC-layer gate (§4) | per-op |
| G8 | `patch apply` overwrites working tree | patch | T2 + dirty-tree gate | T2 |
| G9 | `branch -D` unmerged on protected | branches | T4 + reflog note | T4 |
| G10 | `revert` not offered / `reset` preferred | reset/revert | UI surfaces `revert` first (§2.3) | T0/T1 |
| G11 | Remote branch delete (`:` refspec) | push | structurally impossible (S7) | — |
| G12 | Credential persisted to audit/layout | push/credential | keyring-only + redaction (S6/S8) | audit |

---

## 6. W19 dependency / handoff

- **A6:** consume §2.3 tier map for the command registry; audit that every Git context-menu item maps 1:1 to a registry command with the correct class/tier + a disabled reason (dirty / protected). Machine-checkable registry audit (R3B §A6).
- **A4:** provide the working-tree DTO + dirty precheck used by §2.1; add the workspace `protected` marker DTO used by §2.2.
- **A7:** when extending the whitelist (reset / merge / rebase / cherry-pick / stash / amend / branch-delete / patch), **preserve S1–S8**: confirmable-job gate, `check_invocation_source`, audit redaction, TTL, origin-only non-force push. The 14-unit classification (ADAPT=5 / REIMPLEMENT=9) is the build plan.
- **A10:** verify the §4 claim against real frontend wiring (static PASS; dynamic NOT_RUN here); confirm no contradictory ownership; confirm A6's registry covers all 14 units with matching tiers.
- **A1:** render the 14 units at the 6 target sizes + empty/loading/error/disabled/conflict states with the correct safety classes/tiers from §2.3/§3.

---

## 7. Deliverables of this lane

- `logs/research/M5-W18/A9-R3B-git-safety-matrix.md` — this report (extends `A9-R3-interaction-safety.md` across all 14 Git units).
- `logs/research/M5-W18/A9-R3B-checkpoint.md` — lane checkpoint.
- No product code / ACL / capability / manifest / dependency / user-vault changed. **Not pushed** (A0 integrates). STATUS = PASS (research complete; dynamic GUI verification NOT_RUN, reserved for A0/user).

## 8. Verification

- Evidence citations are file:line reads from canonical source (`bridge.rs`, `domain.rs`, `sync.rs`) at HEAD `200f0f1` — structural/static verification **PASS**.
- `git diff --check` on the new files: clean (no trailing whitespace / conflict markers) — to be run at commit.
- Files added are confined to `logs/research/M5-W18/` (A9's assigned research path). No overlap with A1 (prototype HTML), A6 (registry), A4 (persistence), A7 (reference map) — this report references them but edits none.
- **NOT_RUN (by design):** launching the Tauri window and dynamically clicking through every Git context menu to confirm visual behavior — reserved for A0/user native-desktop review per board; requires the W19 Git slices to be implemented first.
