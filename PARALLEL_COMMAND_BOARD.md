# Parallel Command Board

> Updated: 2026-09-05 22:35 CST
> Controller: main integration agent
> Canonical directory: `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`
> Current mainline: `master` at `c83fde2`
> Current NEXT: `M4-1 数据库驱动与生成契约`

This file is the coordination board for 12 parallel agents. Do not rely on chat history as the source of truth. Read `WORKSPACE_IDENTITY.md`, then read this file before making changes.

## Startup Gate

Every agent must run:

```bash
cat .workspace-identity
pwd
git status --short --branch
git log --oneline -12
```

Hard stop if:

- `pwd` does not match the assigned `WORKDIR`.
- The branch/base does not match the assigned lane.
- The worktree is dirty with changes from another lane.
- The task would require changing files outside the lane's allowed scope.
- The lane depends on a contract that is not yet frozen.

## Merge Rule

Only Lane A0 commits and pushes to `master`.

Other lanes must deliver one of these:

- A clean branch/worktree with a small commit series.
- A patch file plus a checkpoint.
- A review/checkpoint document with no product-code changes.

No lane may force-push, reset, or overwrite another lane's changes.

## Active Lanes

| Lane | Purpose | Route | Allowed Scope | Must Deliver | Merge Order |
|---|---|---|---|---|---|
| A0 | Integration controller | AI:DEEP / R:xhigh | All files, only for merge/verification | merge log, final verification, push | Always last |
| A1 | M4 task-card expansion | AI:DEEP / R:high | `AI-模型切换与接手清单.md`, `详细设计与实施计划.md`, `后续需求TODO.md`, `logs/checkpoints/` | M4-1~M4-8 expanded cards, dependency graph | 1 |
| A2 | M4-1 database contract | AI:DEEP / R:high | docs, `src-tauri/src/domain.rs` type proposal only, policy notes | `SupportedDb`, config schema, dependency decision, checkpoint | 2 |
| A3 | M4-2 database backend core | AI:DEEP / R:xhigh | `src-tauri/src/database*`, `src-tauri/src/domain.rs`, `Cargo.toml`, tests | pool/session skeleton, introspection/query core, tests | 4 |
| A4 | M4-3 database safety gate | AI:DEEP / R:xhigh | `src-tauri/src/security_policy.rs`, db policy script, Rust tests, ACL review | SQL risk classifier, fail-closed rules, privacy/audit tests | 3 |
| A5 | M4-4 database UI | AI:BALANCED / R:high | `src/components/**`, `src/stores/**`, `src/bridge.ts`, `src/types.ts`, UI logic tests | connection form, editor, result grid, dangerous confirmation | 6 |
| A6 | M4-5 scheduler contract | AI:DEEP / R:high | docs, pure domain type proposal, policy notes | `TaskDef` contract, clock/missed-run/cancel semantics | 2 |
| A7 | M4-6/M4-7 scheduler backend | AI:DEEP / R:xhigh | `src-tauri/src/scheduler*`, `bridge.rs`, `domain.rs`, ACL, tests | task CRUD, atomic persistence, scheduler shutdown, audit | 5 |
| A8 | M4-8 scheduler UI | AI:BALANCED / R:medium | `src/components/**`, `src/stores/**`, `src/bridge.ts`, `src/types.ts`, UI tests | CRUD, enable switch, next run, history, retry state | 7 |
| A9 | M5 prework only | AI:DEEP / R:high | `logs/assist/`, future task-card docs only | knowledge graph / Agent / plugin prework, no product code | Not merged before M4 |
| A10 | M4 security review | AI:DEEP / R:xhigh | review notes, policy gaps, `logs/assist/`, no product code unless assigned by A0 | independent review of A2/A3/A4/A7 guardrails | After relevant lane |
| A11 | M4 verification evidence | AI:BALANCED / R:medium | verification logs/checkpoints, manual test checklist, no product code unless assigned by A0 | targeted command matrix, GUI/manual checklist, debt ledger | Before A0 final push |

## Current Priority

1. A1 expands M4 cards and freezes the execution order.
2. A2 and A6 freeze contracts in parallel.
3. A4 starts safety policy from A2 draft and existing M0/M1/M2 gates.
4. A3 starts only after A2 names the database crate/schema.
5. A7 starts only after A6 freezes `TaskDef` and trigger semantics.
6. A5 and A8 start after their command names and DTOs are stable.
7. A9 may research M5, but must not implement M5 product code before M4 is PASS.
8. A10 reviews security-sensitive M4 lanes after their output exists.
9. A11 prepares verification evidence and manual acceptance checklists before final A0 integration.

## File Conflict Map

High-conflict files:

- `src-tauri/src/bridge.rs`
- `src-tauri/src/domain.rs`
- `src-tauri/src/main.rs`
- `src-tauri/permissions/default-commands.toml`
- `src/bridge.ts`
- `src/types.ts`
- `src/stores/*`
- `详细设计与实施计划.md`
- `后续需求TODO.md`
- `AI-模型切换与接手清单.md`
- `scripts/pre-merge.sh`

If two lanes need the same high-conflict file, the earlier merge-order lane lands first. Later lanes must rebase or reapply after A0 integrates.

## M4 Guardrails

- Database writes default to reject.
- Production detection uncertainty is fail-closed.
- Credentials must use Keyring and must not enter logs, frontend state, ordinary files, audit details, or checkpoints.
- SQL results must have row/byte limits and cancellation.
- Scheduler must reuse M2-4 execution channels; no second shell/process execution path.
- Scheduler shutdown must be registered through the existing lifecycle coordinator.
- All new commands must pass source check and be added to ACL.
- Every product-code lane must add or extend policy scripts and tests.

## Lane Output Template

Each lane should finish with:

```text
LANE=<A1..A11>
STATUS=PASS | PASS_WITH_DEBT | BLOCKED
BASE=<commit>
HEAD=<commit or patch path>
FILES=<changed files>
VERIFY=<commands and results>
CHECKPOINT=<logs/checkpoints/...md or logs/assist/...md>
MERGE_NOTES=<conflicts, dependencies, risks>
NEXT=<next lane/checkpoint>
```

## A0 Integration Checklist

Before each merge:

```bash
git fetch origin
git status --short --branch
git log --oneline --left-right --cherry-pick origin/master...master
```

After each merge:

```bash
cargo test <targeted>
npm run build
python3 <new-policy-script> --self-test
python3 <new-policy-script>
bash scripts/pre-merge.sh
git diff --check
git status --short --branch
```

Before push:

```bash
cargo test
cargo build --release
npm run build
bash scripts/pre-merge.sh
git diff --check
git status --short --branch
git push origin master
```
