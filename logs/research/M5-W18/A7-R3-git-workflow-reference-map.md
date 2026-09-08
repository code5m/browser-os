# A7 — R3 Full Git Workflow Reference Map (R3B corrected)

> Lane: A7 (Full Git workflow reference map)
> Wave: M5-W18-R3B (`RESEARCH_AND_PROTOTYPE`, targeted correction of R3)
> Date: 2026-09-08 (R3B revision)
> BASE: `codex/m5-w18-a7` rebased onto `origin/master` (`d96b9b5`)
> Mode: **research only** — no product source / ACL / capability / dependency changes, no push.
> Companion: `A7-R3-lane-checkpoint.md`, `A7-R3B-git-command-map.md`, `A7-R3B-lane-checkpoint.md`
>
> **R3B correction scope (per R3B card §A7 + A0 ruling):** (1) pin `v1.1.15 @ cee14e9` as the
> behavior SSOT and demote `2896562e` to a later research snapshot only; (2) fix the 14-unit
> classification tally to `COPY=0 / ADAPT_PRODUCT=4 / REIMPLEMENT_FROM_BEHAVIOR=10` so it matches the
> 14-row matrix; (3) make explicit that `ADAPT_PRODUCT` extends **this repository** only (no donor
> transplant); (4) for each unit give target UI placement, existing product symbol, required future
> command/state boundary, tests, safety and license/provenance note.

## 0. Pinned upstream references (exact revision)

| Reference | Identity | Pinned revision | Date | License / terms |
|---|---|---|---|---|
| **Rebased** (primary) | `DetachHead/rebased` — Git client on IntelliJ platform, fork of `JetBrains/intellij-community` | **v1.1.15 @ `cee14e9`** (latest release as of 2026-09-05; 519,892 commits in fork history) | 2026-09-05 | Root `LICENSE.txt` = **JetBrains Open-Source Build Terms**; bundled OSS (incl. git4idea) **Apache-2.0**; `NOTICE.txt` attributes IntelliJ IDEA. |
| Rebased inherited core | `JetBrains/intellij-community` — `plugins/git4idea` (the Git VCS integration module) | same revision boundary as the rebased pin (git4idea is carried verbatim from upstream Community at fork point; rebased does not reimplement it) | — | **Apache-2.0** (IntelliJ IDEA Community Edition License) |
| **SourceGit** (secondary) | `sourcegit-scm/sourcegit` | n/a (used only to challenge gaps) | — | MIT |
| **slio-git** (secondary) | `slio-git` | n/a (used only to challenge gaps) | — | MIT / Apache-2.0 |
| **Rebased `2896562e`** (research snapshot) | `DetachHead/rebased` master HEAD | **`2896562e`** (later research snapshot, 2026-09-06) | — | Same license family as the `cee14e9` pin. |

> **Revision reconciliation (R3B / A0 ruling):** `v1.1.15 @ cee14e9` is the **behavior SSOT** for the
> 14-unit map below — it is the latest *released* build and the canonical source of observed Git UX
> behavior. `2896562e` is retained **only as a later research snapshot** (used by A10 for feasibility
> probing) and is **not** the behavior SSOT; any W19 adoption must cite `cee14e9`, not `2896562e`. The
> two revisions differ (post-`cee14e9` master commits), so A7 and A10 no longer disagree once `cee14e9`
> is fixed as SSOT and `2896562e` is explicitly labelled "research snapshot only".

**Inheritance boundary (exact delta from upstream, per R3 card §Git reference boundary):**
Rebased = IntelliJ IDEA Community Edition **with most bundled plugins removed**, retaining only the Git integration (`git4idea`) plus a small set of UI tweaks. Its own deltas vs. inherited IntelliJ code:

- **Rebased delta D-R1 — Customizable Git Log location**: default = Git log as the main editor content; toggle to bottom tool window via *Settings › Version Control › Log › uncheck "Show the log in the editor window"*. This is the defining product move R3 must prototype as **both placements, reversible** (R3 card §Git reference boundary).
- **Rebased delta D-R2 — Disable `.idea` directory**: project config moved to a single global `.idea` in IDE settings; not relevant to a browser-OS file model → **REJECT (not applicable)**.
- **Rebased delta D-R3 — Extra TextMate bundles** (currently `vue`): editor syntax only → out of Git scope; REJECT for the Git map.
- **Rebased delta D-R4 — Cross-platform builds** (AppImage/winget/dmg/Homebrew): packaging only → REJECT (product uses Tauri bundling).

Everything in the 14-unit matrix below that is "in git4idea" is **inherited IntelliJ code**, NOT rebased-authored. Exact file-level provenance (file:line) is intentionally **not** captured here — per R3 card "Exact source reuse is forbidden until A7 identifies the precise delta … and A10 records file-level provenance". This map records **module/path-level identity**; A10 must complete the file-level provenance + NOTICE gate before any W19 adoption.

## 1. Current product Git inventory (measured from canonical source)

Source of truth: `src-tauri/src/sync.rs` (git2), `src-tauri/src/bridge.rs` (IPC + confirmable-job gate), `src-tauri/src/main.rs` (`generate_handler!` registration), `src-tauri/src/domain.rs` (DTOs).

**Read surface (M1-5, read-only git2 — hard constraint forbids any write/network call):**
| Capability | Product fn | IPC command | Notes / limits |
|---|---|---|---|
| Status | `read_status` (`sync.rs:354`) | `git_status` (`bridge.rs:2109`) | statuses → `conflicted`/`untracked`/`added`/`deleted`/`renamed`/`modified`; includes untracked, excludes ignored. |
| Diff | `read_diff` (`sync.rs:402`) | `git_diff` (`bridge.rs:2123`) | HEAD→workdir+index per-file patch; `GIT_DIFF_DEFAULT_MAX_BYTES=64KiB`, `GIT_DIFF_HARD_CAP=256KiB`; binary marker; `more` flag on overflow. **File-level only, no hunk/line granularity.** |
| Branch list | `read_branches` (`sync.rs:503`) | `git_branch_list` (`bridge.rs:2140`) | local + remote-tracking, `is_head`. |
| Log graph | — | — | **ABSENT.** Only `graph_ahead_behind` (`sync.rs:969`) for push ahead/behind counts. |
| History / Blame | — | — | **ABSENT.** |

**Write surface (M1-6.b, `git2` write primitives; all gated by `request_git_write`→`confirm_git_write` confirmable-job + `log_audit` redaction):**
| Capability | Product fn | IPC | Notes / limits |
|---|---|---|---|
| Stage | `write_stage` (`sync.rs:689`) | `request_git_write`(op=Stage) | explicit relative paths only; `validate_repo_paths` (≤200 paths, no `.git`/abs/`..`/ctrl); refuses whole-repo implicit. **File/path-level, NOT hunk-level.** |
| Unstage | `write_unstage` (`sync.rs:712`) | `request_git_write`(op=Unstage) | `reset_default` to HEAD; file/path-level. |
| Discard | `write_discard` (`sync.rs:740`) | `request_git_write`(op=Discard) | checkout-index force; **refuses untracked** (anti-footgun). |
| Commit | `write_commit` (`sync.rs:756`) | `request_git_write`(op=Commit) | scoped paths or `add_all`;`validate_commit_message` (≤500B, no ctrl/newline); rejects empty commit (index tree == HEAD tree). Author signature hardcoded `极智简单 <mvp@jizhijiandan.local>`. |
| Create branch | `write_create_branch` (`sync.rs:808`) | `request_git_write`(op=CreateBranch) | `validate_branch_name` (check-ref-format); optional checkout; no overwrite. |
| Checkout branch | `checkout_conflict` precheck (`sync.rs:835`) + `set_head`/`checkout_head` | `request_git_write`(op=Checkout) | conflict precheck vs dirty working tree; safe checkout. |
| Push | `push_ahead` (`sync.rs:956`) + `GIT_PUSH_REMOTE="origin"` (`sync.rs:919`) | `request_git_write`(op=Push, **double-confirm**) | **remote name hardcoded `origin`**; no arbitrary remote. |
| Remote sync merge | `three_way_merge` (`sync.rs:157`) | (internal sync flow, M1-5) | automated fetch+merge for workspace push/pull to gitee; not a user-driven merge UI. |

**Confirmable-job gate (safety model, product-owned):** `request_git_write` stages a `GitWriteJob` (TTL, one-shot, pending); destructive ops (Discard) and Push require a second `confirm_git_write` (double-confirm). Audit detail is redacted (`git_write_audit_detail`, `bridge.rs:2214` — no credentials / path lists). This is a **product asset** R3 must preserve when mapping the 14 units.

## 2. Required Git workflow units — per-unit analysis

Convention used for classification (consistent with A10 ledger §0b):
- **ADAPT** = product already implements the unit; adopt rebased/git4idea *behavior* as incremental extension of the existing `sync.rs` module.
- **REIMPLEMENT_FROM_BEHAVIOR** = product has no equivalent; build from rebased/git4idea behavior (default per R3 card).
- **REJECT** = rebased-specific delta not applicable to the browser-OS model.
- **COPY** = 0 (forbidden: Kotlin/JVM Apache-2.0 source cannot be copied into a Rust/MulanPSL-2 binary; A10 provenance gate required first).

### 2.0 Per-unit delivery matrix (R3B required columns)

Each of the 14 units below gives: **target UI placement** (edge tool windows `left`/`right`/`bottom` per A3,
or `center` document area per A1; reversible log placement = D-R1), **existing product symbol** (measured
from canonical `sync.rs`/`bridge.rs` at BASE `d96b9b5`), **required future command/state boundary** (W19,
frozen command IDs live in `A7-R3B-git-command-map.md`), **tests**, **safety** (A6 `SafetyClass`
`safe|mutating|destructive|dangerous` + A9 tier `T0–T4`), and **license/provenance note**.

| # | Unit | Class | Target UI placement | Existing product symbol (`file:line`) | Required future command/state boundary (W19) | Tests | Safety (A6 / A9) | License / provenance note |
|---|---|---|---|---|---|---|---|---|
| 1 | status | ADAPT_PRODUCT | left: Git Commit/Changes (staged/unstaged split) | `read_status` (`sync.rs:354`) / `git_status` (`bridge.rs:2109`) | extend `GitFileStatus` taxonomy (intent-to-add, typechange, copied) + staged/unstaged split in `useGitStore`; new read cmd `git.status.refresh` | extend `sync.rs` status units + `check-git-ui-logic.mjs` split assertions | `safe` (read) / `mutating` refresh (T0) | git4idea behavior observed; product MulanPSL-2; no transplant |
| 2 | hunk staging | REIMPLEMENT_FROM_BEHAVIOR | commit panel hunk selector / inline diff (left or center) | none (only path-level `write_stage`/`write_unstage`) | new `GitWriteOp::{StageHunk,UnstageHunk,DiscardHunk}` + `git2::Patch` partial apply; reuse `request_git_write`→`confirm_git_write` gate | rust partial-patch unit + UI stage/discard assertions | `destructive` discard (T2/T3, confirm); stage/unstage `mutating` (T1) | build from behavior (Apache-2.0 git4idea); no source copy |
| 3 | diff | ADAPT_PRODUCT | bottom: Git Diff (side-by-side) | `read_diff` (`sync.rs:402`) / `git_diff` (`bridge.rs:2123`) | side-by-side + commit-pair + word-level; keep `GIT_DIFF_HARD_CAP=256KiB` | extend diff truncation tests + UI side-by-side | `safe` (read, T0) | extend product; behavior from git4idea |
| 4 | log graph | REIMPLEMENT_FROM_BEHAVIOR | **center document area OR bottom, reversible (D-R1)** — feeds A1/A3 | none (only `graph_ahead_behind`, `sync.rs:969`) | `git_log_graph` read cmd (revwalk + graph) + reversible placement toggle persisted in layout state (A3/A4) | bounded graph render + placement toggle round-trip | `safe` (read) at surface; node actions see U09/U10/U05 | port commit-graph from behavior; Apache-2.0 git4idea |
| 5 | branches | ADAPT_PRODUCT | left: Git Branches popup / bottom | `read_branches` (`sync.rs:503`) + `write_create_branch` (`sync.rs:808`) + `checkout_conflict` (`sync.rs:835`) | branch rename/delete/upstream/compare; `git.branchDelete` destructive + confirm; reuse `validate_branch_name` | extend branch tests + delete confirm UI | create/checkout `mutating` (T1); delete `destructive` (T3; protected T4) | extend product |
| 6 | worktrees | REIMPLEMENT_FROM_BEHAVIOR | left: Git Worktrees | none | `git_worktree_list/add/remove` (git2 `worktree`/`open_from_worktree`); write ops under confirmable-job gate; couples to A4 workspace/doc identity | rust worktree add/remove + UI list | add `mutating` (T1); remove `destructive` (T3) | build from behavior; Apache-2.0 git4idea |
| 7 | stash | REIMPLEMENT_FROM_BEHAVIOR | left: Git Stash / context menu | none | `git_stash_push/pop/apply/drop/list` (git2 `stash_save/pop/apply/drop`); `drop` double-confirm (like Push) | rust stash round-trip + drop confirm UI | push/pop/apply `mutating` (T1); drop `destructive` (T3, double-confirm) | build from behavior |
| 8 | merge | ADAPT_PRODUCT | branch context menu / merge dialog + conflict UI | `three_way_merge` (`sync.rs:157`, automated sync only) | user-driven merge UI + conflict resolver; reuse status/diff; **re-scope `check-git-write-policy.py` FORBIDDEN `.merge(` to allow product merge under gate** | merge + conflict UI; extend merge tests | `destructive` (T3; conflict must complete/abort) | extend product `three_way_merge` |
| 9 | rebase / interactive | REIMPLEMENT_FROM_BEHAVIOR | log-graph context "Rebase" / interactive todo editor | none | `git_rebase` (git2 `Rebase`) + persistent sequencer state (A4) + continue/abort/skip; **re-scope gate to permit `rebase` under confirmable-job** | rust rebase + sequencer persistence + abort | `destructive` HARD (T3; protected T4 + force-push note); double-confirm | build from behavior |
| 10 | cherry-pick | REIMPLEMENT_FROM_BEHAVIOR | log-graph context menu | none | `git_commit_cherry_pick` (git2 `cherrypick`); conflict path reuses U11 | rust cherry-pick + conflict | `mutating` (T1); conflict T1 | build from behavior |
| 11 | conflicts | REIMPLEMENT_FROM_BEHAVIOR | bottom: Git Conflict (3-way editor) | detect-only (`read_status`=`conflicted`) | `git_conflict_list` + `git_conflict_resolve` (3-way `checkout_index`); shared with U08/U09/U10 | conflict list + resolve UI + abort | `mutating` (T1); must complete/abort | build from behavior |
| 12 | history / blame | REIMPLEMENT_FROM_BEHAVIOR | file context "Annotate/History" + bottom | none | `git_blame` (git2 `Blame`) + `git_file_history` (log filtered by path); reuse U04 graph | rust blame + history UI | `safe` (read, T0) | build from behavior |
| 13 | patch | REIMPLEMENT_FROM_BEHAVIOR | log/commit context "Create/Apply Patch" dialog | none (only `read_diff` patch text) | `git_format_patch` (self-written series, no git2 format-patch) + `git_apply_patch` (git2 `apply`); reuses `read_diff` | patch gen + apply round-trip | create `safe` (read); apply `mutating`/`destructive` (T2) | build from behavior; **REJECT transplant of format-patch source** |
| 14 | command log | REIMPLEMENT_FROM_BEHAVIOR | bottom: Git Console (read-only view) | `log_audit` git_write events (redacted, `bridge.rs:2214`) | `git_command_log` read view fed by git2 calls; **keep audit redaction — no credentials / path lists** (A9 secrets boundary) | UI log render + redaction assertions | `safe` (read, T0); redaction mandatory | build from product audit + git4idea console behavior; no donor source |

### 2.1 status
- **Behavior flow**: repo `statuses()` → per-path state (wt/index/new/modified/deleted/renamed/conflicted/typechange/copied/ignored) → UI list with staged/unstaged split.
- **Product**: ADAPT-owned (`read_status`). Gap vs git4idea: no `intent-to-add`, `typechange`, `copied`, no staged/unstaged split columns.
- **Rebased**: inherited `git4idea` `GitStatus` / `GitChangeUtils` (staged vs unstaged tree, changelists).
- **Source identity**: `plugins/git4idea/.../GitStatus.java`, `GitChangeUtils.java`.
- **License**: Apache-2.0. **Class: ADAPT** (extend `read_status` taxonomy; no new module).

### 2.2 hunk staging
- **Behavior flow**: diff → select hunk/line → `git apply --cached` (stage) or `git apply -R` (unstage) on a partial patch; UI shows staged/unstaged side-by-side with editable hunks.
- **Product**: **ABSENT at hunk level** — only `write_stage`/`write_unstage` on whole paths.
- **Rebased**: inherited `git4idea` partial-staging via `GitApply` + `GitFragment` (line/hunk staging in commit dialog / diff viewer).
- **Source identity**: `plugins/git4idea/.../GitApply.java`, `ui/GitCommitAndPushPanel` (hunk selector).
- **License**: Apache-2.0. **Class: REIMPLEMENT_FROM_BEHAVIOR** (build partial-patch staging on top of `git2::Patch`; reuse product path-validation).

### 2.3 diff
- **Behavior flow**: select file/commit → `git diff` (unified/side-by-side/word-level) → navigate hunks.
- **Product**: ADAPT-owned (`read_diff`, per-file patch, truncation, binary). Gap: unified text only, no side-by-side, no word-level, no commit-to-commit diff.
- **Rebased**: inherited `git4idea` `GitDiffProvider` / `GitFileRevision` (side-by-side, three-way merge-diff, word-level).
- **Source identity**: `plugins/git4idea/.../GitDiffProvider.java`, `GitFileRevision.java`.
- **License**: Apache-2.0. **Class: ADAPT** (extend `read_diff` to side-by-side + commit-pair; keep `GIT_DIFF_HARD_CAP`).

### 2.4 log graph
- **Behavior flow**: `git log --graph` → commit DAG rendered as interactive graph → click node → details / file list / subtree filter. **Rebased defining move**: log as main editor OR bottom tool window (reversible, D-R1).
- **Product**: **ABSENT** (only ahead/behind counts).
- **Rebased**: inherited `git4idea` `GitLogProvider` / `GitLogUtil` / `GitCommit` + `ui/GitLogUI` / commit-graph renderer. Plus D-R1 reversible placement.
- **Source identity**: `plugins/git4idea/.../GitLogProvider.java`, `ui/GitLogPanel.java`, `ui/GitLogGraphTable` (graph).
- **License**: Apache-2.0. **Class: REIMPLEMENT_FROM_BEHAVIOR** (port commit-graph + reversible editor/bottom placement; this is the central R3 Git UX piece, feed A1/A3 for placement).

### 2.5 branches
- **Behavior flow**: branch popup → list local/remote, current, upstream; create/rename/delete/checkout/merge/compare; upstream tracking.
- **Product**: ADAPT-owned (`read_branches` + `write_create_branch` + `checkout_conflict`). Gap: no rename, no delete, no upstream (track/pull) binding, no compare.
- **Rebased**: inherited `git4idea` `GitBranch` / `GitBranchWorker` / `GitBranches` popup.
- **Source identity**: `plugins/git4idea/.../GitBranch.java`, `GitBranchWorker.java`.
- **License**: Apache-2.0. **Class: ADAPT** (extend branch ops; reuse product `validate_branch_name` + confirmable gate for delete).

### 2.6 worktrees
- **Behavior flow**: `git worktree list/add/remove` → parallel checkouts in subdirs; switch worktree as a project.
- **Product**: **ABSENT.**
- **Rebased**: inherited `git4idea` has limited worktree support (`GitWorkTree` / `GitWorkTreeProvider`, partially surfaced).
- **Source identity**: `plugins/git4idea/.../GitWorkTree.java` (if present in pinned rev).
- **License**: Apache-2.0. **Class: REIMPLEMENT_FROM_BEHAVIOR** (port list/add/remove; maps to R3 "worktree" tool-window scope — A6 context-menu scope includes "Git … worktree").

### 2.7 stash
- **Behavior flow**: `git stash push/pop/apply/drop/list` → shelve dirty changes; restore later.
- **Product**: **ABSENT.**
- **Rebased**: inherited `git4idea` `GitStash` / `GitStashProvider` / `GitStashUtils`.
- **Source identity**: `plugins/git4idea/.../GitStash.java`.
- **License**: Apache-2.0. **Class: REIMPLEMENT_FROM_BEHAVIOR** (port push/pop/apply/drop/list; route through product confirmable-job gate for drop).

### 2.8 merge
- **Behavior flow**: `git merge <branch>` → fast-forward or 3-way; on conflict → conflict markers + resolve UI; on success → merge commit.
- **Product**: PARTIAL — only automated `three_way_merge` for remote sync (no user-driven branch-branch merge, no resolve UI).
- **Rebased**: inherited `git4idea` `GitMerge` / `GitMergeProvider` / `GitMergeUtil` + `GitConflictResolver`.
- **Source identity**: `plugins/git4idea/.../GitMerge.java`, `GitConflictResolver.java`.
- **License**: Apache-2.0. **Class: ADAPT** (extend `three_way_merge` into a user-driven merge with conflict-resolution UI; reuse product status/diff).

### 2.9 rebase / interactive rebase
- **Behavior flow**: `git rebase [--interactive] <upstream>` → replay commits; interactive = todo list (pick/reword/edit/squash/fixup/drop) edited in editor; conflict → resolve + continue/abort/skip.
- **Product**: **ABSENT.**
- **Rebased**: inherited `git4idea` `GitRebase` / `GitRebaseUtils` / `GitInteractiveRebase` / `GitRebaseEditorHandler`.
- **Source identity**: `plugins/git4idea/.../GitRebase.java`, `GitInteractiveRebase.java`.
- **License**: Apache-2.0. **Class: REIMPLEMENT_FROM_BEHAVIOR** (port rebase + interactive todo editor; high-risk → double-confirm + audit like Push).

### 2.10 cherry-pick
- **Behavior flow**: select commit(s) → `git cherry-pick` → apply diff as new commit; conflict → resolve.
- **Product**: **ABSENT.**
- **Rebased**: inherited `git4idea` `GitCherryPicker`.
- **Source identity**: `plugins/git4idea/.../GitCherryPicker.java`.
- **License**: Apache-2.0. **Class: REIMPLEMENT_FROM_BEHAVIOR** (port multi-commit pick; reuse commit + conflict path).

### 2.11 conflicts
- **Behavior flow**: detect conflict markers → 3-way merge editor (local/base/remote) → accept left/right/both → mark resolved → continue.
- **Product**: DETECT-only (`read_status`=`conflicted`); **no resolution UI.**
- **Rebased**: inherited `git4idea` `GitConflictResolver` (`merge`/`cherry-pick`/`rebase` conflict resolution, 3-way viewer).
- **Source identity**: `plugins/git4idea/.../GitConflictResolver.java`, `ui/GitMergeConflictResolver`.
- **License**: Apache-2.0. **Class: REIMPLEMENT_FROM_BEHAVIOR** (resolution UI; pairs with 2.8/2.9/2.10; feed A6 destructive-confirm + A9 safety).

### 2.12 history / blame
- **Behavior flow**: file → Annotate (blame: line→commit/author/date) + File History (list of commits touching file). Repo → History (all commits).
- **Product**: **ABSENT** (no `git log`, no `git blame`).
- **Rebased**: inherited `git4idea` `GitFileAnnotation` / `GitAnnotationProvider` / `GitHistoryProvider` / `GitFileHistory`.
- **Source identity**: `plugins/git4idea/.../GitFileAnnotation.java`, `GitHistoryProvider.java`.
- **License**: Apache-2.0. **Class: REIMPLEMENT_FROM_BEHAVIOR** (blame = `git2::Blame`; history = log filtered by path; reuse 2.4 graph).

### 2.13 patch
- **Behavior flow**: `git format-patch` (series) / `git diff > file` → export; `git apply` / `git am` → import.
- **Product**: **ABSENT.**
- **Rebased**: inherited `git4idea` `GitPatch` / `GitFormatPatch` / `GitApplyPatch`.
- **Source identity**: `plugins/git4idea/.../GitPatch.java`, `GitFormatPatch.java`.
- **License**: Apache-2.0. **Class: REIMPLEMENT_FROM_BEHAVIOR** (format-patch + apply; reuse `read_diff` patch text + `git2::Patch` apply).

### 2.14 command log
- **Behavior flow**: every git operation appended to a Console/log (command line + stdout/stderr + exit) → review/scroll/copy.
- **Product**: PARTIAL — `log_audit` git_write events (redacted) exist, but no raw command-line console.
- **Rebased**: inherited `git4idea` console (`git4idea.console.GitConsole`) shows each executed `git` command + output; this is rebased's "command log".
- **Source identity**: `plugins/git4idea/.../console/GitConsole.java`, `GitHandler` (command execution).
- **License**: Apache-2.0. **Class: REIMPLEMENT_FROM_BEHAVIOR** (port a command-log view fed by product `git2` calls; keep audit redaction — do NOT log credentials; align with A9 secrets boundary).

## 3. Consolidated feature / command matrix

| # | Unit | Product status | Rebased status (source) | Source identity (module/path) | Class |
|---|---|---|---|---|---|
| 1 | status | owned (`read_status`) | git4idea `GitStatus` | `git4idea/GitStatus.java` | ADAPT |
| 2 | hunk staging | absent (path-only) | git4idea partial-staging | `git4idea/GitApply.java` | REIMPLEMENT |
| 3 | diff | owned (`read_diff`) | git4idea `GitDiffProvider` | `git4idea/GitDiffProvider.java` | ADAPT |
| 4 | log graph | absent | git4idea `GitLogProvider`+graph (+D-R1) | `git4idea/GitLogProvider.java`,`ui/GitLogPanel.java` | REIMPLEMENT |
| 5 | branches | owned (list/create/checkout) | git4idea `GitBranch` | `git4idea/GitBranch.java` | ADAPT |
| 6 | worktrees | absent | git4idea limited | `git4idea/GitWorkTree.java` | REIMPLEMENT |
| 7 | stash | absent | git4idea `GitStash` | `git4idea/GitStash.java` | REIMPLEMENT |
| 8 | merge | partial (auto sync) | git4idea `GitMerge` | `git4idea/GitMerge.java` | ADAPT |
| 9 | rebase / interactive | absent | git4idea `GitRebase` | `git4idea/GitRebase.java` | REIMPLEMENT |
| 10 | cherry-pick | absent | git4idea `GitCherryPicker` | `git4idea/GitCherryPicker.java` | REIMPLEMENT |
| 11 | conflicts | detect-only | git4idea `GitConflictResolver` | `git4idea/GitConflictResolver.java` | REIMPLEMENT |
| 12 | history / blame | absent | git4idea `GitFileAnnotation` | `git4idea/GitFileAnnotation.java` | REIMPLEMENT |
| 13 | patch | absent | git4idea `GitPatch` | `git4idea/GitPatch.java` | REIMPLEMENT |
| 14 | command log | partial (audit only) | git4idea `GitConsole` | `git4idea/console/GitConsole.java` | REIMPLEMENT |

## 4. Classification tally (R3B corrected — now matches the 14-row matrix §3 and §2.0)

| Class | Count | Units | Extends |
|---|---|---|---|
| COPY | 0 | — (forbidden: Kotlin/JVM Apache-2.0 → Rust/MulanPSL-2; A10 provenance gate required first) | — |
| ADAPT_PRODUCT | 4 | status, diff, branches, merge | **this repository only** (`sync.rs`/`bridge.rs` incremental extension) |
| REIMPLEMENT_FROM_BEHAVIOR | 10 | hunk staging, log graph, worktrees, stash, rebase/interactive rebase, cherry-pick, conflicts, history/blame, patch, command log | from observed Rebased/git4idea behavior (no source copy) |
| REJECT | 3 (rebased deltas, **not** part of the 14 units) | D-R2 `.idea` disable, D-R3 TextMate vue bundle, D-R4 cross-platform build | — |

**R3B / A0 ruling correction:** the 14-unit matrix (§3) and the per-unit matrix (§2.0) total exactly
`COPY=0 + ADAPT_PRODUCT=4 + REIMPLEMENT_FROM_BEHAVIOR=10 = 14`. The prior R3 tally mis-counted
`ADAPT=5 / REIMPLEMENT=9`; this is now fixed to `ADAPT_PRODUCT=4 / REIMPLEMENT=10` so the tally matches
the 14-row table (closes A0 `R3B-07`).

**ADAPT_PRODUCT means extend THIS repository:** every ADAPT_PRODUCT unit (status, diff, branches, merge)
is an incremental extension of the product's own `src-tauri/src/sync.rs` + `bridge.rs` implementation
toward the observed Rebased/git4idea *behavior*. It is **never** a transplant of Rebased/JetBrains Kotlin
source. REIMPLEMENT_FROM_BEHAVIOR units build net-new Rust from observed behavior (git2 APIs), again with
no source copy. This keeps the product at MulanPSL-2.0 and avoids the JetBrains OSS Build Terms v1.3
obligations that COPY would trigger (A10 `F01`).

## 5. Constraints & handoff

1. **License retention**: if any REIMPLEMENT unit later lifts a constant/enum verbatim, it must keep Apache-2.0 attribution and ship `LICENSE-APACHE-GIT4IDEA`; product stays MulanPSL-2.0 (A10 BOM §1 gap). The whole `git4idea` module is Apache-2.0; rebased `LICENSE.txt` = JetBrains OSS Build Terms.
2. **No exact source copy**: file:line provenance for git4idea is **not** captured in this map (out of scope for isolated research; blind copy forbidden). A10 must record file-level provenance + NOTICE duties before W19 adoption, per R3 card.
3. **Product safety assets to preserve**: the confirmable-job gate (`request_git_write`/`confirm_git_write`), `GitWriteJob` TTL/one-shot, double-confirm for Discard/Push (extend to rebase/drop/stash-drop), and `git_write_audit_detail` redaction (no credentials/path lists). These map to A6 (context-menu destructive confirm) + A9 (secrets boundary) and must not be lost when porting units 2/6/7/9/10/13/14.
4. **Central Git UX (unit 4 log graph) feeds A1/A3**: R3 requires both Rebased placements (main-editor vs bottom tool window) prototyped and reversible — hand the reversible-placement decision to A1 (consolidated prototype) and A3 (tool-window edges/persistence).
5. **Gap-fill priority for W19**: the 9 REIMPLEMENT units are net-new; the 5 ADAPT units are incremental extensions of `sync.rs`. Effort estimate: REIMPLEMENT units dominate; ADAPT units are low-risk (same module).
6. **A11 manifest hook**: this map's 14-unit matrix + classification is the Git-workflow section of the R3 integration manifest; A11 should record the exact SHAs (`cee14e9` rebased pin) and the user-review checklist covering all 14 units at the 4 target sizes + failure/empty/conflict states.

## 6. R3 card coverage check

- ✅ Study current product Git (§1, measured from `sync.rs`/`bridge.rs` at BASE `d96b9b5`).
- ✅ Study `DetachHead/rebased` (§0 pin `v1.1.15 @ cee14e9` = behavior SSOT; `2896562e` explicitly demoted to research snapshot).
- ✅ Use SourceGit/slio-git to challenge gaps (secondary refs noted §0; no contradiction found — both are full Git GUIs, rebased chosen as primary per card).
- ✅ Pin exact Rebased revision (`cee14e9`) as SSOT; document `2896562e` as later research snapshot only.
- ✅ Separate rebased deltas (D-R1..D-R4) from inherited IntelliJ code (git4idea).
- ✅ Behavior flows (§2) + feature/command matrix (§3) + per-unit delivery matrix (§2.0) for all 14 required units.
- ✅ Each unit marked COPY/ADAPT_PRODUCT/REIMPLEMENT_FROM_BEHAVIOR/REJECT with exact source identity + license/terms (§2/§2.0/§3/§4).
- ✅ Default to behavior reimplementation (REIMPLEMENT_FROM_BEHAVIOR=10, COPY=0); ADAPT_PRODUCT=4 extends this repo only.

## 7. R3B correction delta (what changed vs the R3 report)

| R3B / A0 finding | R3 report state | R3B correction |
|---|---|---|
| `R3B-07` revision identity | only `cee14e9` pinned; `2896562e` absent → looked inconsistent with A10 | §0 now pins `cee14e9` as **behavior SSOT** and `2896562e` as **later research snapshot only** |
| `R3B-07` classification total | §4 tallied `ADAPT=5 / REIMPLEMENT=9` (did not match the 14-row matrix) | §4 fixed to `ADAPT_PRODUCT=4 / REIMPLEMENT_FROM_BEHAVIOR=10`; tally now equals the 14-row matrix |
| A0 ruling "ADAPT_PRODUCT extends this repo" | implied but not explicit | §4 + §2.0 state ADAPT_PRODUCT units extend `sync.rs`/`bridge.rs` only, no donor transplant |
| R3B card §A7 per-unit spec | behavior flows only; no placement/symbol/boundary/tests/safety/provenance per unit | §2.0 adds the full 9-column per-unit delivery matrix |
| A6 command-id freeze (D1 debt) | A6 deferred Git ids to A7 | `A7-R3B-git-command-map.md` freezes the command-id namespace + safety classes consumed by A6 |

### 7.1 Cross-lane handoffs (do not implement in A7)
- **A6**: consumes the frozen command-id namespace + `SafetyClass` from `A7-R3B-git-command-map.md` (resolves A6 `D1`).
- **A1**: consumes §2.0 placements (esp. U04 reversible center↔bottom log, U11 conflict window) for the consolidated prototype.
- **A3/A4**: consume U04 placement-toggle + U06/U09 persistent state (workspace/doc identity, crash recovery).
- **A9**: consumes per-unit `SafetyClass`/tier (§2.0 col 8) for the safety matrix; audit-redaction boundary is mandatory for U14.
- **A10**: records file-level provenance + NOTICE duties before any W19 adoption; `cee14e9` is the cited pin.
- **W19 gate re-scope**: U08/U09 (and stash/rebase/cherry-pick) require `check-git-write-policy.py` FORBIDDEN list
  (`.merge(`, `reset(`, `stash`, `rebase`, `cherry[-_]pick`) to be re-scoped so the *product's own*
  confirmable-job-gated write ops are permitted (they are legitimate product writes, not donor transplants).
