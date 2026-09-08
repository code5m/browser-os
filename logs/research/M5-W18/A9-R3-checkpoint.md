# A9 — Lane Checkpoint (M5-W18-R3-UX Interaction-Safety Review)

```text
LANE=A9
STATUS=PASS
MODE=RESEARCH_AND_PROTOTYPE
BASE=200f0f1   (origin/master HEAD; branch rebased --onto origin/master, duplicate R2B commits dropped)
HEAD=<set by commit>
REFERENCE=M5-W18-R3-UX-TASKS-20260908.md, M5-W18-PROTOTYPE-REVIEW-20260908.md, WORKBENCH_BLUEPRINT-20260908.md, logs/checkpoints/A0-M5-W18-R2B-integration-audit-20260908.md, WORKSPACE_IDENTITY.md, PARALLEL_COMMAND_BOARD.md
GIT_REFERENCE=DetachHead/rebased (primary Git behavior model, Apache-2.0); sourcegit/sliogit (secondary)
FILES=logs/research/M5-W18/A9-R3-interaction-safety.md, logs/research/M5-W18/A9-R3-checkpoint.md
SURFACES=context menus, drag/drop, Git history rewriting, database writes, external navigation
DELIVERABLES=interaction-safety threat model + single safety framework (safety_class, T0-T4 tiers, dirty checks, protected-branch policy, undo/abort, source checks, audit redaction, secrets boundaries)
PRODUCT_CODE_CHANGED=no
ACL_CAPABILITY_CHANGED=no
USER_VAULT_CHANGED=no
PUSHED=no (A0 integrates)
```

## A9 R3 summary

R3 interaction-safety review for the browser-first workbench shell (Chrome-like + IDEA
progressive disclosure, full Git workflow per `DetachHead/rebased`). This report is the
**user-facing action-safety** companion to the R2B zvec-grep trust-boundary audit
(`A9-threat-model.md`); it does not duplicate that engine/network analysis.

**Five surfaces threat-modeled** with a single reusable safety framework:
1. **Context menus** — scoped to object under pointer; every action carries a `safety_class`
   + disabled reason + keyboard/command-palette parity (consumed by A6's registry).
2. **Drag/drop** — `accepts` contract per target; cross-workspace-root moves are
   `DESTRUCTIVE_HARD` (T3); invalid drops are no-op with reason.
3. **Git history rewriting** — amend/interactive-rebase/squash/reset/push/force-push/branch
   delete/cherry-pick/revert mapped to Rebased/IDEA behavior; dirty-tree gate; force-push
   OFF by default; revert preferred over reset; reflog recovery echo.
4. **Database writes** — WHERE-less DELETE/UPDATE guard (T3), DROP/TRUNCATE T4 on non-empty,
   production-signal gating (A0 #9), real backend cancellation (A0 #8, not flag-only), secrets
   redacted from audit.
5. **External navigation** — workspace root as trust boundary (no `..` escape), debug/release
   origin integrity per `WORKSPACE_IDENTITY.md`, remote page cannot raise native IPC tier,
   egress consent fail-closed (R2B B1 pattern).

**Framework (single source):** `safety_class` ∈ {SAFE, MUTATES_LOCAL, DESTRUCTIVE_SOFT,
DESTRUCTIVE_HARD, EXTERNAL_EGRESS}; confirmation tiers T0–T4 with **Cancel-focused** default
for T3/T4 (no Enter-to-destroy); dirty-document / dirty-tree / dirty-result checks;
protected-branch policy (mainline protected, force-push disabled by default); undo/abort
paths (soft undo + cancel token + tombstone/reflog recovery, no in-app undo for irreversible);
source-independent tiering (shortcuts/scripts do not lower tiers); audit redaction fail-closed
+ secrets boundaries (keyring-only, no secret-bearing URL/SQL persistence).

**Threat model:** 12 threats (T1–T12) across the five surfaces with class/tier controls.
**Alignment:** adopts A0 rulings #8 (real DB cancel) and #9 (keyring-only credentials);
depends on A4 (persistence/dirty DTO), A5 (DB cancellation backend), A6 (context-menu
registry), A3 (progressive disclosure); hands off to A10 for contradiction/ownership review.

**Constraints honored:** no product source / ACL / capability / manifest / user-vault change;
deliverables are design evidence only under the R3 research boundary; STATUS=PASS with W19
implementation deferred per board (`W19=CLOSED`).

## Verification

- `git diff --check` on the new files: clean (no trailing whitespace / conflict markers).
- Worktree rebase: `git rebase --onto origin/master 66bd511 codex/m5-w18-a9` landed branch
  exactly at `origin/master` `200f0f1`, no ahead/behind, working tree clean.
- Files added are confined to `logs/research/M5-W18/` (A9's assigned research path).
- No overlap with other lanes' files (A1 prototype HTML, A6 context-menu inventory, A4
  persistence contract, A5 DB loop) — this report references them but edits none.

## NEXT

W19 stays `CLOSED`. If A0 opens the workbench shell slice (S1) and the Git/DB slices
(S2/S4), the safety framework here becomes the input for: (1) A6's command-registry safety
classes, (2) A4's dirty/document DTO, (3) A5's DB-write confirmation + cancellation UI,
(4) Git history-rewrite dialogs, (5) external-navigation consent. A10 reviews this report
for contradictory ownership before A1 finalizes.
