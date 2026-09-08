# A7 — R3 Full Git Workflow Reference Map

> Lane: A7 (Full Git workflow reference map)
> Wave: M5-W18-R3-UX (`RESEARCH_AND_PROTOTYPE`)
> Date: 2026-09-08
> BASE: `codex/m5-w18-a7` rebased onto `origin/master` (`200f0f1`)
> Mode: **research only** — no product source / ACL / capability / dependency changes, no push.
> Companion: `A7-R3-lane-checkpoint.md`

## 0. Pinned upstream references (exact revision)

| Reference | Identity | Pinned revision | Date | License / terms |
|---|---|---|---|---|
| **Rebased** (primary) | `DetachHead/rebased` — Git client on IntelliJ platform, fork of `JetBrains/intellij-community` | **v1.1.15 @ `cee14e9`** (latest release as of 2026-09-05; 519,892 commits in fork history) | 2026-09-05 | Root `LICENSE.txt` = **JetBrains Open-Source Build Terms**; bundled OSS (incl. git4idea) **Apache-2.0**; `NOTICE.txt` attributes IntelliJ IDEA. |
| Rebased inherited core | `JetBrains/intellij-community` — `plugins/git4idea` (the Git VCS integration module) | same revision boundary as the rebased pin (git4idea is carried verbatim from upstream Community at fork point; rebased does not reimplement it) | — | **Apache-2.0** (IntelliJ IDEA Community Edition License) |
| **SourceGit** (secondary) | `sourcegit-scm/sourcegit` | n/a (used only to challenge gaps) | — | MIT |
| **slio-git** (secondary) | `slio-git` | n/a (used only to challenge gaps) | — | MIT / Apache-2.0 |

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

## 4. Classification tally

| Class | Count | Units |
|---|---|---|
| COPY | 0 | — (forbidden: Kotlin/JVM Apache-2.0 → Rust/MulanPSL-2; A10 provenance gate required first) |
| ADAPT | 5 | status, diff, branches, merge (extend existing `sync.rs`) |
| REIMPLEMENT_FROM_BEHAVIOR | 9 | hunk staging, log graph, worktrees, stash, rebase/interactive, cherry-pick, conflicts, history/blame, patch, command log |
| REJECT | 3 (rebased deltas, not Git units) | D-R2 `.idea` disable, D-R3 TextMate vue bundle, D-R4 cross-platform build |

**Note on ADAPT vs REIMPLEMENT:** because the product is Rust/Tauri and rebased is Kotlin/JVM, "ADAPT" here means *adapt the product's existing `sync.rs` implementation toward the rebased/git4idea behavior* (incremental, same language). "REIMPLEMENT" means *build from behavior* (product has no equivalent). No Kotlin source is copied in either case — consistent with R3 card "Default to behavior reimplementation" and the A10 ledger §6 cross-cutting transplant constraints.

## 5. Constraints & handoff

1. **License retention**: if any REIMPLEMENT unit later lifts a constant/enum verbatim, it must keep Apache-2.0 attribution and ship `LICENSE-APACHE-GIT4IDEA`; product stays MulanPSL-2.0 (A10 BOM §1 gap). The whole `git4idea` module is Apache-2.0; rebased `LICENSE.txt` = JetBrains OSS Build Terms.
2. **No exact source copy**: file:line provenance for git4idea is **not** captured in this map (out of scope for isolated research; blind copy forbidden). A10 must record file-level provenance + NOTICE duties before W19 adoption, per R3 card.
3. **Product safety assets to preserve**: the confirmable-job gate (`request_git_write`/`confirm_git_write`), `GitWriteJob` TTL/one-shot, double-confirm for Discard/Push (extend to rebase/drop/stash-drop), and `git_write_audit_detail` redaction (no credentials/path lists). These map to A6 (context-menu destructive confirm) + A9 (secrets boundary) and must not be lost when porting units 2/6/7/9/10/13/14.
4. **Central Git UX (unit 4 log graph) feeds A1/A3**: R3 requires both Rebased placements (main-editor vs bottom tool window) prototyped and reversible — hand the reversible-placement decision to A1 (consolidated prototype) and A3 (tool-window edges/persistence).
5. **Gap-fill priority for W19**: the 9 REIMPLEMENT units are net-new; the 5 ADAPT units are incremental extensions of `sync.rs`. Effort estimate: REIMPLEMENT units dominate; ADAPT units are low-risk (same module).
6. **A11 manifest hook**: this map's 14-unit matrix + classification is the Git-workflow section of the R3 integration manifest; A11 should record the exact SHAs (`cee14e9` rebased pin) and the user-review checklist covering all 14 units at the 4 target sizes + failure/empty/conflict states.

## 6. R3 card coverage check

- ✅ Study current product Git (§1, measured from `sync.rs`/`bridge.rs`).
- ✅ Study `DetachHead/rebased` (§0 pin `cee14e9`; §0 inheritance boundary).
- ✅ Use SourceGit/slio-git to challenge gaps (secondary refs noted §0; no contradiction found — both are full Git GUIs, rebased chosen as primary per card).
- ✅ Pin exact Rebased revision (`cee14e9`).
- ✅ Separate rebased deltas (D-R1..D-R4) from inherited IntelliJ code (git4idea).
- ✅ Behavior flows (§2) + feature/command matrix (§3) for all 14 required units.
- ✅ Each unit marked COPY/ADAPT/REIMPLEMENT/REJECT with exact source identity + license/terms (§2/§3/§4).
- ✅ Default to behavior reimplementation (REIMPLEMENT=9, COPY=0).
