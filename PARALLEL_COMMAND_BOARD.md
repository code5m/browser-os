# Parallel Command Board

> Updated: 2026-09-05 23:35 CST
> Controller: main integration agent
> Canonical directory: `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`
> Current mainline: `master` at `47fce60`
> Current NEXT: `M4-1.c` + `M4-2.s` + `M4-5.d` parallel unblock, then M4 implementation lanes

This file is the coordination board for 12 parallel agents. Do not rely on chat history as the source of truth. Read `WORKSPACE_IDENTITY.md`, then read this file before making changes.

## One-Line Resume Prompt

Use this when assigning a Trae/WorkBuddy agent:

```text
继续 Lane AX，先读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，只按自己的 Lane 执行，不 push。
```

Replace only `AX` with the lane id. The lane-specific work is defined below; do not paste long prompts unless the lane reports ambiguity.

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
- The instruction names one lane but says to follow another lane's scope.

If an instruction says `You are Lane A3` but later says `follow Lane A1`, treat it as a prompt typo. Do not blend scopes. Stop and report the conflict unless the controller has already corrected the lane in `PARALLEL_COMMAND_BOARD.md`.

If the only blocker is an unfinished predecessor lane, do not edit product code. Produce a read-only assist note under `logs/assist/` with findings, risks, likely files, and exact unblock condition.

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
| A3 | M4-2 PoolKind and production safety policy | AI:DEEP / R:xhigh | `src-tauri/src/database*`, `src-tauri/src/domain.rs`, `src-tauri/src/security_policy.rs`, db policy script, Rust tests | connection-pool abstraction, SQL risk classifier, fail-closed write policy, privacy/audit tests | 3 |
| A4 | M4-3 database command layer | AI:DEEP / R:xhigh | `src-tauri/src/database*`, `src-tauri/src/bridge.rs`, `src-tauri/src/main.rs`, `src-tauri/permissions/default-commands.toml`, `src-tauri/src/domain.rs`, `src/bridge.ts`, `src/types.ts`, tests | `connect/query/disconnect`, ACL, source check, Keyring, audit, result limits | 4 |
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
3. A3 starts only after A2 freezes `SupportedDb`, config schema, and dependency choice.
4. A4 starts only after A2 freezes command DTOs and A3 freezes pool/safety interfaces.
5. A7 starts only after A6 freezes `TaskDef` and trigger semantics.
6. A5 and A8 start after their command names and DTOs are stable.
7. A9 may research M5, but must not implement M5 product code before M4 is PASS.
8. A10 reviews security-sensitive M4 lanes after their output exists.
9. A11 prepares verification evidence and manual acceptance checklists before final A0 integration.

## Code Dispatch Now

> A0 integrated and pushed `6d73ce1`, `d6457fe`, and `47fce60`.
> Current facts:
> - A1 M4 expansion is integrated.
> - A2 `M4-1.a` and `M4-1.b` are integrated.
> - A2 `M4-1.d` policy notes are integrated, but `M4-1.c` remains STOPPED and must be completed.
> - A6 M4-5 scheduler contract is integrated.
> - A3/A4/A5/A7/A8 may now write code only where the unblock condition below says `START`.

| Lane | Status | Code Task To Execute Now | Allowed Extra Scope For This Dispatch | Stop / Wait Condition |
|---|---|---|---|---|
| A2 | **START** | Complete `M4-1.c`: database result row/byte/field limits, cancellation, timeout, truncation contract. Prefer docs + constants/type proposal in `domain.rs` only if needed by A3/A4. | `logs/checkpoints/M4-1.c-*.md`, `src-tauri/src/domain.rs` type/constants only | Stop if touching DB implementation, commands, frontend, or scripts is required. |
| A3 | **START AFTER A2 M4-1.c, otherwise LIMITED START** | Implement M4-2 PoolKind foundation: add database module skeleton, pool enum/trait boundary, SQLite path validation hooks, credential-key helper `db:<conn_id>`, no command exposure yet. | `src-tauri/src/database.rs` or `src-tauri/src/database/`, `src-tauri/src/domain.rs`, focused Rust tests | If M4-1.c is still STOPPED, only implement pieces independent of limits/cancel; do not expose commands. |
| A4 | **START** | Implement M4-2.s safety gate first: SQL risk classifier, production verdict, fail-closed write gate pure functions in `security_policy.rs`, plus `check-database-policy.py` and pre-merge hook. Do **not** implement bridge commands until A3 pool boundary exists. | `src-tauri/src/security_policy.rs`, `scripts/check-database-policy.py`, `scripts/pre-merge.sh`, focused tests | Stop before `bridge.rs/main.rs/ACL` command work unless A3 has landed pool/safety interfaces. |
| A5 | **LIMITED START** | Implement database UI pure logic only: connection form validation helpers, result-grid formatting, filter/export helper tests, and component skeleton hidden behind no command calls if DTOs are missing. | `src/components/**`, `src/stores/**`, `src/types.ts`, `scripts/check-database-ui-logic.mjs` | Stop before wiring live `db_*` bridge calls until A4 command DTOs exist. |
| A6 | **START** | Complete `M4-5.d`: scheduler policy script and pre-merge integration for TaskDef/scheduler invariants. No scheduler runtime implementation. | `scripts/check-scheduler-policy.py`, `scripts/pre-merge.sh`, `logs/checkpoints/M4-5.d-*.md` | Stop if product scheduler code is needed. |
| A7 | **START** | Implement M4-6 scheduler backend foundation: `TaskDef`/`TaskRunRecord` concrete types if absent, `tasks.json` atomic persistence, validation, `task_list/add/update/remove/run_now` command shell with source check/ACL. Then M4-7 shutdown hook using existing coordinator. | `src-tauri/src/scheduler.rs` or `src-tauri/src/scheduler/`, `src-tauri/src/domain.rs`, `src-tauri/src/bridge.rs`, `src-tauri/src/main.rs`, `src-tauri/permissions/default-commands.toml`, tests | Must reuse M2-4 `script_runner`; no second execution path. If A6 policy script conflicts, stop and report. |
| A8 | **LIMITED START** | Implement scheduler UI pure logic and component skeleton: trigger editor, missed-run/retry controls, next-run/history display helpers, UI tests. | `src/components/**`, `src/stores/**`, `src/types.ts`, `scripts/check-scheduler-ui-logic.mjs` | Stop before wiring live `task_*` bridge calls until A7 command DTOs exist. |
| A9 | **START READ/WRITE DOCS ONLY** | Prepare M5 split after M4: knowledge graph, protocol, agent/plugin lanes, dependency blockers. | `logs/assist/`, future task-card docs only | No M5 product code before M4 PASS. |
| A10 | **START** | Review A2/A3/A4/A6/A7 security-sensitive changes as they land. May add failing policy samples, but product-code fixes require A0 assignment. | `logs/assist/`, policy gap docs; if A0 assigns, policy scripts only | Do not edit product code by default. |
| A11 | **START** | Maintain verification matrix and manual/GUI checklist; update after each implementation lane output. | `logs/assist/`, `logs/checkpoints/` verification docs | Do not edit product code. |
| A12 | **START** | Cross-lane conflict scan: stale NEXT, empty files, lane ownership drift, duplicate command names, doc/code mismatch. | `logs/assist/A12-*.md` | Do not edit product code. |

## Lane-Specific Minimal Prompts

Use exactly one line per agent:

```text
继续 Lane A2，先读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，只按自己的 Lane 执行，不 push。
继续 Lane A3，先读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，只按自己的 Lane 执行，不 push。
继续 Lane A4，先读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，只按自己的 Lane 执行，不 push。
继续 Lane A5，先读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，只按自己的 Lane 执行，不 push。
继续 Lane A6，先读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，只按自己的 Lane 执行，不 push。
继续 Lane A7，先读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，只按自己的 Lane 执行，不 push。
继续 Lane A8，先读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，只按自己的 Lane 执行，不 push。
继续 Lane A9，先读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，只按自己的 Lane 执行，不 push。
继续 Lane A10，先读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，只按自己的 Lane 执行，不 push。
继续 Lane A11，先读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，只按自己的 Lane 执行，不 push。
继续 Lane A12，先读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，只按自己的 Lane 执行，不 push。
```

## Dispatch Waves

### Wave 1: Start Now

These lanes may run immediately from `f376346`:

- A1: Expand M4 cards and dependency graph.
- A2: Freeze M4-1 database contract.
- A6: Freeze M4-5 scheduler contract.
- A9: M5 prework only, no product code.
- A10: Baseline security review of existing M4 plans and prior gates; product-code changes forbidden.

### Wave 2: Start After Contract Output Exists

These lanes should not edit product code until their unblock condition is true:

- A3: Start after A2 delivers `SupportedDb`, config schema, dependency choice, and database safety contract.
- A4: Start after A2 delivers command DTOs and A3 delivers pool/safety interfaces.
- A7: Start after A6 freezes `TaskDef`, trigger kind, missed-run policy, and cancellation semantics.

### Wave 3: Start After Command DTOs Exist

These lanes should wait for stable backend command names and DTOs:

- A5: Start after A2/A3/A4 freeze database command DTOs.
- A8: Start after A6/A7 freeze scheduler command DTOs.
- A11: Start after at least one implementation lane returns output; before that it may only draft a verification matrix in `logs/assist/`.

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
