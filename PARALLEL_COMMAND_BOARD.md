# Parallel Command Board

> Updated: 2026-09-05 23:55 CST
> Controller: main integration agent
> Canonical directory: `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`
> Current mainline: `master` at `a75ba24` locally, remote may lag if A0 has not pushed
> Current NEXT: batch implementation mode for M4 database + scheduler lanes

This file is the coordination board for 12 parallel agents. Do not rely on chat history as the source of truth. Read `WORKSPACE_IDENTITY.md`, then read this file before making changes.

## One-Line Resume Prompt

Use this when assigning a Trae/WorkBuddy agent:

```text
继续 Lane AX，先读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 Batch Implementation Dispatch 做完整代码包，自行 rebase/整理补丁，不 push。
```

Replace only `AX` with the lane id. The lane-specific work is defined below; do not paste long prompts unless the lane reports ambiguity.

## Batch Implementation Rule

To save A0 integration time and avoid tiny partial drops, every coding lane must work in **batch mode**:

1. Start with `git fetch origin` and `git pull --ff-only` when possible.
2. If local changes already exist, inspect them first; keep your own lane changes, do not overwrite another lane.
3. If `pull --ff-only` fails because your lane has local work, finish your lane package, then create a patch with:

```bash
git diff --binary > logs/checkpoints/Lane-AX-<task>-<YYYYMMDD-HHMM>.patch
```

4. A product-code lane must not stop after only a note unless blocked by a hard dependency. It should deliver a coherent code package with tests, policy updates, and a checkpoint.
5. Do not create empty files. If a file is intentionally a placeholder, it must contain `STATUS=BLOCKED` and the exact unblock condition.
6. Do not ask A0 to merge after every small file. Continue until the lane's Must Deliver is complete or a hard blocker is proven.
7. At finish, the lane must leave either:
   - a clean lane branch/commit series, or
   - a single patch file plus checkpoint, with all verification results copied into the checkpoint.
8. Only A0 pushes to remote. Other lanes may rebase/pull/commit locally inside their own lane worktree, but must not push.

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

## Batch Implementation Dispatch

This section supersedes the short `Code Dispatch Now` table for the next parallel wave. Agents should execute the largest safe chunk in their lane, verify it, and leave one coherent deliverable.

### Lane A2: Finish M4-1 Contract Closure

**Goal:** close the remaining M4-1 contract gap so A3/A4/A5 no longer guess result-limit/cancel semantics.

**Read first:** `logs/checkpoints/M4-1.a-20260905-2245.md`, `logs/checkpoints/M4-1.b-20260905-2250.md`, `logs/checkpoints/M4-1.d-20260905-2300.md`, `src-tauri/src/domain.rs`.

**Implement/write:**

- Replace the STOPPED `logs/checkpoints/M4-1.c-20260905-2255.md` with a complete M4-1.c checkpoint.
- Add domain constants/types only if they are needed by implementation lanes: result row limit, result byte limit, field byte limit, SQL byte limit, default timeout, max timeout, truncation marker shape, cancellation state names.
- Do not implement DB drivers, commands, UI, or scripts.

**Must verify:** `cargo test --manifest-path src-tauri/Cargo.toml domain`, `git diff --check`.

**Finish only when:** M4-1.a/b/c/d can be read together without contradictions; explicitly state whether A3/A4 are unblocked.

### Lane A3: Database Pool + Driver Foundation Package

**Goal:** implement the M4-2 database runtime foundation far enough that A4 can expose commands without inventing internals.

**Read first:** all M4-1 checkpoints, A10 security notes, `src-tauri/src/workspace.rs`, `src-tauri/src/keyring_store.rs`, `src-tauri/src/security_policy.rs`, `src-tauri/src/script_runner.rs` for cancellation style.

**Implement/write:**

- Add the selected sync dependencies from M4-1.a: `rusqlite` with bundled SQLite, `mysql`, and `postgres`; do not add `tokio`, `sqlx`, `diesel`, JDBC, YAML, or codegen.
- Add `src-tauri/src/database.rs` or `src-tauri/src/database/` with a small, testable boundary:
  - connection config validation,
  - credential key helper exactly `db:<conn_id>`,
  - `DbPool` enum or equivalent per-driver holder,
  - connect/disconnect lifecycle primitives,
  - SQLite path root validation through existing path policy,
  - query result DTO construction with row/byte/field limits from M4-1.c,
  - cancellation and timeout hooks that do not forge unsupported fields.
- Do not expose Tauri commands directly unless A4's command layer is already present in the same rebased base and can be cleanly consumed.
- Add Rust tests for config validation, credential key namespace, SQLite happy path, path rejection, limit/truncation, timeout/cancel boundary where practical.
- Update `scripts/check-database-policy.py` pending/default codes only if A4 has not already done it and the edit is needed to make your package self-verifying; otherwise leave a checkpoint note for A4.

**Must verify:** `cargo test --manifest-path src-tauri/Cargo.toml database`, `cargo test --manifest-path src-tauri/Cargo.toml domain`, `cargo check --manifest-path src-tauri/Cargo.toml --locked`, `git diff --check`.

**Finish only when:** A4 can call a stable internal API without changing your module design.

### Lane A4: Database Safety Gate + Command Layer Package

**Goal:** implement the full backend IPC surface after A3 provides or while you provide a compatible internal API, without weakening source check or privacy.

**Read first:** all M4-1 checkpoints, A3 output if present, A10 notes, `src-tauri/src/bridge.rs`, `src-tauri/src/main.rs`, `src-tauri/permissions/default-commands.toml`, `src/bridge.ts`, `src/types.ts`.

**Implement/write:**

- First implement `M4-2.s` in `src-tauri/src/security_policy.rs`: SQL risk classifier, production verdict, fail-closed write gate, tests.
- Add or complete `scripts/check-database-policy.py` with self-test, default scan, pending handling, and pre-merge integration.
- Add backend commands only after an internal database API exists:
  - `db_connect`
  - `db_query`
  - `db_disconnect`
  - `db_forget_connection` only if A0/D28 is explicitly resolved in docs; otherwise leave it out and document D28.
- All commands must pass `check_invocation_source`, validate `tab-`/source expectations where applicable, redact audit details, never log SQL bodies with credentials, never serialize password/DSN/body.
- Register commands in `main.rs` and ACL before `list_artifact_images`.
- Mirror stable DTOs in `src/types.ts` and wrappers in `src/bridge.ts`.

**Must verify:** `cargo test --manifest-path src-tauri/Cargo.toml security_policy`, `cargo test --manifest-path src-tauri/Cargo.toml database`, `python3 scripts/check-database-policy.py --self-test`, `python3 scripts/check-database-policy.py`, `bash scripts/pre-merge.sh`, `git diff --check`.

**Finish only when:** backend command names/DTOs are stable enough for A5 to wire UI live.

### Lane A5: Database UI Full Package

**Goal:** deliver M4-4 UI as a usable database panel once A4 DTOs exist; before that, build pure logic and component shell but keep live calls guarded.

**Read first:** A4 command DTOs if present, `src/types.ts`, `src/bridge.ts`, existing browser/workspace panels, M2-5/M2-6 UI patterns.

**Implement/write:**

- Add a database panel in the existing workspace/browser UI location without marketing layout or broad restyle.
- Include connection list/form, driver-specific fields, password entered only as component-local transient state, write toggle, production warning, SQL editor, result grid, truncation indicators, copy/export controls, disconnect/clear affordances.
- Wire live `db_*` bridge calls only when A4 has landed wrappers; otherwise expose disabled states with clear internal TODO in checkpoint, not visible feature-explainer text.
- Add UI logic helpers for formatting rows, bytes, durations, truncation, risk labels, and connection form validation.
- Add `scripts/check-database-ui-logic.mjs`; add policy checks if in scope and low conflict.

**Must verify:** `node scripts/check-database-ui-logic.mjs`, `npm run build`, `git diff --check`.

**Finish only when:** a user can exercise the UI with real A4 commands or the checkpoint precisely lists the single missing backend unblock.

### Lane A6: Scheduler Policy Fixture Package

**Goal:** complete M4-5.d so A7 has machine-checkable scheduler invariants.

**Read first:** `logs/checkpoints/A6-M4-5-scheduler-contract-20260905-2330.md`, A10 scheduler security notes, `scripts/pre-merge.sh`.

**Implement/write:**

- Add `scripts/check-scheduler-policy.py`.
- Include self-test good/bad fixtures, pending codes, mutation-proof checks, and default scan.
- Enforce at least: no second execution path, clock injectable, missed policy field exists, tasks persistence atomic, no secret params persisted, scheduler shutdown order expected, no `tokio_cron_scheduler`, no direct `tokio` promotion unless a later A0 decision says otherwise.
- Connect it to `scripts/pre-merge.sh`.
- Add `logs/checkpoints/M4-5.d-<timestamp>.md`.

**Must verify:** `python3 scripts/check-scheduler-policy.py --self-test`, `python3 scripts/check-scheduler-policy.py`, `python3 scripts/check-scheduler-policy.py --expect-pending` if implemented, `bash scripts/pre-merge.sh`, `git diff --check`.

**Finish only when:** A7 can turn pending codes into default codes during implementation.

### Lane A7: Scheduler Backend Full Package

**Goal:** implement M4-6/M4-7 backend in one coherent package using A6's contract and existing script runner.

**Read first:** A6 contract, A6 policy fixture if present, A10 notes, `src-tauri/src/script_runner.rs`, `src-tauri/src/scripts.rs`, `src-tauri/src/snippets.rs`, `src-tauri/src/workspace.rs`, `src-tauri/src/shutdown.rs`, `bridge.rs`, `main.rs`, ACL.

**Implement/write:**

- Add scheduler/domain types if A6 did not already place concrete types in `domain.rs`: `TaskDef`, `TaskTrigger`, `TaskKind`, `MissedRunPolicy`, `RetryPolicy`, `TaskRunRecord`, status/trigger enums.
- Add `src-tauri/src/scheduler.rs` or module folder with:
  - `tasks.json` atomic persistence,
  - validation for target existence and non-secret params,
  - cron 5-field and interval validation,
  - next-run calculation with injectable clock,
  - missed-run policy handling,
  - same-task non-overlap,
  - history cap,
  - corrupt-file recovery behavior documented and tested.
- Add commands `task_list`, `task_add`, `task_update`, `task_remove`, `task_run_now` with source check, ACL, audit, DTOs, and `main.rs` registration.
- `task_run_now` and scheduler firing must reuse `script_runner::start_run` / `start_command`; no `std::process::Command`, no shell plugin, no second process path.
- Register `stop-scheduler` in `ShutdownCoordinator` before `kill-running-scripts` and add a test for the order.
- Update `scripts/check-scheduler-policy.py` pending/default codes as implementation lands.

**Must verify:** `cargo test --manifest-path src-tauri/Cargo.toml scheduler`, `cargo test --manifest-path src-tauri/Cargo.toml task`, `cargo test --manifest-path src-tauri/Cargo.toml`, `python3 scripts/check-scheduler-policy.py --self-test`, `python3 scripts/check-scheduler-policy.py`, `bash scripts/pre-merge.sh`, `git diff --check`.

**Finish only when:** A8 has stable task command wrappers/DTOs to wire UI live.

### Lane A8: Scheduler UI Full Package

**Goal:** deliver M4-8 UI as a usable task scheduler panel once A7 command DTOs exist; before that, build tested pure logic and component shell.

**Read first:** A6 contract, A7 command DTOs if present, `src/types.ts`, `src/bridge.ts`, existing script/command execution UI.

**Implement/write:**

- Add scheduler UI in existing workspace style:
  - task list,
  - add/edit form,
  - script/command target picker,
  - params editor,
  - cron 5-field or interval trigger controls,
  - missed-run policy,
  - retry controls,
  - enable switch defaulting to disabled if backend says so,
  - next run and last run display,
  - run-now action,
  - remove confirmation,
  - history/status panel.
- Wire live `task_*` calls only after A7 wrappers exist; otherwise keep disabled shell and document the backend unblock.
- Add `src/utils/taskUi.ts` or equivalent pure logic and `scripts/check-scheduler-ui-logic.mjs`.
- Add scheduler UI policy script only if it does not conflict with A6/A7, otherwise leave a precise checkpoint.

**Must verify:** `node scripts/check-scheduler-ui-logic.mjs`, `npm run build`, `git diff --check`.

**Finish only when:** the panel is either live against A7 commands or has exactly one documented backend unblock.

### Lane A9: M5 Future Split, No Product Code

**Goal:** prepare the next acceleration wave after M4 without polluting M4 code.

**Write:** one or more `logs/assist/A9-M5-*.md` docs covering database-backed knowledge graph, scheduled jobs feeding graph updates, protocol/agent/plugin split, dependencies on M4 commands, and suggested lanes A13+.

**Do not write:** `src/`, `src-tauri/`, `scripts/pre-merge.sh`, product code.

### Lane A10: Batch Security Review

**Goal:** review completed lane packages in batches, not per tiny file.

**Review batches:**

- Batch DB-1: A2+A3+A4 together.
- Batch DB-UI: A5 after live wiring.
- Batch SCHED-1: A6+A7 together.
- Batch SCHED-UI: A8 after live wiring.

**Write:** `logs/assist/A10-M4-security-review-batch-<name>-<timestamp>.md`.

**May edit policy scripts only if A0 explicitly assigns a fix.** Otherwise no product-code edits.

### Lane A11: Batch Verification Evidence

**Goal:** stop updating verification files for every tiny movement. Update after a complete batch only.

**Update after:**

- DB backend batch A2/A3/A4 completes.
- DB UI batch A5 completes.
- Scheduler backend batch A6/A7 completes.
- Scheduler UI batch A8 completes.

**Write:** verification matrix, manual checklist, debt ledger. Do not edit product code.

### Lane A12: Batch Conflict Scan

**Goal:** run conflict scans before A0 merge, not continuously.

**Check:** dirty files, empty files, stale NEXT, duplicate command names, ACL order, bridge/main/type mismatch, policy scripts in pre-merge, docs claiming PASS without evidence.

**Write:** `logs/assist/A12-M4-conflict-scan-<timestamp>.md`.

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
