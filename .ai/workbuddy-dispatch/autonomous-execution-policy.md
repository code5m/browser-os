# Autonomous Execution Policy

Adopted by Chief Architect — Phase 03, 2026-09-12. Governs the control flow of all
phase execution. Its purpose is to remove per-step human confirmation and turn the
Chief Architect from an "approver of every pipeline action" into the true Owner who
only decides direction.

## Principle

**No Ask Unless Blocked.** Default to autonomous advance. Do not request Owner
confirmation for routine pipeline actions (commit, next-batch, verification, dispatch).

## Stop Conditions (must block and request Owner decision)

1. P1 / BLOCKER finding.
2. Conflict Report (overlapping or contradictory constraints between agents).
3. Need to modify a forbidden / out-of-scope area.
4. Change to an already-approved architecture or product decision.
5. Destructive / irreversible operation (data loss, migration, force push, rewrite).
6. Two valid options both affect product direction.

Anything outside this list is auto-executed.

## Auto-execute WITHOUT asking

- Advance to the next Batch after `CODE_PASS`.
- Commit verified files (after `git diff --check` / `--self-test` / `--json` pass).
- Run prescribed acceptance commands.
- Treat P3 / WARN non-blocking items as notes, not blockers.
- Tolerate minor checker output-format differences.
- Choose commit message wording.
- Generate the next-stage dispatch card.

## State machine

```
IMPLEMENTING
   -> SELF_TEST_PASS
   -> REVIEW_PASS
   -> COMMIT
   -> NEXT_BATCH
```

Only `BLOCKED` / `CONFLICT` / `P1` / `OWNER_DECISION_REQUIRED` halt the flow.

## Per-Batch pipeline

1. Agent implements.
2. Self-test.
3. Review.
4. Fix non-blocking issues.
5. `CODE_PASS`.
6. Auto commit.
7. Auto next Batch.

No Owner questioning between steps 3–7 unless a Stop Condition is hit.

## Commit hygiene

- Implementation commits and governance-doc commits are SEPARATE. Do not mix
  `scripts/**` with `.ai/**` / `docs/**` in one commit unless both are frozen.
- Phase 03 acceptance requires `git diff --check` to pass for all changed files.
