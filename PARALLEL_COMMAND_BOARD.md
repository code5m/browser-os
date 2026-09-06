# Parallel Command Board

> Updated: 2026-09-06 18:35 CST
> Controller: main integration agent
> Canonical directory: `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`
> Current mainline: `master` at `1610939` locally and on `origin/master`
> Current NEXT: M5-W5 parallel implementation; Lane A6 owns M5-6 Agent/Skill UI pure logic/panel shell, Lane A7 owns M5-7/8 graph model/store policy slice, other lanes docs/review/support only

This file is the coordination board for 11 parallel agents plus A0 integration. Do not rely on chat history as the source of truth. Read `WORKSPACE_IDENTITY.md`, then read this file before making changes.

## One-Line Resume Prompt

Use this when assigning a Trae/WorkBuddy agent:

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane AX，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W5 Parallel Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
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
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
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






## M5-W5 Parallel Dispatch

> Added 2026-09-06 18:35 CST by A0 after pushing through `1610939`.
> Current facts: core seam, MCP policy shell, agent memory KV policy shell, and Agent/Skill domain policy shell are integrated.
> W5 opens A6 and A7 product-code lanes only. A6 may build frontend pure logic/panel shell for existing Agent/Skill DTOs; A7 may build graph model/store policy slice. No live graph UI or agent execution runtime yet.

### W5 One-Line Prompt

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane AX，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W5 Parallel Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

### W5 Assignments

| Lane | Status | Task | Allowed Scope | Must Deliver |
|---|---|---|---|---|
| A1 | **START DOCS ONLY** | Reconcile W5 as active NEXT; mark W4 pushed and tighten M5-6/M5-7/M5-8 acceptance criteria. | `PARALLEL_COMMAND_BOARD.md`, three main docs, `logs/checkpoints/M5-20260906/*.md` | One reconciliation checkpoint; no product code. |
| A2 | **SUPPORT/REVIEW ONLY** | Review whether A7 graph store should use existing core seam or stay bin-side; no product code. | `logs/assist/A2-M5-W5-*.md` | Boundary note only. |
| A3 | **SUPPORT/REVIEW ONLY** | Review A7 graph model against MCP registry exposure expectations; no MCP runtime expansion. | `logs/assist/A3-M5-W5-*.md` | MCP compatibility note. |
| A4 | **SUPPORT/REVIEW ONLY** | Review A6/A7 for memory privacy/capacity interactions; no product code. | `logs/assist/A4-M5-W5-*.md` | Review note. |
| A5 | **SUPPORT/REVIEW ONLY** | Review A6 UI against AgentDef/SkillDef contracts; no product code unless fixing docs only. | `logs/assist/A5-M5-W5-*.md` | Contract review note. |
| A6 | **START PRODUCT CODE** | Implement M5-6 Agent/Skill UI pure logic and panel shell: validation display, permission preview, capability list, empty/error states. Prefer helper module + headless logic test. Do not execute skills, install plugins, or call live runtime. | `src/components/**`, `src/stores/**`, `src/types.ts`, `src/bridge.ts` only if no new command, `scripts/check-agent-skill-ui-logic.mjs`, optional UI policy script, docs/checkpoint | `npm run build` PASS, UI logic test PASS, no new dependency, no live execution. |
| A7 | **START PRODUCT CODE** | Implement M5-7/M5-8 graph model/store policy slice: `GraphNode`/`GraphEdge` DTOs, capacity/redaction rules, pure graph store/query helpers, policy script. No graph UI and no agent consumption yet. | `src-tauri/src/domain.rs`, optional `src-tauri/src/graph.rs`, `src-tauri/src/security_policy.rs`, `scripts/check-graph-policy.py`, `scripts/pre-merge.sh`, focused Rust tests/checkpoint | Policy self-test/default PASS, Rust tests PASS, bounded nodes/edges, no command/ACL unless fully atomic. |
| A8 | **SUPPORT DOCS ONLY** | Prepare graph UI after A7 schema lands; no UI code in W5. | `logs/assist/A8-M5-W5-*.md`, `logs/checkpoints/M5-20260906/M5-9*.md` | UI card delta only. |
| A9 | **SUPPORT DOCS ONLY** | Align plugin manifest/lifecycle to A6 UI and A7 graph capabilities; no plugin product code. | `logs/assist/A9-M5-W5-*.md` | Delta note. |
| A10 | **START REVIEW** | Security review A6/A7 for secret display, unbounded graph growth, command exposure, source-check/ACL drift, and prompt/body persistence. | `logs/assist/A10-M5-W5-*.md`; policy fixtures only for concrete failure | Review after A6/A7 output. |
| A11 | **START VERIFICATION** | Update W5 verification matrix and GUI/manual debt ledger after A6/A7 outputs. | `logs/assist/M5-A11-W5-*.md`, `logs/checkpoints/M5-A11-W5-*.md` | One verification delta. |

### W5 Hard Stops

- Only A6 and A7 may write product code in W5.
- A6 must not add execution runtime, installer, network/model calls, or new backend commands.
- A7 must not add live UI, agent consumption, background graph rebuild workers, or network access.
- Any new command requires source check, ACL, frontend bridge/types, policy coverage, and tests in the same package; prefer no command in W5.
- All stores/maps/lists must be bounded; no token/cookie/Authorization/body/prompt-secret logging or persistence.
- All lanes pull from `origin/master` first and must not push.

## M5-W4 Parallel Dispatch

> Added 2026-09-06 17:55 CST by A0 after pushing through `f8f1f49`.
> Current facts: M5 core boundary/seam is integrated; M5-2 MCP registry/policy shell is integrated without rmcp/server runtime.
> W4 opens A4 and A5 product-code lanes only. A2/A3 become support/review for their landed contracts; A6-A11 remain docs/review/support unless explicitly listed below.

### W4 One-Line Prompt

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane AX，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W4 Parallel Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

### W4 Assignments

| Lane | Status | Task | Allowed Scope | Must Deliver |
|---|---|---|---|---|
| A1 | **START DOCS ONLY** | Reconcile W4 as active NEXT; mark W3 pushed and split M5-3/M5-4/M5-5 into next-card acceptance criteria. | `PARALLEL_COMMAND_BOARD.md`, three main docs, `logs/checkpoints/M5-20260906/*.md` | One reconciliation checkpoint; no product code. |
| A2 | **SUPPORT/REVIEW ONLY** | Review A4/A5 use of `mvp_core::seam`; propose next core extraction only if needed, no product code. | `logs/assist/A2-M5-W4-*.md` | Boundary review note; no product code. |
| A3 | **SUPPORT/REVIEW ONLY** | Review A4/A5 against MCP registry/policy decisions; no MCP server/runtime expansion. | `logs/assist/A3-M5-W4-*.md` | MCP compatibility note; no product code. |
| A4 | **START PRODUCT CODE** | Implement first M5-3 A2A/agent memory KV contract slice: DTOs, validation, capacity/privacy policy, pure store helpers or JSON persistence shell if already patterned; no network protocol and no background runtime. | `src-tauri/src/domain.rs`, optional `src-tauri/src/agent_memory.rs` or `src-tauri/src/a2a.rs`, `src-tauri/src/security_policy.rs`, `scripts/check-agent-memory-policy.py`, `scripts/pre-merge.sh`, focused Rust tests/checkpoint | Policy self-test/default PASS, no credentials/body leakage, bounded records, no new dependency unless already present and justified. |
| A5 | **START PRODUCT CODE** | Implement first M5-4/M5-5 Agent/Skill domain + command policy shell: `AgentDef`/`SkillDef` DTOs, validation, permission preview, policy script. Do not execute skills yet; do not add command runtime unless source check/ACL/types/bridge/tests are complete in same patch. | `src-tauri/src/domain.rs`, optional `src-tauri/src/agent.rs` or `src-tauri/src/skills.rs`, `src-tauri/src/security_policy.rs`, `scripts/check-agent-skill-policy.py`, `scripts/pre-merge.sh`, focused Rust tests/checkpoint | Policy self-test/default PASS, no second execution path, no installer/network/download. |
| A6 | **SUPPORT DOCS ONLY** | Convert A5 domain into future UI data contract and panel state plan; no UI code. | `logs/assist/A6-M5-W4-*.md`, `logs/checkpoints/M5-20260906/M5-6*.md` | UI delta note; no frontend product code. |
| A7 | **SUPPORT DOCS ONLY** | Prepare graph core slice after A4/A5 data contracts; no graph runtime. | `logs/assist/A7-M5-W4-*.md`, `logs/checkpoints/M5-20260906/M5-7*.md`, `M5-8*.md` | Next-card delta; no product code. |
| A8 | **SUPPORT DOCS ONLY** | Prepare graph UI after A7 freezes schema; no UI code. | `logs/assist/A8-M5-W4-*.md` | UI delta note. |
| A9 | **SUPPORT DOCS ONLY** | Align plugin manifest/lifecycle plans to A5 Agent/Skill policy; no plugin product code. | `logs/assist/A9-M5-W4-*.md`, `logs/checkpoints/M5-20260906/M5-10*.md` through `M5-12*.md` | Plugin delta note. |
| A10 | **START REVIEW** | Security review A4/A5 for credential leakage, unbounded maps, command exposure, source-check/ACL drift, duplicate execution path. | `logs/assist/A10-M5-W4-*.md`; policy fixtures only if fixing concrete failure | Review after A4/A5 output. |
| A11 | **START VERIFICATION** | Update W4 verification matrix and manual debt ledger after A4/A5 outputs. | `logs/assist/M5-A11-W4-*.md`, `logs/checkpoints/M5-A11-W4-*.md` | One verification delta. |

### W4 Hard Stops

- Only A4 and A5 may write product code in W4.
- No network protocol listener, rmcp server, plugin installer, model provider integration, background agent runtime, npm dependency, or GUI panel in W4.
- Any new command must be atomic with source check, ACL, frontend bridge/types, policy coverage, and tests. Prefer no command in W4 unless the contract is fully ready.
- Stores must be bounded and privacy-filtered; no token/cookie/Authorization/body/logged prompt secrets.
- All lanes pull from `origin/master` first and must not push.

## M5-W3 Parallel Dispatch

> Added 2026-09-06 17:10 CST by A0 after pushing through `712a14c`.
> Current facts: M5-W1 core boundary gate is integrated; M5-W2 constants are centralized in `domain.rs`; A3's M5-2 policy gate is already integrated.
> W3 opens two isolated product-code lanes only: A2 for core extraction, A3 for MCP command-registry/policy shell. Everything else stays docs/review/support.

### W3 One-Line Prompt

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane AX，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W3 Parallel Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

### W3 Assignments

| Lane | Status | Task | Allowed Scope | Must Deliver |
|---|---|---|---|---|
| A1 | **START DOCS ONLY** | Reconcile cards to current mainline `712a14c`; mark W1/W2 complete and make M5-W3 the active checkpoint. | `PARALLEL_COMMAND_BOARD.md`, `AI-模型切换与接手清单.md`, `详细设计与实施计划.md`, `后续需求TODO.md`, `logs/checkpoints/M5-20260906/*.md` | One reconciliation patch/checkpoint; no product code. |
| A2 | **START PRODUCT CODE** | Implement `M5-1.b` smallest B-class core extraction: trait/seam injection only, no behavior change. Prefer extracting pure path/limit helpers or contract adapters already identified in A2 notes. | `src-tauri/src/core/**`, `src-tauri/src/lib.rs`, narrowly related `src-tauri/src/domain.rs` / `workspace.rs` / `database.rs` / `script_runner.rs`, `scripts/check-core-boundary.py`, focused tests/checkpoint | `check-core-boundary.py` PASS, targeted `cargo test` PASS, no new dependency, no command/ACL/UI. |
| A3 | **START PRODUCT CODE** | Implement the first M5-2 MCP command-registry/policy slice without adding `rmcp`: frozen DTOs/pure registry/policy checks only; no server runtime and no network listener. | `src-tauri/src/domain.rs`, `src-tauri/src/security_policy.rs`, optional `src-tauri/src/mcp.rs`, `scripts/check-mcp-policy.py`, `scripts/pre-merge.sh`, focused Rust tests/checkpoint | Policy self-test + default PASS, pre-merge hook, no new Tauri commands unless ACL/source check are included and A0 can merge atomically. |
| A4 | **SUPPORT DOCS ONLY** | Prepare M5-3 A2A/agent memory contract against A2/A3 actual outputs; no product code. | `logs/assist/A4-M5-a2a-memory-*.md`, optional checkpoint patch | Delta note with exact dependency on MCP registry and core seam. |
| A5 | **SUPPORT DOCS ONLY** | Prepare M5-4/M5-5 Agent/Skill runtime next implementation card; keep execution reuse of M2-4 explicit. | `logs/assist/A5-M5-agent-skill-*.md`, `logs/checkpoints/M5-20260906/M5-4*.md`, `M5-5*.md` | Next-card delta; no runtime code. |
| A6 | **SUPPORT DOCS ONLY** | Prepare Agent/Skill UI data contract and no-router panel placement; no UI code. | `logs/assist/A6-M5-agent-ui-*.md` | UI readiness delta; no frontend product code. |
| A7 | **SUPPORT DOCS ONLY** | Prepare graph model/storage implementation card after A2/A3; do not write graph runtime yet. | `logs/assist/A7-M5-graph-core-*.md`, `logs/checkpoints/M5-20260906/M5-7*.md`, `M5-8*.md` | Exact DTO/schema and blockers; no graph product code. |
| A8 | **SUPPORT DOCS ONLY** | Prepare graph UI card after A7 schema freeze; no UI code. | `logs/assist/A8-M5-graph-ui-*.md` | UI split and logic-test plan; no frontend product code. |
| A9 | **SUPPORT DOCS ONLY** | Align plugin seam to A3 MCP registry and A5 Agent/Skill boundary; no plugin runtime code. | `logs/assist/A9-M5-plugin-*.md`, `logs/checkpoints/M5-20260906/M5-10*.md` through `M5-12*.md` | Delta note; keep Ed25519/form③ assumptions explicit. |
| A10 | **START REVIEW** | Review A2/A3 W3 outputs for boundary leaks, duplicate execution path, ACL/source-check drift, sensitive logging, and policy false negatives. | `logs/assist/A10-M5-W3-*.md`; policy script tests only if assigned in response to a concrete failure | Review after A2/A3 output; no product code by default. |
| A11 | **START VERIFICATION** | Update verification matrix for W3; collect A2/A3 commands and results. | `logs/assist/M5-A11-*.md`, `logs/checkpoints/M5-A11-*.md` | One verification delta after W3 outputs. |

### W3 Hard Stops

- Only A2 and A3 may write product code in W3.
- A2 must not touch MCP/Agent/Graph/Plugin command/UI/runtime.
- A3 must not add a network server, background listener, `rmcp`, npm dependency, plugin runtime, or Agent runtime.
- Any new Tauri command must include source check, ACL entry, frontend type/bridge sync, and policy coverage in the same lane package.
- A4-A11 must stop if product code is required; produce docs/checkpoint only.
- All lanes must pull from `origin/master` first; no push.

## M5-W1 Implementation Dispatch

> Added 2026-09-06 08:35 CST by A0 after reviewing M5-W0 outputs.
> W0 completion facts: A1 expanded M5 cards under `logs/checkpoints/M5-20260906/`; A2-A9 prework exists; A10 security review exists; A11 verification/debt docs exist.
> A0 decision: start only the M5-1 core-boundary slice. MCP/Agent/Graph/Plugin product code remains locked until M5-1 boundary checks land.

### W1 One-Line Prompt

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane AX，先 cd 到 WORKDIR，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W1 Implementation Dispatch 完成自己的整包交付，自行 rebase/整理补丁，不 push。
```

### W1 Assignments

| Lane | Status | Task | Allowed Scope | Must Deliver |
|---|---|---|---|---|
| A1 | **START DOCS ONLY** | Reconcile M5 task cards with A2/A6/A10 findings: M5-1 internal order must be boundary policy first, then minimal extraction; remove stale assumptions such as vue-router and empty prework notes. | `logs/checkpoints/M5-20260906/*.md`, three main docs only if strictly needed | One reconciliation checkpoint; no product code. |
| A2 | **START PRODUCT CODE** | Implement M5-1.a boundary gate first: add `scripts/check-core-boundary.py` with self-test/default/pending modes, wire it into `scripts/pre-merge.sh`, then create the smallest `mvp_core` module/lib boundary that compiles without moving behavior. Do not extract scheduler/database yet. | `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs` or `src-tauri/src/core/**`, `scripts/check-core-boundary.py`, `scripts/pre-merge.sh`, focused docs/checkpoint | Boundary policy PASS, cargo test PASS for touched modules, no behavior change, no new Cargo/npm dependency. |
| A3 | **SUPPORT DOCS ONLY** | Prepare M5-2 after M5-1: update MCP plan to consume A2 boundary shape; no rmcp dependency yet. | `logs/assist/A3-M5-mcp-*.md` | Delta note only if A2 boundary changes assumptions. |
| A4 | **SUPPORT DOCS ONLY** | Prepare M5-3 A2A memory against the shared capability boundary; no product code. | `logs/assist/A4-M5-a2a-memory-*.md` | Delta note with exact dependency on capability/core boundary. |
| A5 | **SUPPORT DOCS ONLY** | Prepare M5-4/5 Agent/Skill runtime commands after core boundary; no product code. | `logs/assist/A5-M5-agent-skill-*.md` | Delta note; keep Skill execution reuse of M2-4 explicit. |
| A6 | **SUPPORT DOCS ONLY** | Update Agent/Skill UI prework to remove vue-router assumption and align to `useLayoutStore` unless A1/A0 decides otherwise. | `logs/assist/A6-M5-agent-ui-*.md` | Delta note; no UI code. |
| A7 | **SUPPORT DOCS ONLY** | Prepare graph model/storage to consume A2 boundary; identify which graph pure functions belong in core. | `logs/assist/A7-M5-graph-core-*.md` | Delta note; no graph product code. |
| A8 | **SUPPORT DOCS ONLY** | Prepare graph UI after graph DTO freeze; no UI code. | `logs/assist/A8-M5-graph-ui-*.md` | Delta note only. |
| A9 | **SUPPORT DOCS ONLY** | Prepare plugin system after capability boundary; preserve form③ and Ed25519 proposal unless A10 finds blocker. | `logs/assist/A9-M5-plugin-*.md` | Delta note; no plugin product code. |
| A10 | **START REVIEW** | Review A2's W1 boundary patch for leaks: core importing tauri, duplicate execution path, hidden dependencies, command/ACL drift, policy false positives. | `logs/assist/A10-M5-W1-security-review-*.md` | Review after A2 output; no product code. |
| A11 | **START VERIFICATION** | Update M5 verification matrix with A2's actual boundary gate, commands, and pre-merge result. | `logs/checkpoints/M5-A11-*.md`, `logs/assist/M5-A11-*.md` | One verification delta after A2/A10. |

### W1 Hard Stops

- Only A2 may touch product code in W1.
- No `rmcp`, `tokio`, npm package, MCP server, Agent runtime, graph runtime, plugin runtime, new Tauri command, or ACL entry in W1.
- `check-core-boundary.py` must fail if core imports `tauri`, references `AppHandle`, references `crate::bridge`, or creates a second execution path.
- If A2 cannot keep behavior unchanged, stop with a patch/checkpoint instead of broad extraction.

## M5 Dispatch Now

> Added 2026-09-06 07:55 CST by A0 after pushing `a1a2061`.
> M4 database + scheduler code is integrated and pushed; machine gates are green (`cargo test` 329/329, `npm run build`, `pre-merge` ALL_PASS).
> This wave is **M5 expansion/prework first**. Do not write M5 product code until A1 has produced the M5 task cards and A0 has signed the first implementation dispatch.

### One-Line Prompt For All Lanes

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane AX，先 cd 到 WORKDIR，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W1 Implementation Dispatch 完成自己的整包交付，自行 rebase/整理补丁，不 push。
```

### M5 Wave 0 Assignments

| Lane | Status | Task | Allowed Scope | Must Deliver |
|---|---|---|---|---|
| A1 | **START** | Expand M5 into implementation cards. Split M5-1~M5-12 into safe subcards, dependency graph, red lines, test matrix, and merge order. | `AI-模型切换与接手清单.md`, `详细设计与实施计划.md`, `后续需求TODO.md`, `logs/checkpoints/M5-*.md` | M5 task-card checkpoint; NEXT should become `M5-1.a` only if cards are complete. |
| A2 | **START DOCS ONLY** | M5-1 core workspace extraction prework: map current Rust modules, propose crate boundaries, dependency direction checks, and minimal migration order. | `logs/assist/A2-M5-core-*.md` only | No product code; produce risks, file map, and first safe extraction slice. |
| A3 | **START DOCS ONLY** | M5-2 MCP/rmcp prework: inspect Rust MCP feasibility, command registry shape, global policy surface, and no-npm rule. | `logs/assist/A3-M5-mcp-*.md` only | Contract proposal + blockers; no dependency changes. |
| A4 | **START DOCS ONLY** | M5-3 A2A/agent memory prework: define agent dialect abstraction, `agent_kv` boundaries, retention/capacity/privacy rules. | `logs/assist/A4-M5-a2a-memory-*.md` only | Contract proposal + tests to require later. |
| A5 | **START DOCS ONLY** | M5-4/M5-5 Agent/Skill runtime and command prework: model `AgentDef`/`SkillDef`, execution reuse via M2-4, permission gates. | `logs/assist/A5-M5-agent-skill-*.md` only | Runtime/command split and security red lines. |
| A6 | **START DOCS ONLY** | M5-6 Agent/Skill UI prework: panel map, state model, streaming display, permission preview, failure recovery. | `logs/assist/A6-M5-agent-ui-*.md` only | UI task split and logic-test plan; no UI code. |
| A7 | **START DOCS ONLY** | M5-7/M5-8 knowledge graph model/storage prework: node/edge schema, derived-index rules, rebuild/export/query contracts, SQLite reuse. | `logs/assist/A7-M5-graph-core-*.md` only | Graph contract proposal + storage/query tests. |
| A8 | **START DOCS ONLY** | M5-9 graph UI and Agent consumption prework: force graph, search, node details, capacity, accessibility, subgraph injection. | `logs/assist/A8-M5-graph-ui-*.md` only | UI split + data contract needs; no UI code. |
| A9 | **START DOCS ONLY** | M5-10/M5-11/M5-12 plugin system prework: manifest, lifecycle, install/enable/disable/uninstall, permission UI, cleanup. | `logs/assist/A9-M5-plugin-*.md` only | Plugin task split + security blockers. |
| A10 | **START REVIEW** | Review M5 Wave 0 proposals for bypasses: duplicate execution paths, command exposure without ACL, credential leakage, unsafe plugin/skill installs. | `logs/assist/A10-M5-security-review-*.md` only | Security review after A1-A9 notes exist; do not edit product code. |
| A11 | **START VERIFICATION PLAN** | Create M5 verification matrix: required commands, policy scripts, UI logic tests, manual/GUI checklist, debt ledger numbers starting after M4. | `logs/assist/`, `logs/checkpoints/M5-A11-*.md` only | Verification matrix; no product code. |

### M5 Wave 0 Hard Stops

- No lane except A1 may change the three main planning docs in this wave.
- No M5 product code, Cargo dependency, npm dependency, command registration, ACL entry, or frontend panel may be added in Wave 0.
- If a lane finds product code is required, it writes an assist note with the exact future file list and stops.
- All proposals must preserve: no second execution path, source check + ACL for every future command, Keyring for credentials, audit redaction, capacity limits, and `pre-merge.sh` coverage.

## Batch Implementation Dispatch

This section supersedes the short `Code Dispatch Now` table for the next parallel wave. Agents should execute the largest safe chunk in their lane, verify it, and leave one coherent deliverable.

## Integration Fix Wave

> Added 2026-09-06 after A0 spot-check of the large M4 batch.
> Current machine facts: `cargo test --manifest-path src-tauri/Cargo.toml` = **328 passed**; `npm run build` = **PASS**; db/scheduler policy self-tests and UI logic tests = **PASS**; `bash scripts/pre-merge.sh` = **FAIL**.
> The goal of this wave is to make the batch mergeable, not to add new features.

### Current Red Lights

| ID | Red Light | Owner | Required Fix |
|---|---|---|---|
| IF-1 | `cargo fmt main` fails on M4 Rust files | A3/A4/A7, then A0 | Run `cargo fmt --manifest-path src-tauri/Cargo.toml`; do not manually reformat unrelated files. |
| IF-2 | Build metrics growth: frontend main JS about 197.04 kB vs 161.36 kB baseline, over 15% gate | A5/A8 investigate, A0 decides | Identify whether growth is expected from M4 UI or accidental eager imports. Prefer lazy-loading database/scheduler panels or splitting heavy helpers. If still over budget, document exact justification for A0 threshold/baseline decision. |
| IF-3 | `check-tools-policy.py --self-test` fails; A11 says existing non-M4 gate defect | A10 inspect, A0 fixes/decides | Confirm whether it is pre-existing by comparing with `origin/master`; propose smallest fix or explicit temporary waiver. Do not hide the failure. |
| IF-4 | `Cargo.toml` added `rusqlite`/`mysql`/`postgres`; `cargo check` warning count rose from 2 to 5 in one run | A3/A4 | Remove new unused imports/dead public methods where possible, or justify `#[allow(dead_code)]` with named consumer. Target warning count returns to baseline 2 unless A0 accepts a documented exception. |
| IF-5 | A11 batch2 report is stale in places: it records 323 tests and old DB policy failures, while A0 spot-check now sees 328 tests and DB policy PASS | A11 | Produce one final verification batch after IF-1..IF-4 are resolved; do not keep updating per tiny change. |

### Fix Assignments

#### Lane A3: DB Runtime Fix Package

Focus only on DB runtime merge blockers.

- Rebase/refresh on current local `master` state when possible; if local dirty state blocks ff-only pull, keep your lane changes and produce one patch.
- Run `cargo fmt --manifest-path src-tauri/Cargo.toml` only if you own the Rust files being formatted, or coordinate by noting that A0 may run global fmt at the end.
- Remove or justify new warnings from `src-tauri/src/database.rs`.
- Re-check that user SQL never uses `execute_batch`, `simple_query`, or `batch_execute`; test fixtures may use setup helpers, but production user-SQL paths must be single statement.
- Verify:
  - `cargo test --manifest-path src-tauri/Cargo.toml database`
  - `python3 scripts/check-database-policy.py --self-test`
  - `python3 scripts/check-database-policy.py`
  - `python3 scripts/check-database-policy.py --expect-pending`
  - `git diff --check`

#### Lane A4: DB Safety/Command Fix Package

Focus only on command/safety merge blockers.

- Confirm `DB_MULTI_STATEMENT_FORBIDDEN` policy is active if implementation exists; `--expect-pending` should not report implemented pending items.
- Verify `db_*` commands, if present, are in all three places: Rust handler, frontend bridge/types, ACL before `list_artifact_images`.
- Ensure audit detail never contains raw SQL, password, DSN, token, cookie, Authorization, or Set-Cookie.
- Remove unused import warnings in `security_policy.rs` if present.
- Verify:
  - `cargo test --manifest-path src-tauri/Cargo.toml security_policy`
  - `cargo test --manifest-path src-tauri/Cargo.toml database`
  - `python3 scripts/check-database-policy.py --self-test`
  - `python3 scripts/check-database-policy.py`
  - `bash scripts/pre-merge.sh` if the worktree is otherwise ready

#### Lane A5: DB UI Size/Import Fix Package

Focus only on frontend build size and DB UI readiness.

- Inspect whether `DatabasePanel.vue`, `useDatabaseStore.ts`, or `dbUi.ts` are eagerly imported in a way that grows the main chunk unnecessarily.
- Prefer lazy-loading the database panel from the layout switch if this matches existing patterns and does not destabilize UI.
- Keep password values component-local; do not add persistence.
- Verify:
  - `node scripts/check-database-ui-logic.mjs`
  - `npm run build`
  - record resulting `dist/assets/index-*.js` size in checkpoint

#### Lane A6: Scheduler Policy Fix Package

Focus only on scheduler policy/pre-merge stability.

- Keep `scripts/check-scheduler-policy.py` active set consistent with implemented scheduler code.
- Ensure the script has no false positive on current real repo.
- Verify:
  - `python3 scripts/check-scheduler-policy.py --self-test`
  - `python3 scripts/check-scheduler-policy.py`
  - `bash scripts/pre-merge.sh` if the worktree is otherwise ready

#### Lane A7: Scheduler Backend Fix Package

Focus only on scheduler backend merge blockers.

- Remove or justify new Rust warnings introduced by scheduler/task code.
- Ensure scheduler has no direct process execution path and only calls `script_runner::start_run` / `start_command`.
- Ensure shutdown registration order remains `stop-scheduler` before `kill-running-scripts`.
- Verify:
  - `cargo test --manifest-path src-tauri/Cargo.toml scheduler`
  - `cargo test --manifest-path src-tauri/Cargo.toml task`
  - `python3 scripts/check-scheduler-policy.py`
  - `git diff --check`

#### Lane A8: Scheduler UI Size/Import Fix Package

Focus only on scheduler UI readiness and build size.

- Inspect whether `TaskPanel.vue`, `TaskEditDialog.vue`, `useTaskStore.ts`, or `taskUi.ts` are eagerly imported unnecessarily.
- Prefer lazy-loading scheduler UI if it reduces main chunk and fits existing layout patterns.
- Verify:
  - `node scripts/check-scheduler-ui-logic.mjs`
  - `python3 scripts/check-scheduler-ui-policy.py --self-test`
  - `python3 scripts/check-scheduler-ui-policy.py`
  - `npm run build`

#### Lane A10: Batch Re-Review After Fixes

Do not review every small edit. Wait until A3/A4/A5/A6/A7/A8 report their fix package, then produce:

- `logs/assist/A10-M4-security-review-batch-final-<timestamp>.md`

Must explicitly answer:

- Are DB write gates still fail-closed?
- Are credentials absent from DTOs, audit, frontend state, files, and logs?
- Are user SQL paths single-statement only?
- Does scheduler reuse M2-4 execution and shutdown correctly?
- Are any new warnings or policy exceptions acceptable?

#### Lane A11: Final Verification Batch Only

Wait for A10 final review or all fix packages, then produce:

- `logs/checkpoints/A11-M4-verification-final-<timestamp>.md`

Must rerun and record:

- `cargo test --manifest-path src-tauri/Cargo.toml`
- `npm run build`
- all new policy scripts self-test/default/pending modes
- `bash scripts/pre-merge.sh`
- `git diff --check`

#### Lane A11: Final Conflict Scan Before A0

As part of the final verification batch, A11 must also check:

- no empty files
- no stale `STOPPED` claiming as PASS
- no duplicate command names
- ACL order
- bridge/main/types command parity
- docs NEXT consistency
- no lane scope drift in checkpoints

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

## Lane-Specific Minimal Prompts

Use exactly one line per agent:

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A1，先 cd 到 WORKDIR，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 Integration Fix Wave 修复自己的阻塞项并按 Batch Implementation Dispatch 做完整代码包，自行 rebase/整理补丁，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A2，先 cd 到 WORKDIR，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 Integration Fix Wave 修复自己的阻塞项并按 Batch Implementation Dispatch 做完整代码包，自行 rebase/整理补丁，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A3，先 cd 到 WORKDIR，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 Integration Fix Wave 修复自己的阻塞项并按 Batch Implementation Dispatch 做完整代码包，自行 rebase/整理补丁，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A4，先 cd 到 WORKDIR，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 Integration Fix Wave 修复自己的阻塞项并按 Batch Implementation Dispatch 做完整代码包，自行 rebase/整理补丁，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A5，先 cd 到 WORKDIR，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 Integration Fix Wave 修复自己的阻塞项并按 Batch Implementation Dispatch 做完整代码包，自行 rebase/整理补丁，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A6，先 cd 到 WORKDIR，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 Integration Fix Wave 修复自己的阻塞项并按 Batch Implementation Dispatch 做完整代码包，自行 rebase/整理补丁，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A7，先 cd 到 WORKDIR，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 Integration Fix Wave 修复自己的阻塞项并按 Batch Implementation Dispatch 做完整代码包，自行 rebase/整理补丁，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A8，先 cd 到 WORKDIR，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 Integration Fix Wave 修复自己的阻塞项并按 Batch Implementation Dispatch 做完整代码包，自行 rebase/整理补丁，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A9，先 cd 到 WORKDIR，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 Integration Fix Wave 修复自己的阻塞项并按 Batch Implementation Dispatch 做完整代码包，自行 rebase/整理补丁，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A10，先 cd 到 WORKDIR，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 Integration Fix Wave 修复自己的阻塞项并按 Batch Implementation Dispatch 做完整代码包，自行 rebase/整理补丁，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A11，先 cd 到 WORKDIR，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 Integration Fix Wave 修复自己的阻塞项并按 Batch Implementation Dispatch 做完整代码包，自行 rebase/整理补丁，不 push。
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
