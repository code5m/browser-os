# A7 — R3 Lane Checkpoint

> Lane: A7 (Full Git workflow reference map)
> Wave: M5-W18-R3-UX
> Date: 2026-09-08
> Worktree: `/home/ainfinit/.codex/worktrees/m5-w18-a7/mvp-browser-os-v3`
> Branch: `codex/m5-w18-a7` (rebased onto `origin/master` = `200f0f1`)
> STATUS: **PASS** (research only; no product code, no other-lane files, no push)

## 1. Startup / entry verification

- Read `WORKSPACE_IDENTITY.md`, `PARALLEL_COMMAND_BOARD.md`, `M5-W18-R3-UX-TASKS-20260908.md` (task files).
- Worktree path matches lane pattern `m5-w18-aN` (N=7) exactly; branch `codex/m5-w18-a7` matches.
- `git status --short --branch` clean before rebase.
- Rebase: `git fetch origin` + `git rebase origin/master`. The 6 prior W18-R/R2B A7 research commits (zvec-grep) were already integrated into `origin/master` (per WORKSPACE_IDENTITY "All A1-A11 R/R2/R2B research commits have been integrated"), so git **skipped** them; branch now sits at `origin/master` `200f0f1` clean.

## 2. Scope performed (A7 R3 lane card: "Full Git workflow reference map")

Deliverable: `logs/research/M5-W18/A7-R3-git-workflow-reference-map.md`
- Pinned exact Rebased revision **v1.1.15 @ `cee14e9`** (2026-09-05, latest release) + license facts (JetBrains OSS Build Terms / Apache-2.0 / NOTICE attributes IntelliJ IDEA).
- Separated rebased's own deltas (D-R1 customizable Git-log placement, D-R2 `.idea` disable, D-R3 TextMate vue, D-R4 cross-platform) from inherited IntelliJ `git4idea` (Apache-2.0).
- Measured current product Git inventory from `src-tauri/src/sync.rs` + `bridge.rs` (read: status/diff/branch_list; write: stage/unstage/discard/commit/create_branch/checkout/push via confirmable-job gate; automated sync merge). Confirmed **no hunk-level staging, no log graph/worktree/stash/rebase/cherry-pick/blame/patch/command-log; conflicts detect-only**.
- Produced behavior flows + feature/command matrix for all 14 required units (status, hunk staging, diff, log graph, branches, worktrees, stash, merge, rebase/interactive rebase, cherry-pick, conflicts, history/blame, patch, command log).
- Classified each unit COPY/ADAPT/REIMPLEMENT/REJECT with module/path source identity + license: **COPY=0, ADAPT=5, REIMPLEMENT=9, REJECT=3** (rebased non-Git deltas). Default = behavior reimplementation.
- Secondary refs SourceGit/slio-git noted (no contradiction; rebased kept primary per card).

## 3. Files written (this lane only)

- `logs/research/M5-W18/A7-R3-git-workflow-reference-map.md` (new, research)
- `logs/research/M5-W18/A7-R3-lane-checkpoint.md` (new, this file)

No edits to `src-tauri/**`, `src/**`, `scripts/**`, `*.toml`, `capabilities/**`, or any other lane's files. No product code, ACL, capability, dependency, or native-runtime change.

## 4. Hard-stop compliance

- ✅ Worktree path = `m5-w18-a7`, branch = `codex/m5-w18-a7` (exact match).
- ✅ Worktree clean (no dirty files from other agents).
- ✅ No product source modified.
- ✅ No push performed (A0 is sole integrator/pusher).
- ✅ Did not touch other lanes' files (only `logs/research/M5-W18/` A7-named files).

## 5. Handoff pointers

- Central Git UX (unit 4 log graph, reversible editor/bottom placement) → **A1** (consolidated prototype) + **A3** (tool-window edges/persistence).
- Safety assets to preserve when porting (confirmable-job gate, double-confirm, audit redaction) → **A6** (context-menu destructive confirm) + **A9** (secrets boundary).
- File-level provenance + NOTICE gate before any W19 adoption → **A10**.
- 14-unit matrix + `cee14e9` pin + user-review checklist → **A11** integration manifest.

## 6. Git commit plan (local only, not pushed)

Single commit on `codex/m5-w18-a7` (at `origin/master` base):
`A7 R3: full Git workflow reference map (rebased pin cee14e9; 14-unit matrix; COPY=0/ADAPT=5/REIMPLEMENT=9)`
