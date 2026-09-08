# A7 — R3B Git Command-ID & Safety Map (A6 handoff, resolves A6 `D1`)

> Lane: A7 (R3B). Wave: M5-W18-R3B.
> Companion: `A7-R3-git-workflow-reference-map.md` §2.0 (per-unit placement/boundary/tests).
> Consumer: **A6** freezes these ids/labels/`SafetyClass` in the `CommandRegistry` (A6 §4.5 placeholder
> list becomes authoritative here). **No product code is changed in A7** — this is the contract W19
> implements; A6 only records it.

## 0. Conventions

- **Scope groups** (A6 `CommandDef.group`): `git.repo`, `git.branch`, `git.commit`, `git.file`, `git.hunk`,
  plus new `git.log`, `git.worktree`, `git.stash`, `git.conflict`, `git.patch`, `git.console`.
- **`SafetyClass`** (A6 §3 schema): `safe` | `mutating` | `destructive` | `dangerous`.
- **Tier** (A9 §4): `T0` (no confirm) … `T4` (typed confirm / workspace policy). Destructive ops route through
  the product's existing `request_git_write`→`confirm_git_write` confirmable-job gate (double-confirm for
  `destructive`/`dangerous` and remote egress).
- **ACL insertion rule** (per A0/A6 + M5 memory): new `tauri::command`s MUST be registered in
  `src-tauri/src/main.rs` `generate_handler!`, surfaced in `src/bridge.ts` + `src/types.ts`, and allowed in
  `src-tauri/permissions/default-commands.toml` **inserted before the trailing `list_artifact_images` line**
  (the ACL tail is invariant). No command leaves the bundle without an ACL entry.
- **Audit redaction**: `git_write_audit_detail` (`bridge.rs:2214`) redacts `paths/diff/content/credential/token/secret`
  — mandatory for every write op and for `git.console` (U14).

## 1. Frozen command-id table

| id | scope | SafetyClass | Tier | confirm | Unit (§2.0) | notes / existing product symbol |
|---|---|---|---|---|---|---|
| `git.status.refresh` | git.repo | `safe` | T0 | no | U01 | new read; extends `read_status` (`sync.rs:354`) |
| `git.fetch` | git.repo | `mutating` | T1 | no | — | new read/network; needs keyring creds (A9) |
| `git.pull` | git.repo | `mutating` | T1 | no | — | reuses `three_way_merge` (`sync.rs:157`) + conflict UI |
| `git.push` | git.repo | `destructive`+`dangerous` (egress) | T4 | double | U08-adj | `push_ahead` (`sync.rs:956`), remote `origin` hardcoded; non-default remote T4 |
| `git.log.graph` | git.log | `safe` | T0 | no | U04 | new read (revwalk); reversible placement center↔bottom (D-R1) |
| `git.log.placement.toggle` | git.log | `safe` | T0 | no | U04 | persists center/bottom choice (A3/A4) |
| `git.blame` | git.log | `safe` | T0 | no | U12 | new read (`git2::Blame`) |
| `git.file.history` | git.log | `safe` | T0 | no | U12 | new read (log filtered by path) |
| `git.worktree.list` | git.worktree | `safe` | T0 | no | U06 | new read (`worktree`/`worktrees`) |
| `git.worktree.add` | git.worktree | `mutating` | T1 | single | U06 | new write (gate); couples A4 doc identity |
| `git.worktree.remove` | git.worktree | `destructive` | T3 | single | U06 | new write (gate) |
| `git.stash.push` | git.stash | `mutating` | T1 | no | U07 | new write (`stash_save`) |
| `git.stash.pop` | git.stash | `mutating` | T1 | no | U07 | new write (`stash_pop`) |
| `git.stash.apply` | git.stash | `mutating` | T1 | no | U07 | new write (`stash_apply`) |
| `git.stash.drop` | git.stash | `destructive` | T3 | double | U07 | new write (`stash_drop`); double-confirm like Push |
| `git.stash.list` | git.stash | `safe` | T0 | no | U07 | new read |
| `git.branch.list` | git.branch | `safe` | T0 | no | U05 | `read_branches` (`sync.rs:503`) |
| `git.branch.new` | git.branch | `mutating` | T1 | no | U05 | `write_create_branch` (`sync.rs:808`) |
| `git.branch.checkout` | git.branch | `mutating` | T1 | no | U05 | `checkout_conflict`+`set_head` (`sync.rs:835`) |
| `git.branch.merge` | git.branch | `destructive` | T3 | single | U08 | user-driven merge (reuses `three_way_merge`) |
| `git.branch.delete` | git.branch | `destructive` | T3 | single | U05 | new write; protected-branch T4 + reflog note |
| `git.branch.rename` | git.branch | `mutating` | T1 | no | U05 | new write |
| `git.branch.upstream` | git.branch | `mutating` | T1 | no | U05 | new write (track/pull bind) |
| `git.commit` | git.commit | `mutating` | T1 | no | U08-adj | `write_commit` (`sync.rs:756`) |
| `git.commit.amend` | git.commit | `destructive` | T2 | single | U08-adj | new write (rewrites SHA); prior-HEAD echo + reflog (A9) |
| `git.commit.revert` | git.commit | `mutating` | T0/T1 | no | U09-adj | new write; **preferred over reset** (A9) |
| `git.commit.reset` | git.commit | `destructive` | T2/T3 | single | U09-adj | new write (`reset`); protected T3; hard T3 + reflog |
| `git.commit.cherrypick` | git.commit | `mutating` | T1 | no | U10 | new write (`cherrypick`); conflict path → U11 |
| `git.commit.checkout` | git.commit | `mutating` | T1 | no | U04-adj | new write (detached HEAD) |
| `git.file.stage` | git.file | `mutating` | T1 | no | U01-adj | `write_stage` (`sync.rs:689`) |
| `git.file.unstage` | git.file | `mutating` | T1 | no | U01-adj | `write_unstage` (`sync.rs:712`) |
| `git.file.discard` | git.file | `destructive` | T2 | single | U01-adj | `write_discard` (`sync.rs:740`); refuses untracked |
| `git.file.diff` | git.file | `safe` | T0 | no | U03 | `read_diff` (`sync.rs:402`) |
| `git.file.ignore` | git.file | `mutating` | T1 | no | U01-adj | new write (.gitignore) |
| `git.hunk.stage` | git.hunk | `mutating` | T1 | no | U02 | new write (`StageHunk`) |
| `git.hunk.unstage` | git.hunk | `mutating` | T1 | no | U02 | new write (`UnstageHunk`) |
| `git.hunk.discard` | git.hunk | `destructive` | T2/T3 | single | U02 | new write (`DiscardHunk`) |
| `git.rebase` | git.commit | `destructive` | T3 | double | U09 | new write (`Rebase`); protected T4 + force-push note |
| `git.rebase.continue` | git.commit | `mutating` | T1 | no | U09 | new write (sequencer) |
| `git.rebase.abort` | git.commit | `mutating` | T1 | no | U09 | new write; restores pre-rebase HEAD |
| `git.rebase.skip` | git.commit | `mutating` | T1 | no | U09 | new write |
| `git.conflict.list` | git.conflict | `safe` | T0 | no | U11 | new read (`IndexConflict`) |
| `git.conflict.resolve` | git.conflict | `mutating` | T1 | no | U11 | new write (3-way `checkout_index`) |
| `git.conflict.abort` | git.conflict | `mutating` | T1 | no | U11 | new write |
| `git.patch.create` | git.patch | `safe` | T0 | no | U13 | new read (self-written series; no git2 format-patch) |
| `git.patch.apply` | git.patch | `destructive` | T2 | single | U13 | new write (`git2::apply`) |
| `git.console` | git.console | `safe` | T0 | no | U14 | new read view; **redaction mandatory** (no creds/paths) |

## 2. Safety-class ↔ A9 tier mapping (single source)

- `safe` → T0 (read-only; `git.status.refresh`, `git.log.*`, `git.blame`, `git.file.history`, `git.worktree.list`,
  `git.stash.list`, `git.branch.list`, `git.file.diff`, `git.conflict.list`, `git.patch.create`, `git.console`).
- `mutating` → T1 (single confirm optional; `git.fetch/pull/stash.push/pop/apply`, `git.branch.new/checkout/rename/upstream`,
  `git.commit`, `git.commit.revert/cherrypick`, `git.commit.checkout`, `git.file.stage/unstage/ignore`, `git.hunk.stage/unstage`,
  `git.worktree.add`, `git.rebase.continue/abort/skip`, `git.conflict.resolve/abort`).
- `destructive` → T2/T3 (single or double confirm; `git.file.discard`, `git.hunk.discard`, `git.commit.amend/reset`,
  `git.worktree.remove`, `git.stash.drop`, `git.branch.merge/delete`, `git.conflict.*`, `git.patch.apply`).
- `dangerous` → T4 (typed branch-name confirm / workspace policy; `git.push` egress, `git.rebase` on protected branch).

## 3. W19 gate re-scope requirement (critical)

The current `scripts/check-git-write-policy.py` `FORBIDDEN_IN_WRITE_SECTION` blacklists
`.merge(`, `reset(`, `stash`, `rebase`, `cherry[-_]pick` inside the product write section. For the 10
REIMPLEMENT units (and ADAPT merge) these must become **product-owned, confirmable-job-gated write ops**.
W19 must therefore re-scope the gate so that the product's own `GitWriteOp` variants for merge/reset/stash/rebase/
cherry-pick are whitelisted (permitted under `request_git_write`→`confirm_git_write`), while the blacklist
continues to forbid *raw* `Command::new("git"…)` / shell git and *ungated* direct API calls. This is a
legitimate product-write enablement, not a donor transplant — consistent with `ADAPT_PRODUCT`/`REIMPLEMENT`
definitions in the reference map §4.

## 4. Acceptance hooks for A11

- A11 records `cee14e9` as the Rebased pin and `2896562e` as research snapshot only.
- A11 verifies the 14-unit matrix totals `COPY=0 / ADAPT_PRODUCT=4 / REIMPLEMENT=10` (this map + reference map §4).
- A11 verifies every context-menu Git action has a registry id from §1, a disabled-reason hook, a keyboard/palette
  path, and a `SafetyClass` (A6 contract).
