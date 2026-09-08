# Parallel Command Board

> Updated: 2026-09-08 11:44 CST
> Controller: A0; external lanes: A1-A11
> Canonical directory: `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`
> Evidence baseline: `master` at `d6127c4`; obtain the current HEAD with git before starting. This dispatch changes documentation only.
> Baseline chain: `61cff56` (BUG-HUNT + scheduler safety) -> `e3b6b40` (ACL rebuild) -> `8e67ab0` (debug-only IPC origin isolation) -> `12a09b8` (W18 lane isolation) -> `78d2cfb` (research-first reset) -> `d6127c4` (R2 evidence dispatch). Runtime authority remains LOCKED. A0 is the controller and the only lane that integrates or pushes; the external agents are A1-A11.
> Current NEXT: `M5-W18-R3-UX`. The first IDEA-shell prototype was rejected by the user. Preserve the current Chrome-like content-first shell and redesign it with IDEA-style progressive disclosure, complete context menus, and a full Git workflow inspired by open-source Git clients. `W19=CLOSED`.

## Current Dispatch Entry

Read [R3 UX task cards](M5-W18-R3-UX-TASKS-20260908.md), [rejected prototype review](M5-W18-PROTOTYPE-REVIEW-20260908.md), [A0 integration audit](logs/checkpoints/A0-M5-W18-R2B-integration-audit-20260908.md), and [workbench blueprint](WORKBENCH_BLUEPRINT-20260908.md).

This entry supersedes conflicting historical NEXT, directory, model, and lane instructions below. R2B domain research remains accepted. R3 reopens only interaction research and prototype work: no product source, dependencies, capabilities, ACLs, native runtime, or user data may change.

Runtime authority remains locked. A1 owns the consolidated prototype; A2-A9 provide bounded design evidence; A10 independently reviews; A11 packages acceptance. A0 alone integrates and pushes.

This file is the coordination board for 11 parallel agents plus A0 integration. Do not rely on chat history as the source of truth. Read `WORKSPACE_IDENTITY.md`, then read this file before making changes.

## One-Line Resume Prompt

Use this when assigning a Trae/WorkBuddy agent:

```text
LANE=A1；读取 /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3/WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按当前派发入口推导本lane目录/分支并进入，完成自己的整包任务；沿用已有成果，按卡自检并提交本lane，不改范围外文件、不push。
```

Replace only the first `A1` with the assigned lane id. Lowercase that one identity to derive both directory and branch using the current task card. Existing lane conversations may resume with "continue"; new conversations must declare the lane.

## Batch Implementation Rule

Historical coding-wave rules follow. For the active research wave, the R2B task card overrides pull/patch/scope instructions; do not run a pull on a lane branch with local research commits. Once a coding slice is explicitly opened, deliver a complete tested package:

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

First read canonical identity/dispatch without editing it. Then enter the existing worktree derived from the single assigned LANE in the R2B card. In that directory run:

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

If the only blocker is an unfinished predecessor, complete independent research in the lane-owned scope, then record the precise dependency there. Do not write shared assist files or product code outside the current card.

## Merge Rule

Only Lane A0 commits and pushes to `master`.

Other lanes must deliver one of these:

- A clean branch/worktree with a small commit series.
- A patch file plus a checkpoint.
- A review/checkpoint document with no product-code changes.

No lane may force-push, reset, or overwrite another lane's changes.

## Active Lanes

The authoritative assignments are the latest `M5-W18 Obsidian + dbx + zvec-grep Dispatch` at the end of this file. Earlier M4/M5 tables are retained only as history.

| Lane | Current role | Status | Merge order |
|---|---|---|---|
| A0 | Integration controller; only lane allowed to push | ACTIVE | Last |
| A1 | Current product inventory and requirements baseline | RESEARCH | 1 |
| A2 | Obsidian vault/data/link semantics | RESEARCH | 2 |
| A3 | Obsidian graph/backlink/search UX behavior | RESEARCH | 3 |
| A4 | dbx backend architecture and transplantable modules | RESEARCH | 4 |
| A5 | dbx desktop workbench and data-grid UX | RESEARCH | 5 |
| A6 | dbx security, credentials, cancellation, and lifecycle | RESEARCH | 6 |
| A7 | zvec-grep indexing, extraction, and freshness | RESEARCH | 7 |
| A8 | zvec-grep exact/BM25/vector/hybrid retrieval | RESEARCH | 8 |
| A9 | zvec-grep authorization, MCP/server, and privacy boundary | RESEARCH | 9 |
| A10 | Licensing, dependency BOM, and source-transplant plan | RESEARCH | 10 |
| A11 | Consolidated acceptance matrix and implementation-wave design | RESEARCH | 11 |

## Current Priority

1. A1-A11 research in parallel from pinned sources and the current product; no product implementation occurs in W18-R.
2. Each report must map exact source files/symbols, data flows, dependencies, tests, failure modes, license obligations, and what should be copied/adapted/rejected.
3. A10 consolidates the legal/dependency source-transplant map; A11 consolidates acceptance and proposed implementation slices.
4. A0 reviews all reports, resolves contradictions, freezes the target architecture, and only then opens W19 implementation.

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








## M5-W7 Integration Dispatch

> Added 2026-09-07 00:50 CST by A0 after pushing through `5f92ece`.
> Current facts: graph UI shell and plugin manifest/lifecycle policy are integrated. W7 is a narrow read-only command bridge wave, not runtime activation.
> W7 opens A3 and A5 product-code lanes only. A3 may expose read-only MCP registry/policy commands. A5 may expose read-only Agent/Skill parse/validate/permission-preview commands. No server, no execution, no plugin install.

### W7 One-Line Prompt

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane AX，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W7 Integration Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

### W7 Assignments

| Lane | Status | Task | Allowed Scope | Must Deliver |
|---|---|---|---|---|
| A1 | **START DOCS ONLY** | Reconcile W7 as active NEXT; mark W6 pushed and define M5 final acceptance/debt list. | `PARALLEL_COMMAND_BOARD.md`, three main docs, `logs/checkpoints/M5-20260906/*.md` | One reconciliation checkpoint; no product code. |
| A2 | **SUPPORT/REVIEW ONLY** | Review A3/A5 command bridges for core boundary leaks and duplicate execution paths. | `logs/assist/A2-M5-W7-*.md` | Boundary review note. |
| A3 | **START PRODUCT CODE** | Implement M5-2 read-only MCP registry/policy bridge commands: list registry entries, preview capability verdicts, return redacted DTOs. Must include source check, ACL, frontend bridge/types only if commands are added. No rmcp/server/listener. | `src-tauri/src/mcp.rs`, `src-tauri/src/bridge.rs`, `src-tauri/src/main.rs`, `src-tauri/permissions/default-commands.toml`, `src/bridge.ts`, `src/types.ts`, `scripts/check-mcp-policy.py`, focused tests/checkpoint | Commands + ACL/source check + policy/tests PASS, no runtime server, no network. |
| A4 | **SUPPORT/REVIEW ONLY** | Review A5 command payloads for memory/privacy interactions; no product code. | `logs/assist/A4-M5-W7-*.md` | Review note. |
| A5 | **START PRODUCT CODE** | Implement Agent/Skill read-only command bridge: parse/validate AgentDef/SkillDef and permission preview. No skill execution, no install, no network, no persistence writes. Must include source check, ACL, frontend bridge/types if commands are added. | `src-tauri/src/agent.rs`, `src-tauri/src/skills.rs`, `src-tauri/src/bridge.rs`, `src-tauri/src/main.rs`, `src-tauri/permissions/default-commands.toml`, `src/bridge.ts`, `src/types.ts`, `scripts/check-agent-skill-policy.py`, focused tests/checkpoint | Commands + ACL/source check + policy/tests PASS, no execution path. |
| A6 | **SUPPORT DOCS ONLY** | Prepare UI wiring expectations for A5 read-only commands; no UI code unless A0 later opens it. | `logs/assist/A6-M5-W7-*.md` | UI wiring note. |
| A7 | **SUPPORT DOCS ONLY** | Prepare graph command bridge plan for later W8; no graph command code in W7. | `logs/assist/A7-M5-W7-*.md` | Graph bridge card. |
| A8 | **SUPPORT/REVIEW ONLY** | Review A3/A5 frontend bridge/types for graph UI impact; no graph UI code. | `logs/assist/A8-M5-W7-*.md` | Review note. |
| A9 | **SUPPORT/REVIEW ONLY** | Review plugin command surface stays absent or read-only; no plugin install/runtime code. | `logs/assist/A9-M5-W7-*.md` | Plugin review note. |
| A10 | **START REVIEW** | Security review A3/A5 command bridges: source check, ACL, redaction, no execution/install/network, no sensitive audit. | `logs/assist/A10-M5-W7-*.md`; policy fixtures only for concrete failure | Review after A3/A5 output. |
| A11 | **START VERIFICATION** | Update W7 verification matrix and M5 final readiness list after A3/A5 outputs. | `logs/assist/M5-A11-W7-*.md`, `logs/checkpoints/M5-A11-W7-*.md` | One verification delta. |

### W7 Hard Stops

- Only A3 and A5 may write product code in W7.
- W7 commands are read-only only: no skill execution, no plugin install/enable/disable/uninstall, no MCP server/listener, no graph rebuild worker.
- Every new command must be atomic with source check, ACL, frontend bridge/types, policy coverage, and tests.
- No token/cookie/Authorization/body/prompt-secret logging or persistence.
- All lanes pull from `origin/master` first and must not push.





## M5-W13 Plugin Runtime Stage-I Manifest Lifecycle Dispatch

> Added: 2026-09-07 23:55 CST by A0 after W12 local validation. W12 graph live-query readonly bridge/UI is accepted for commit in this batch. W13 is deliberately narrow: **manifest lifecycle state only**, no plugin code execution.

### One-Line Resume Prompt

WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane AX，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W13 Plugin Runtime Stage-I Manifest Lifecycle Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。

### W13 Hard Stops

- If `pwd` is not `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`, stop.
- If branch is not `master`, stop.
- If local branch is behind `origin/master`, run `git pull --ff-only`; if that fails, stop and report.
- Do not push; only A0 pushes.
- Do not implement `plugin_invoke`, command execution, dynamic code loading, network download/listener, daemon, model call, Agent/Skill execution, MCP full runtime, graph build/write/export, or background workers.
- Do not store credentials, raw signatures beyond the frozen DTO, request/response bodies, plugin stdout/stderr, or unredacted paths in audit/checkpoints.

### Lane Table

| Lane | Status | Scope | Allowed Files | Must Deliver |
|---|---|---|---|---|
| A1 | START DOCS ONLY | Reconcile W12 as accepted after A0 push and mark W13 active. Keep W13 scope as plugin manifest lifecycle only. | `PARALLEL_COMMAND_BOARD.md`, three main docs, `logs/checkpoints/M5-20260906/*.md`, `logs/checkpoints/A1-M5-W13-*.md` | Docs checkpoint + patch; no product code. |
| A2 | START BOUNDARY REVIEW ONLY | Review plugin lifecycle boundary: pure domain/store helpers in plugin modules, commands in bridge, no graph/db/script/mcp/agent execution path, no threads/listeners. | `logs/assist/A2-M5-W13-*.md`; policy fixtures only if concrete | Boundary verdict with blockers. |
| A3 | START MCP REVIEW ONLY | Confirm plugin lifecycle commands are not exposed through MCP stdio; MCP remains dry-run/read-only introspection. | `logs/assist/A3-M5-W13-*.md`, `scripts/check-mcp-policy.py` only for concrete fixture | MCP isolation verdict. |
| A4 | START PRIVACY REVIEW ONLY | Review plugin DTO/errors/audit/storage for secret echo; audit must use ids/counts/hash prefixes only. | `logs/assist/A4-M5-W13-*.md`; policy fixtures only if concrete | Privacy verdict. |
| A5 | START AGENT/SKILL REVIEW ONLY | Confirm Agent/Skill execution remains locked and plugin lifecycle UI does not imply agent/tool execution. | `logs/assist/A5-M5-W13-*.md`, `scripts/check-agent-skill-policy.py` only if concrete | Agent/Skill lock verdict. |
| A6 | START UI REVIEW ONLY | Review any plugin manager UI changes for disabled invoke controls, clear lifecycle status, and no fake execution affordance. | `logs/checkpoints/A6-M5-W13-*.md`; UI logic fixtures only if concrete | UI verdict. |
| A7 | SUPPORT DOCS ONLY | Graph W12 is closed; do not write graph product code. Review W13 only for graph non-regression if asked. | `logs/assist/A7-M5-W13-*.md` | Non-regression note. |
| A8 | SUPPORT DOCS ONLY | Graph UI W12 is closed; do not write graph UI product code. Review W13 only for graph UI non-regression if asked. | `logs/assist/A8-M5-W13-*.md` | Non-regression note. |
| A9 | START PRODUCT CODE NARROW | Implement plugin manifest lifecycle Stage-I: local manifest registry/store, `plugin_install` metadata-only local package validation, `plugin_enable`, `plugin_disable`, `plugin_list`, `plugin_get`, trusted-key add/list/remove as data management if feasible; source check, ACL, audit redaction, policy/tests. No invoke/execution/network/download/dynamic load. | `src-tauri/src/plugin.rs`, `src-tauri/src/domain.rs`, `src-tauri/src/bridge.rs`, `src-tauri/src/main.rs`, `src-tauri/permissions/default-commands.toml`, `src/bridge.ts`, `src/types.ts`, plugin UI files, `scripts/check-plugin-policy.py`, focused tests/checkpoint | Rust plugin tests PASS; plugin policy self/default PASS; ACL/source check parity; audit redacted; checkpoint + patch. |
| A10 | START SECURITY REVIEW | Review W13 plugin lifecycle for execution bypass, path escape, secret echo, ACL/source-check gaps, build warnings/metric regression. | `logs/assist/A10-M5-W13-*.md`; policy fixtures only for concrete failure | Security verdict. |
| A11 | START VERIFICATION | Maintain W13 verification matrix: cargo test plugin, cargo test, mcp feature test, plugin/graph/mcp/agent-skill policies, UI logic, npm build, pre-merge, diff check, push readiness. | `logs/checkpoints/A11-M5-W13-*.md`, `logs/assist/A11-M5-W13-*.md` | Verification checkpoint and push readiness. |

### Direct Prompts

WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A1，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W13 Plugin Runtime Stage-I Manifest Lifecycle Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A2，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W13 Plugin Runtime Stage-I Manifest Lifecycle Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A3，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W13 Plugin Runtime Stage-I Manifest Lifecycle Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A4，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W13 Plugin Runtime Stage-I Manifest Lifecycle Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A5，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W13 Plugin Runtime Stage-I Manifest Lifecycle Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A6，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W13 Plugin Runtime Stage-I Manifest Lifecycle Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A7，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W13 Plugin Runtime Stage-I Manifest Lifecycle Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A8，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W13 Plugin Runtime Stage-I Manifest Lifecycle Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A9，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W13 Plugin Runtime Stage-I Manifest Lifecycle Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A10，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W13 Plugin Runtime Stage-I Manifest Lifecycle Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A11，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W13 Plugin Runtime Stage-I Manifest Lifecycle Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。

## M5-W12 Graph Live-Query Readonly Dispatch

> Added 2026-09-07 20:30 CST by A0 after W11 validation.
> Current facts: W11 MCP stdio dry-run hardening passes (`cargo test --features mcp mcp_server` 21/21, MCP policy ACTIVE=11/PENDING=0, no listener/network/raw argument echo, default build unchanged). A5 added Agent/Skill execution-lock policy (PENDING=6) with default PASS. A7 prepared the graph W12 implementation card; A9 prepared plugin W12/W13 cards.
> W12 goal: implement a **read-only graph live-query bridge** and frontend consumption. This is not a graph builder, not a background indexer, and not plugin/agent runtime activation.

### W12 One-Line Prompt

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane AX，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W12 Graph Live-Query Readonly Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

### W12 Assignments

| Lane | Status | Task | Allowed Scope | Must Deliver |
|---|---|---|---|---|
| A1 | **START DOCS ONLY** | Reconcile W11 as accepted after A0 push and mark W12 active. Update M5 graph cards so W12 scope is exactly three read-only graph commands plus UI consumption. | `PARALLEL_COMMAND_BOARD.md`, three main docs, `logs/checkpoints/M5-20260906/*.md`, `logs/checkpoints/A1-M5-W12-*.md` | One reconciliation checkpoint; no product code. |
| A2 | **START BOUNDARY REVIEW ONLY** | Review graph W12 bridge for core boundary: `graph.rs` pure/state only, commands in `bridge.rs`, no `crate::bridge` from graph, no DB/script/plugin/agent execution path, no background worker. | `logs/assist/A2-M5-W12-*.md`; policy fixtures only if concrete | Boundary verdict with blockers. |
| A3 | **START MCP REVIEW ONLY** | Review that W12 graph commands are not accidentally exposed through MCP stdio as executable tools. MCP remains dry-run/read-only introspection only; do not change MCP product code unless fixing a concrete policy regression. | `logs/assist/A3-M5-W12-*.md`, `scripts/check-mcp-policy.py` only for concrete fixture | MCP review note; no runtime expansion. |
| A4 | **START PRIVACY REVIEW ONLY** | Review graph query DTO/errors/audit for secret echo: output must omit `props`, errors must be stable codes, audit must not include labels/props/query bodies with secrets. | `logs/assist/A4-M5-W12-*.md`; policy fixtures only if concrete | Privacy verdict. |
| A5 | **START AGENT/SKILL REVIEW ONLY** | Confirm Agent/Skill execution remains locked and graph UI does not imply agent consumption runtime. Add policy only for concrete leakage. | `logs/assist/A5-M5-W12-*.md`, `scripts/check-agent-skill-policy.py` only if concrete | Agent/Skill lock verdict. |
| A6 | **START UI REVIEW ONLY** | Review workspace UI for Agent/Skill panels after W12 graph changes; keep execution controls disabled and deterministic. No backend changes. | `src/components/workspace/**`, `scripts/check-agent-skill-ui-logic.mjs`, checkpoint/assist if changed | UI logic PASS or no-change review. |
| A7 | **START PRODUCT CODE NARROW** | Implement graph live-query read-only bridge: `GraphState` managed state, startup load of existing graph store if available, `graph_query`, `graph_node_get`, `graph_stats` commands with `check_invocation_source`, bounded depth/limit, stable error codes, no props in output DTOs, ACL/bridge.ts/types parity, graph policy self-test update. No graph build/index/write/export/background worker. | `src-tauri/src/graph.rs`, `src-tauri/src/domain.rs`, `src-tauri/src/bridge.rs`, `src-tauri/src/main.rs`, `src-tauri/permissions/default-commands.toml`, `src/bridge.ts`, `src/types.ts`, `scripts/check-graph-policy.py`, focused tests/checkpoint | Rust graph tests PASS; graph policy self/default PASS; ACL/source check parity; no props/secret echo; checkpoint + patch. |
| A8 | **START PRODUCT CODE UI NARROW** | Consume W12 read-only graph commands in UI: enable backend-ready graph load/query/stats, AbortController/debounce where useful, bounded rendering preserved, deterministic empty/error/loading states. No backend changes beyond bridge/types use. | `src/components/graph/**`, `src/stores/useGraphStore.ts`, `src/utils/graphUi.ts`, `src/bridge.ts`, `src/types.ts`, `scripts/check-graph-ui-logic.mjs`, checkpoint/assist | Graph UI logic PASS; npm build PASS; no secret/props rendering. |
| A9 | **START PLUGIN DOCS ONLY** | Keep plugin runtime locked. Refine W12/W13 plugin runtime cards only if W11 feedback changed blockers; do not implement install/enable/delete. | `logs/assist/A9-M5-W12-*.md`, plugin M5 docs only | Plugin plan delta or no-change note. |
| A10 | **START SECURITY REVIEW** | Batch review W12 graph bridge/UI. Block on graph writes/builders/background workers, props/secret output, source-check/ACL gaps, DB/script/plugin/agent execution paths, build metric/warning regression. | `logs/assist/A10-M5-W12-*.md`; policy fixtures only for concrete failure | Security verdict after A7/A8 output. |
| A11 | **START VERIFICATION** | Maintain W12 verification matrix: cargo test, graph focused tests, graph policy self/default, MCP policy still PASS, Agent/Skill lock still PASS, graph UI logic, npm build, pre-merge, build metrics <=23%, warnings unchanged. | `logs/checkpoints/A11-M5-W12-*.md`, `logs/assist/A11-M5-W12-*.md` | Verification checkpoint and push readiness. |

### W12 Hard Stops

- W12 graph scope is read-only query only: no graph build/index/write/export/background worker.
- Graph output DTOs must omit raw `props`; errors must be stable codes and must not echo labels, props, query bodies, paths, URLs, tokens, cookies, or Authorization.
- New commands must pass `check_invocation_source`, ACL, `main.rs` handler, `bridge.ts`, `types.ts`, policy, and tests in the same package.
- MCP full runtime, plugin install/enable/delete/download, Agent/Skill execution, network listener, daemon, model call, and hidden script/db execution remain LOCKED.
- Build metrics threshold is 23% after W12; cargo warnings must not increase.
- Only A0 pushes to remote.

### W12 Direct Prompts

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A1，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W12 Graph Live-Query Readonly Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A2，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W12 Graph Live-Query Readonly Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A3，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W12 Graph Live-Query Readonly Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A4，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W12 Graph Live-Query Readonly Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A5，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W12 Graph Live-Query Readonly Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A6，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W12 Graph Live-Query Readonly Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A7，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W12 Graph Live-Query Readonly Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A8，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W12 Graph Live-Query Readonly Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A9，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W12 Graph Live-Query Readonly Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A10，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W12 Graph Live-Query Readonly Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A11，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W12 Graph Live-Query Readonly Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

## M5-W11 MCP Stdio Dry-Run Hardening Dispatch

> Added 2026-09-07 18:30 CST by A0 after W10 focused validation.
> Current facts: W10 A3 stdio-prep skeleton is feature-gated (`mcp`), std-only, no rmcp/tokio dependency, no TCP/network listener, and `cargo test --features mcp mcp_server` passes 7/7. W10 policy gates pass with `MCP_POLICY_SELF_TEST=PASS(ACTIVE=9,PENDING=0)` and `MCP_CURRENT_GAPS_RESULT=PASS`. A6/A8 UI logic checks and npm build pass in W10.
> W11 goal: harden the MCP stdio shell through deterministic dry-run behavior and review evidence only. This is still not full runtime activation: no file/db/script/plugin execution, no network listener, no daemon, no model call.

### W11 One-Line Prompt

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane AX，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W11 MCP Stdio Dry-Run Hardening Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

### W11 Assignments

| Lane | Status | Task | Allowed Scope | Must Deliver |
|---|---|---|---|---|
| A1 | **START DOCS ONLY** | Reconcile W10 as accepted after A0 push and mark W11 active. Update M5 cards so W10 stdio-prep is recorded as feature-gated PASS and W11 remains dry-run only. | `PARALLEL_COMMAND_BOARD.md`, three main docs, `logs/checkpoints/M5-20260906/*.md`, `logs/checkpoints/A1-M5-W11-*.md` | One reconciliation checkpoint; no product code. |
| A2 | **START BOUNDARY REVIEW ONLY** | Review W11 MCP dry-run changes for core/bin boundary: no tauri in core, no bridge::* calls from stdio tools, no duplicate script/db/plugin execution path, no workspace writes. | `logs/assist/A2-M5-W11-*.md`; policy-only fixtures if concrete | Boundary verdict with exact blockers. |
| A3 | **START PRODUCT CODE NARROW** | Harden MCP stdio dry-run/list-call behavior only: deterministic JSON-RPC errors, bounded input/response size, stable `tools/list` schema, explicit fail-closed `tools/call` for all unbound capabilities, and tests/smoke for invalid JSON/unknown tool/large params. No real file/db/script/plugin execution. | `src-tauri/src/mcp_server.rs`, `src-tauri/src/mcp.rs` only if needed, `scripts/check-mcp-policy.py`, focused tests/checkpoint | `cargo test --features mcp mcp_server` PASS; MCP policy self/default/current PASS; no listener/network/rmcp/tokio; checkpoint + patch. |
| A4 | **START PRIVACY REVIEW ONLY** | Review W11 stdio responses/errors for secret echo: URL/userinfo/query redaction, arguments not reflected raw, no audit/log payload leakage. | `logs/assist/A4-M5-W11-*.md`; policy-only fixtures if concrete | Privacy verdict; concrete blocker if raw params can echo. |
| A5 | **START AGENT/SKILL POLICY ONLY** | Keep Agent/Skill execution locked. Add only missing policy fixtures if W10/W11 reviews found leaks; otherwise write readiness note for future execution-wave contract. | `scripts/check-agent-skill-policy.py`, `logs/assist/A5-M5-W11-*.md` | PASS/BLOCKED note; no execution runtime. |
| A6 | **START UI SMALL ONLY** | Agent/Skill UI small polish only: disabled execution affordances, deterministic empty/error/loading states, no new backend command. | `src/components/workspace/**`, `src/stores/**`, `scripts/check-agent-skill-ui-logic.mjs`, checkpoint/assist | UI logic PASS and npm build if changed. |
| A7 | **START GRAPH DOCS ONLY** | Convert W10 graph live-query card into a concrete W12 implementation plan with command contracts, limits, cancellation, privacy, and tests. Do not implement commands. | `logs/assist/A7-M5-W11-*.md`, M5 graph docs only | One graph W12 implementation card; no product code. |
| A8 | **START GRAPH UI SMALL ONLY** | Graph UI deterministic polish only if backend-free: bounded rendering tests, no-backend states, selection/search stability. | `src/components/graph/**`, `src/stores/useGraphStore.ts`, `src/utils/graphUi.ts`, `scripts/check-graph-ui-logic.mjs`, checkpoint/assist | Graph UI logic PASS; no backend graph commands. |
| A9 | **START PLUGIN DOCS/POLICY ONLY** | Turn W10 plugin runtime plan into W12/W13 staged implementation cards: manifest storage, install/enable/delete lifecycle, command isolation, audit redaction. Do not implement runtime. | `logs/assist/A9-M5-W11-*.md`, M5 plugin docs only | Plugin staged cards + blockers; no product code. |
| A10 | **START SECURITY REVIEW** | Batch review W11 outputs, especially A3 bounded stdio dry-run. Block on any listener/network, raw argument echo, side-effecting tool, default dependency pollution, or policy weakening. | `logs/assist/A10-M5-W11-*.md`; policy fixtures only for concrete failure | Security verdict after A3 output. |
| A11 | **START VERIFICATION** | Maintain W11 verification matrix: default cargo test, feature cargo test, MCP/Agent/Plugin/Graph policy, UI logic scripts, npm build, pre-merge, build metrics <=23%, warnings unchanged. | `logs/checkpoints/A11-M5-W11-*.md`, `logs/assist/A11-M5-W11-*.md` | Verification checkpoint and push readiness. |

### W11 Hard Stops

- W11 is still dry-run only: no real file/db/script/plugin/agent/skill/model execution.
- No TCP listener, HTTP server, network bind, background daemon, plugin install/enable/delete/download, model call, or hidden script/db execution.
- No raw argument/query/token/cookie/Authorization echo in stdio responses, logs, audit, checkpoints, or UI state.
- Any dependency addition must be optional and feature-gated; default build behavior must stay unchanged.
- All new or changed command surfaces must keep source check, ACL, bridge/types parity, and policy self-tests synchronized.
- Build metrics threshold is 23% after W12; cargo warnings must not increase.
- Only A0 pushes to remote.

### W11 Direct Prompts

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A1，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W11 MCP Stdio Dry-Run Hardening Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A2，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W11 MCP Stdio Dry-Run Hardening Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A3，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W11 MCP Stdio Dry-Run Hardening Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A4，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W11 MCP Stdio Dry-Run Hardening Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A5，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W11 MCP Stdio Dry-Run Hardening Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A6，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W11 MCP Stdio Dry-Run Hardening Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A7，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W11 MCP Stdio Dry-Run Hardening Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A8，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W11 MCP Stdio Dry-Run Hardening Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A9，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W11 MCP Stdio Dry-Run Hardening Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A10，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W11 MCP Stdio Dry-Run Hardening Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A11，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W11 MCP Stdio Dry-Run Hardening Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

## M5-W10 Controlled Runtime Prep Dispatch

> Added 2026-09-07 16:00 CST by A0 after W9 validation.
> Current facts: W9 focused checks are green: Agent/Skill bridge tests 26 passed, MCP tests 9 passed, plugin tests 10 passed, Agent/Skill UI logic 99 assertions, Graph UI logic 43 assertions, npm build passes, MCP/Agent policy scripts pass. W9 still needs A0 final pre-merge/push after this dispatch is committed.
> W10 goal: prepare the next runtime wave without opening unsafe execution. Only A3 may touch MCP runtime-prep code, and it must be stdio-only, feature-gated, no listener/network, no tool execution side effects. Plugin/Agent execution remains locked.

### W10 One-Line Prompt

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane AX，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W10 Controlled Runtime Prep Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

### W10 Assignments

| Lane | Status | Task | Allowed Scope | Must Deliver |
|---|---|---|---|---|
| A1 | **START DOCS ONLY** | Reconcile W9 as accepted and mark W10 active. Update M5 cards to state exactly which runtime surfaces remain locked and which A3 MCP stdio-prep slice is opened. | `PARALLEL_COMMAND_BOARD.md`, three main docs, `logs/checkpoints/M5-20260906/*.md`, `logs/checkpoints/A1-M5-W10-*.md` | One reconciliation checkpoint; no product code. |
| A2 | **START BOUNDARY REVIEW ONLY** | Review A3 W10 MCP stdio-prep plan/code for core/bin boundary: no tauri in core, no bridge::* calls from mcp tools, no duplicate script/db/plugin execution path. | `logs/assist/A2-M5-W10-*.md` | Boundary verdict with exact blockers. |
| A3 | **START PRODUCT CODE NARROW** | Implement MCP stdio-prep shell only: feature-gated `mcp` Cargo feature/bin or equivalent compile-isolated skeleton, registry/policy wiring reused from existing `mcp.rs`, no TCP listener, no network, no rmcp tool side effects, no file/db/script/plugin execution. If adding `rmcp`/`tokio`, they must be optional and required-features gated, default build unchanged. | `src-tauri/Cargo.toml`, optional `src-tauri/src/bin/mcp_server.rs`, optional `src-tauri/src/mcp_tools/**`, `src-tauri/src/mcp.rs`, `scripts/check-mcp-policy.py`, `scripts/pre-merge.sh`, focused tests/checkpoint | Default `cargo test` PASS, `cargo test --features mcp` or equivalent focused compile PASS if feature added, `check-mcp-policy.py` self/default/current PASS, no listener/network/runtime side effects. |
| A4 | **START PRIVACY REVIEW ONLY** | Review W10 MCP stdio-prep and existing Agent/Skill/Graph/Plugin surfaces for error/audit/log secret echo. Pay attention to tool result URLs, capability reasons, and serialized errors. | `logs/assist/A4-M5-W10-*.md`; policy-only fixtures if concrete | Privacy verdict. |
| A5 | **START TEST/POLICY ONLY** | Agent/Skill read-only bridge remains locked from execution. Add missing policy/tests only if A10/A4 identify a concrete leak; otherwise produce no product-code patch and record readiness for future execution design. | `scripts/check-agent-skill-policy.py`, `src-tauri/src/bridge.rs` tests only if needed, `logs/assist/A5-M5-W10-*.md` | PASS/BLOCKED note; no execution/runtime. |
| A6 | **START UI SMALL ONLY** | Agent/Skill UI: no execution buttons. Polish disabled/preview states and loading/error determinism if needed; ensure no secret text echo. | `src/components/**`, `src/stores/**`, `scripts/check-agent-skill-ui-logic.mjs`, checkpoint/assist | UI logic PASS; no backend changes. |
| A7 | **START GRAPH DOCS ONLY** | Prepare graph live-query implementation card for a later wave, including command names, result limits, cancellation, and privacy; do not implement commands. | `logs/assist/A7-M5-W10-*.md`, M5 graph docs only | One graph runtime card; no product code. |
| A8 | **START GRAPH UI SMALL ONLY** | Continue Graph UI polish only if it does not require backend commands: no-backend empty state, deterministic selection, bounded rendering. | `src/components/graph/**`, `src/stores/useGraphStore.ts`, `src/utils/graphUi.ts`, `scripts/check-graph-ui-logic.mjs`, checkpoint/assist | Graph UI logic PASS and npm build if code changed. |
| A9 | **START PLUGIN RUNTIME PLAN ONLY** | Prepare plugin runtime dispatch card: install/enable/delete/list command sequence, signature failure modes, storage limits, audit redaction, UI dependencies. Do not implement plugin runtime. | `logs/assist/A9-M5-W10-*.md`, M5 plugin docs only | One plugin runtime card with hard stops and tests. |
| A10 | **START SECURITY REVIEW** | Batch review W10 outputs, especially A3 feature-gated MCP prep. Block on any listener/network/default dependency pollution/side-effecting tool. | `logs/assist/A10-M5-W10-*.md`; policy fixtures only for concrete failure | Security verdict after A3 output. |
| A11 | **START VERIFICATION** | Maintain W10 verification matrix: default build/test, feature build/test if A3 adds `mcp`, policy scripts, UI scripts, pre-merge, build metrics <=23%, warnings unchanged. | `logs/checkpoints/A11-M5-W10-*.md`, `logs/assist/A11-M5-W10-*.md` | Verification checkpoint and push readiness. |

### W10 Hard Stops

- Only A3 may touch MCP runtime-prep product code in W10; all other runtime surfaces remain locked.
- No TCP listener, HTTP server, network bind, background daemon, plugin install/enable/delete/download, skill/agent execution, model call, or hidden script/db execution.
- Any `rmcp`/`tokio` addition must be optional, feature-gated, absent from default build behavior, and guarded by policy self-test.
- All command surfaces remain source-checked and ACL-synchronized; no new command without bridge/types/policy/tests in the same package.
- Build metrics threshold is 23% after W12; cargo warnings must not increase.
- Only A0 pushes to remote.

### W10 Direct Prompts

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A1，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W10 Controlled Runtime Prep Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A2，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W10 Controlled Runtime Prep Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A3，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W10 Controlled Runtime Prep Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A4，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W10 Controlled Runtime Prep Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A5，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W10 Controlled Runtime Prep Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A6，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W10 Controlled Runtime Prep Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A7，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W10 Controlled Runtime Prep Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A8，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W10 Controlled Runtime Prep Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A9，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W10 Controlled Runtime Prep Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A10，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W10 Controlled Runtime Prep Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A11，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W10 Controlled Runtime Prep Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

## M5-W9 Runtime-Free Polish Dispatch

> Added 2026-09-07 14:30 CST by A0 after W8 validation.
> Current facts: A3 MCP policy is current-phase (`ACTIVE=8/PENDING=0`), A5 Agent/Skill policy is green after A0 redacted `CredentialLeak` Display, Agent/Skill UI logic has 79 assertions, Graph UI logic has 41 assertions, `npm run build` passes. Build metrics are accepted at 22% because current total_bytes_pct is 21.07 and cargo warnings did not increase.
> W9 goal: finish runtime-free polish, docs, and final verification before any MCP server / plugin runtime / skill execution wave.

### W9 One-Line Prompt

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane AX，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W9 Runtime-Free Polish Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

### W9 Assignments

| Lane | Status | Task | Allowed Scope | Must Deliver |
|---|---|---|---|---|
| A1 | **START DOCS ONLY** | Reconcile W8 as accepted-with-fixes and mark W9 as active NEXT. Update M5 child-card status for MCP read-only bridge, Agent/Skill read-only bridge, Graph UI polish, plugin review, and build metrics threshold 22%. | `PARALLEL_COMMAND_BOARD.md`, three main docs, `logs/checkpoints/M5-20260906/*.md`, `logs/checkpoints/A1-M5-W9-*.md` | One reconciliation checkpoint; no product code. |
| A2 | **START REVIEW ONLY** | Re-review command boundary after A3/A5 W8 fixes: no runtime server/listener, no core tauri leak, read-only bridge only, no duplicate execution path. | `logs/assist/A2-M5-W9-*.md` | Boundary verdict with concrete blockers only. |
| A3 | **START POLICY/REVIEW ONLY** | Keep MCP policy current-phase green and prepare a later M5-2.b card for actual rmcp/server work. Do not implement rmcp/server/listener/network. | `scripts/check-mcp-policy.py` only if a regression is found, `logs/assist/A3-M5-W9-*.md` | Policy/review note; self-test/default/current-gaps command results. |
| A4 | **START PRIVACY REVIEW ONLY** | Re-review all command error/display surfaces for secret echo after the A0 `CredentialLeak` redaction fix. Check Agent/Skill/MCP/Plugin/Graph visible errors and audit text. | `logs/assist/A4-M5-W9-*.md`, optional policy fixtures only | Privacy verdict; no product code unless policy-only bad sample is concrete. |
| A5 | **START PRODUCT CODE SMALL** | Finish Agent/Skill read-only bridge hardening only: add focused tests for redacted validation errors and frontend parse/permission preview edge cases. No execution, install, persistence write, network, or model call. | `src-tauri/src/bridge.rs`, `src-tauri/src/agent.rs`, `src-tauri/src/skills.rs`, `scripts/check-agent-skill-policy.py`, `src/bridge.ts`, `src/types.ts`, checkpoint/patch | Focused Rust tests PASS and agent/skill policy PASS. |
| A6 | **START UI LOGIC ONLY** | Agent/Skill panel consumption polish: deterministic empty/error/loading states, no secret text echo in UI state, bounded preview rendering. | `src/components/**`, `src/stores/**`, `scripts/check-agent-skill-ui-logic.mjs`, `logs/assist/A6-M5-W9-*.md` | UI logic assertions PASS; no backend command changes. |
| A7 | **START GRAPH CONTRACT DOCS ONLY** | Finalize next graph bridge contract for later live graph query commands, separating runtime-free UI/store items from blocked backend runtime items. | `logs/assist/A7-M5-W9-*.md`, optional M5 graph card docs | One graph bridge contract note. |
| A8 | **START GRAPH UI SMALL** | Graph UI polish only: keep filters/search/layout deterministic, preserve bounded arrays, improve no-backend/read-only states if needed. No backend graph commands. | `src/components/graph/**`, `src/stores/useGraphStore.ts`, `src/utils/graphUi.ts`, `scripts/check-graph-ui-logic.mjs`, checkpoint/patch | Graph UI logic PASS and `npm run build` PASS if code changed. |
| A9 | **START PLUGIN REVIEW ONLY** | Plugin surface review after W8: verify manifest/lifecycle remains pure and real install/enable/delete/download commands are still absent. Do not implement plugin runtime. | `logs/assist/A9-M5-W9-*.md`, optional policy-only patch | Plugin verdict and any exact next runtime blockers. |
| A10 | **START SECURITY FINAL REVIEW** | Batch review W8/W9 outputs after A5/A6/A8/A9 finish. Focus source check, ACL parity, read-only guarantees, redaction, and no runtime expansion. | `logs/assist/A10-M5-W9-*.md` | One final security verdict. |
| A11 | **START FINAL VERIFICATION** | Produce final W9 verification matrix with exact command results, build metrics 21.07 <= 22%, cargo warnings unchanged, pre-merge result, remaining GUI/runtime debts, and push readiness. | `logs/checkpoints/A11-M5-W9-*.md`, `logs/assist/A11-M5-W9-*.md` | Verification checkpoint; no product code. |

### W9 Hard Stops

- No MCP server/listener/rmcp runtime, no plugin install/enable/delete/download runtime, no skill/agent execution, no model call, no network access.
- New/changed commands must remain read-only and keep source check + ACL + bridge/types + policy/tests in the same package.
- No token/cookie/Authorization/body/prompt-secret in logs, audit, frontend state, checkpoints, or error strings.
- Build metrics threshold is 22% for this wave; any increase beyond 22% or any cargo warning increase blocks A0 push.
- Only A0 pushes to remote.

### W9 Direct Prompts

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A1，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W9 Runtime-Free Polish Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A2，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W9 Runtime-Free Polish Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A3，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W9 Runtime-Free Polish Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A4，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W9 Runtime-Free Polish Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A5，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W9 Runtime-Free Polish Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A6，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W9 Runtime-Free Polish Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A7，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W9 Runtime-Free Polish Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A8，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W9 Runtime-Free Polish Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A9，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W9 Runtime-Free Polish Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A10，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W9 Runtime-Free Polish Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A11，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W9 Runtime-Free Polish Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

## M5-W8 Full-Lane Follow-up Dispatch

> Added 2026-09-07 09:45 CST by A0.
> Current facts: A3 MCP read-only bridge code is present in local master and focused checks pass except the old `--expect-pending` phase gate, which now correctly flags that W1 pending assumptions must be retired. A5 Agent/Skill read-only bridge focused checks pass. W7 assist/review notes from A1/A2/A4/A5/A7/A8/A9/A10/A11 are present.
> W8 goal: close the A3 MCP policy pending-mode debt, harden Agent/Skill read-only bridge, wire/polish UI consumption, refresh graph/plugin plans, and produce one batch verification package before A0 push.

### W8 One-Line Prompt

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane AX，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W8 Full-Lane Follow-up Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

### W8 Assignments

| Lane | Status | Task | Allowed Scope | Must Deliver |
|---|---|---|---|---|
| A1 | **START DOCS ONLY** | Reconcile W7 full acceptance and promote W8 as active NEXT. Record that A3 W7 is present but has `check-mcp-policy.py --expect-pending` phase debt to close. Update M5 readiness/debt list for MCP bridge, Agent/Skill bridge, graph UI, and plugin policy. | `PARALLEL_COMMAND_BOARD.md`, `AI-模型切换与接手清单.md`, `详细设计与实施计划.md`, `后续需求TODO.md`, `logs/checkpoints/M5-20260906/*.md`, `logs/checkpoints/A1-M5-W8-*.md` | One reconciliation checkpoint; no product code. |
| A2 | **START REVIEW ONLY** | Boundary review of A3/A5 W7 command bridges: confirm MCP and Agent/Skill bridges stay bin-side/read-only, do not leak into `mvp_core`, duplicate script execution, graph runtime, plugin runtime, or start MCP server/listener. Identify exact files/commands that must remain isolated. | `logs/assist/A2-M5-W8-*.md` | Boundary review note with PASS/BLOCKED and actionable line/file references. |
| A3 | **START POLICY FIX ONLY** | Close W7 MCP policy phase debt: `check-mcp-policy.py --expect-pending` currently fails because MCP artifacts now exist. Convert W1 pending semantics into W7/W8 active checks or replace the mode with a current-phase assertion; keep default/self-test green. Do not add rmcp/server/listener/network/runtime execution. | `scripts/check-mcp-policy.py`, `scripts/pre-merge.sh` only if needed, `logs/checkpoints/A3-M5-W8-*.md`, optional policy-only fixtures | `check-mcp-policy.py --self-test` PASS, default PASS, current-phase mode PASS; focused MCP Rust tests still PASS; no product runtime expansion. |
| A4 | **START REVIEW ONLY** | Privacy/memory review for Agent/Skill bridge and graph/plugin surfaces: credential redaction, bounded preview payloads, audit contents, no prompt/body persistence, no local secret capture. May propose policy fixtures but do not edit product code. | `logs/assist/A4-M5-W8-*.md` | Review note plus concrete recommended bad samples if any. |
| A5 | **START PRODUCT CODE** | Harden Agent/Skill read-only bridge after A0 validation: add missing edge-case tests, improve validation error shape if needed, ensure ACL/source check/policy script and frontend bridge/types stay atomic. No execution, install, network, persistence writes, or plugin enablement. | `src-tauri/src/agent.rs`, `src-tauri/src/skills.rs`, `src-tauri/src/bridge.rs`, `src-tauri/src/main.rs`, `src-tauri/permissions/default-commands.toml`, `src/bridge.ts`, `src/types.ts`, `scripts/check-agent-skill-policy.py`, focused checkpoint/patch | Focused Rust tests PASS; `check-agent-skill-policy.py` self-test/default PASS; no A3/MCP files unless strictly shared ACL/type hunk and documented. |
| A6 | **START UI DOCS/LOGIC** | Convert W7 UI wiring note into Agent/Skill panel consumption plan and pure UI helper tests. No live command execution unless bridge functions already exist and are typed; no visual redesign. | `src/components/**`, `src/stores/**`, `src/types.ts`, `src/bridge.ts`, `scripts/check-agent-skill-ui-logic.mjs`, `logs/assist/A6-M5-W8-*.md` | UI logic test PASS and checkpoint/assist note describing exact wiring state. |
| A7 | **START DOCS/GRAPH BRIDGE PLAN ONLY** | Advance graph bridge plan without MCP/A3 dependency: define read-only graph query command contract, capacity/error states, and how GraphPanel consumes existing graph store. Do not implement backend commands. | `logs/assist/A7-M5-W8-*.md`, optional `logs/checkpoints/M5-20260906/M5-7*.md`, `M5-8*.md`, `M5-9*.md` | One graph bridge card with blocked-by-A3/MCP items separated from independently shippable UI/store items. |
| A8 | **START UI POLISH/TEST ONLY** | Review and polish Graph UI pure logic already landed: accessibility labels, empty/error/oversize states, deterministic filters/search, no unbounded arrays. Do not add backend graph commands. | `src/components/**`, `src/stores/**`, `scripts/check-graph-ui-logic.mjs`, `logs/assist/A8-M5-W8-*.md` | `npm run build` or focused UI logic PASS; patch/checkpoint. |
| A9 | **START POLICY REVIEW ONLY** | Plugin surface W8 review: ensure no install/enable/delete/download/runtime command slipped in, lifecycle remains pure, and capability verdict text is bounded/redacted. May extend plugin policy docs/tests only if concrete failure. | `logs/assist/A9-M5-W8-*.md`, optional `scripts/check-plugin-policy.py` policy-only hunk | Review note or policy patch; no runtime product code. |
| A10 | **START SECURITY BATCH REVIEW** | Batch security review of all non-A3 W8 outputs after at least A5/A6/A8/A9 report. Focus source check, ACL drift, read-only guarantees, redaction, command payload bounds, no hidden execution/install/network. | `logs/assist/A10-M5-W8-*.md`; policy fixtures only for concrete failures | One security verdict, not per-file drip updates. |
| A11 | **START VERIFICATION BATCH** | Maintain W8 verification matrix including A3. Record exact commands, pass/fail, residual debt, and whether A0 may push after A3 policy phase debt closes. | `logs/assist/A11-M5-W8-*.md`, `logs/checkpoints/A11-M5-W8-*.md` | One final verification delta after implementation lanes finish. |

### W8 Hard Stops

- A3 may edit only MCP policy/checkpoint files in W8; no new MCP product-code commands beyond the existing W7 read-only bridge.
- No rmcp runtime, server/listener, plugin install/enable/delete/download, skill execution, model calls, or network access.
- Every command touched by A5 must remain read-only and must include source check, ACL, frontend bridge/types, policy coverage, and focused tests in the same package.
- No token/cookie/Authorization/body/prompt-secret logging, audit, persistence, checkpoint, or frontend state.
- Lanes must deliver a coherent patch/checkpoint and must not ask A0 to merge tiny partial notes.
- Only A0 pushes to remote.

### W8 Direct Prompts

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A1，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W8 Full-Lane Follow-up Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A2，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W8 Full-Lane Follow-up Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A3，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W8 Full-Lane Follow-up Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A4，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W8 Full-Lane Follow-up Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A5，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W8 Full-Lane Follow-up Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A6，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W8 Full-Lane Follow-up Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A7，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W8 Full-Lane Follow-up Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A8，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W8 Full-Lane Follow-up Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A9，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W8 Full-Lane Follow-up Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A10，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W8 Full-Lane Follow-up Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A11，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W8 Full-Lane Follow-up Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

## M5-W6 Parallel Dispatch

> Added 2026-09-06 19:25 CST by A0 after pushing through `4b438ef`.
> Current facts: Agent/Skill UI shell is integrated; graph model/store policy slice is integrated; build metrics threshold is documented at 19% with W5 debt.
> W6 opens A8 and A9 product-code lanes only. A8 may build graph UI pure logic/panel shell over A7 DTOs; A9 may build plugin manifest/lifecycle policy slice. No graph live agent consumption and no plugin installation runtime yet.

### W6 One-Line Prompt

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane AX，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W6 Parallel Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

### W6 Assignments

| Lane | Status | Task | Allowed Scope | Must Deliver |
|---|---|---|---|---|
| A1 | **START DOCS ONLY** | Reconcile W6 as active NEXT; mark W5 pushed and tighten M5-9/M5-10/M5-11/M5-12 acceptance criteria. | `PARALLEL_COMMAND_BOARD.md`, three main docs, `logs/checkpoints/M5-20260906/*.md` | One reconciliation checkpoint; no product code. |
| A2 | **SUPPORT/REVIEW ONLY** | Review A8/A9 for core boundary and seam direction; no product code. | `logs/assist/A2-M5-W6-*.md` | Boundary review note. |
| A3 | **SUPPORT/REVIEW ONLY** | Review plugin/graph exposure against MCP registry policy; no MCP runtime expansion. | `logs/assist/A3-M5-W6-*.md` | MCP compatibility note. |
| A4 | **SUPPORT/REVIEW ONLY** | Review graph/plugin interactions with agent memory privacy/capacity; no product code. | `logs/assist/A4-M5-W6-*.md` | Review note. |
| A5 | **SUPPORT/REVIEW ONLY** | Review A9 plugin manifest against Agent/Skill domain and permission preview assumptions. | `logs/assist/A5-M5-W6-*.md` | Review note. |
| A6 | **SUPPORT/REVIEW ONLY** | Review A8 UI consistency with existing Agent/Skill UI shell; docs only. | `logs/assist/A6-M5-W6-*.md` | UI consistency note. |
| A7 | **SUPPORT/REVIEW ONLY** | Review A8 graph UI against A7 DTO/query helpers; no graph product code unless fixing docs only. | `logs/assist/A7-M5-W6-*.md` | Graph contract review note. |
| A8 | **START PRODUCT CODE** | Implement M5-9 graph UI pure logic and panel shell: graph list/search/filter, node detail summary, capacity/error/empty states, helper module + headless logic test. Do not call live agent consumption or backend graph commands unless already existing and fully typed. | `src/components/**`, `src/stores/**`, `src/types.ts`, `src/bridge.ts` only if no new command, `scripts/check-graph-ui-logic.mjs`, optional UI policy script, docs/checkpoint | `npm run build` PASS, UI logic test PASS, no new dependency, no live agent consumption. |
| A9 | **START PRODUCT CODE** | Implement M5-10/M5-11 plugin manifest/lifecycle policy slice: DTOs, validation, lifecycle state machine, permission manifest rules, policy script. No install/uninstall file mutation runtime, no downloaded plugins, no signature enforcement beyond pure validation unless fully local. | `src-tauri/src/domain.rs`, optional `src-tauri/src/plugin.rs`, `src-tauri/src/security_policy.rs`, `scripts/check-plugin-policy.py`, `scripts/pre-merge.sh`, focused Rust tests/checkpoint | Policy self-test/default PASS, bounded metadata, no secrets, no network/install runtime. |
| A10 | **START REVIEW** | Security review A8/A9 for secret display, unbounded UI/store growth, plugin path traversal, signature bypass, command exposure, and ACL drift. | `logs/assist/A10-M5-W6-*.md`; policy fixtures only for concrete failure | Review after A8/A9 output. |
| A11 | **START VERIFICATION** | Update W6 verification matrix and GUI/manual debt ledger after A8/A9 outputs. | `logs/assist/M5-A11-W6-*.md`, `logs/checkpoints/M5-A11-W6-*.md` | One verification delta. |

### W6 Hard Stops

- Only A8 and A9 may write product code in W6.
- A8 must not add backend commands, live agent consumption, model calls, or graph rebuild workers.
- A9 must not install, delete, download, execute, or enable real plugins; pure manifest/lifecycle policy only.
- Any new command requires source check, ACL, frontend bridge/types, policy coverage, and tests in the same package; prefer no command in W6.
- All stores/maps/lists must be bounded; no token/cookie/Authorization/body/prompt-secret logging or persistence.
- All lanes pull from `origin/master` first and must not push.

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

## M5-W17 Desktop Client Completeness and Home Recovery Dispatch

### Product decision and boundary

The user has reported that the installed desktop client is fast but feels incomplete: the home page lacks important entry points and the client must be made ready for a focused human look-through before further runtime-platform work. This is a **real product-code wave**, but it is deliberately bounded to existing local client capabilities.

Preserve M5-W16 outputs. Do not start any M6 authority slice. Do not add command execution, plugin invocation, dynamic loading, remote download/listener, daemon, model call, Agent/Skill execution, MCP live runtime, graph writes/exports, or background workers. No new dependency. No new Tauri command, bridge capability, ACL entry, filesystem permission, or network privilege.

### Shared acceptance criteria

1. A developer can start the desktop debug client without landing on `localhost:1421` connection refused: the documented helper detects/starts the Vite dev server, waits for it, launches the desktop app, and cleans up only the server it owns on exit. Release behavior must continue to use bundled assets.
2. The home view exposes the existing principal work areas as usable routes, keeps user shortcuts/recent items bounded and resilient to malformed local data, and has coherent empty/loading states. It must not become a marketing page.
3. Navigation and active-panel presentation make existing functionality discoverable in the native client, including narrow-window behavior. No backend contract changes.
4. All new visible controls are keyboard reachable, have an accessible name, and use existing visual language. No sensitive URL/query/credential/local-path disclosure in the new UI or errors.
5. No agent may evade the build-metric guard. Record the metric result; if an implementation exceeds its current budget, stop and report the exact delta rather than raising the ceiling.
6. Each lane delivers a complete batch: scoped code/tests where assigned, a checkpoint, exact commands/results, `git diff --check`, and a binary patch. Do not commit or push.

### Lane assignments

| Lane | Status | Allowed scope | Must deliver |
|---|---|---|---|
| A1 | START DOCS | `AI-模型切换与接手清单.md`, `详细设计与实施计划.md`, `后续需求TODO.md`, `logs/checkpoints/` | Reconcile retained W16 outputs and write the W17 user-visible acceptance checklist/known limitations. No product code. |
| A2 | START CODE | `run-gui.sh`, `scripts/` startup helpers/tests, narrowly `src-tauri/src/main.rs` only when needed for debug-vs-release asset selection | A reproducible, ownership-safe desktop dev start path and smoke check. Do not alter commands, bridge, ACL, or production runtime authority. |
| A3 | START CODE | `src/stores/useHomeStore.ts`, `src/utils/homeUi.ts` (new if useful), `scripts/check-home-store-logic.mjs` | Bounded, validated home shortcut/recent-item state with stable defaults and migration-safe malformed-data handling. No component styling or backend edits. |
| A4 | START REVIEW | `scripts/check-home-client-policy.py` (new), `logs/assist/`, `logs/checkpoints/` | Static privacy/security review for W17 UI/startup changes; guard against credentials, raw sensitive URLs/query values, shell injection, and privilege expansion. No product UI edits. |
| A5 | START CODE | `src/components/home/` only | Rebuild the existing home surface around A3's state contracts: principal-area launchers, shortcuts/recent section, polished empty states, and responsive layout. Use no new dependency and no backend access. |
| A6 | START CODE | `src/components/layout/ActivityBar.vue`, `src/stores/useLayoutStore.ts`, `scripts/check-client-navigation-logic.mjs` | Make existing modules discoverable and keyboard-accessible with stable active state/narrow-window behavior. Do not edit home components, `MainArea.vue`, or backend files. |
| A7 | START CODE | `src/components/layout/MainArea.vue`, `src/components/layout/StatusBar.vue`, `src/App.vue` only | Improve client shell fallback/boot/error presentation for existing panels without adding runtime behavior; ensure no blank/dead main area when a panel fails to resolve. |
| A8 | START MANUAL QA | `logs/assist/`, `logs/checkpoints/` | Run the actual native client after W17 changes are integrated or from a supplied patch. Capture an honest desktop/narrow-window acceptance checklist; report blockers, never fabricate screenshots. |
| A9 | START TEST | `scripts/check-home-ui-logic.mjs` (new), `scripts/pre-merge.sh` only if integration is required, `logs/checkpoints/` | DOM/source-level checks for A5 home actions, accessible names, bounded lists, and no sensitive copy. Do not modify product components. |
| A10 | START SECURITY REVIEW | `logs/assist/`, `logs/checkpoints/` | Review W17 patch boundaries against the locked runtime-authority list and desktop startup safety. Findings first; no product-code modifications. |
| A11 | START VERIFICATION | `logs/assist/`, `logs/checkpoints/` | Independent integration-verification matrix: targeted tests, `npm run build`, build metrics, and relevant Rust tests. No source edits except a self-contained verification script if essential. |

### Integration order

`A2 -> A3 -> A5 -> A6 -> A7 -> A4/A9 -> A8/A10/A11 -> A0`.

Each lane must inspect existing dirty W16 material and preserve it. If its permitted file is already modified by another lane, it must produce a binary patch instead of overwriting or resolving cross-lane conflicts. A0 alone integrates, commits, and pushes after review.

### M5-W17 Acceptance Closeout Dispatch

> Current evidence: targeted W17 checks PASS, `npm run build` PASS, and `bash scripts/pre-merge.sh` is `ALL_PASS`. Do not repeat completed UI work. The remaining product debt is `HOME_NO_SECRET_PERSIST`: app execution command bodies are still stored in browser storage by `useHomeStore.ts`. Native desktop visual acceptance is still user-side evidence only.

| Lane | Status | Allowed scope | Must deliver |
|---|---|---|---|
| A1 | START DOCS | `AI-模型切换与接手清单.md`, `详细设计与实施计划.md`, `后续需求TODO.md`, `logs/checkpoints/` | Record W17 evidence, the one real home-storage debt, and the native-client manual-acceptance boundary. No product code. |
| A2 | START VERIFY | `run-gui.sh`, `scripts/check-dev-startup.sh`, `logs/assist/`, `logs/checkpoints/` | Re-run the ownership-safe desktop startup smoke path and document exactly how the user launches the native client. No runtime authority changes. |
| A3 | START CODE | `src/stores/useHomeStore.ts`, `src/utils/homeUi.ts`, `scripts/check-home-store-logic.mjs` | Remove or redesign persistence of app command bodies so browser storage keeps only non-sensitive home metadata; preserve migration and bounded-state behavior. Add regression assertions. |
| A4 | START REVIEW | `scripts/check-home-client-policy.py`, `logs/assist/`, `logs/checkpoints/` | Re-check the A3 fix and W17 privacy boundary; default policy must pass, and report any intentionally pending debt. No product UI edits. |
| A5 | HOLD | `src/components/home/` | No new work unless A3 changes the state contract and a focused UI adjustment is proven necessary. |
| A6 | HOLD | `src/components/layout/ActivityBar.vue`, `src/stores/useLayoutStore.ts`, `scripts/check-client-navigation-logic.mjs` | No new work; existing navigation checks are green. Re-open only for a concrete acceptance finding. |
| A7 | HOLD | `src/components/layout/MainArea.vue`, `src/components/layout/StatusBar.vue`, `src/App.vue` | No new work; existing shell fallback is covered. Re-open only for a concrete acceptance finding. |
| A8 | START MANUAL QA | `logs/assist/`, `logs/checkpoints/` | Run the actual native client and record desktop, narrow-window, home launcher, and startup results. Screenshots may be attached only if genuinely observed. |
| A9 | START TEST REVIEW | `scripts/check-home-ui-logic.mjs`, `scripts/check-client-navigation-logic.mjs`, `logs/checkpoints/` | Re-run focused UI logic checks after A3; add only narrow assertions needed for the persistence fix. No component edits. |
| A10 | START SECURITY REVIEW | `logs/assist/`, `logs/checkpoints/` | Review the final W17 diff for raw sensitive persistence, shell/privilege expansion, and startup ownership. Findings first; no product-code edits. |
| A11 | START VERIFICATION | `logs/assist/`, `logs/checkpoints/` | Final matrix after A3: policy self-tests/defaults, focused UI checks, `npm run build`, build metrics, `bash scripts/pre-merge.sh`, and `git diff --check`. No source edits. |

### Closeout integration order

`A3 -> A4/A9 -> A2/A8/A10 -> A1 -> A11 -> A0`.

Only A0 integrates, commits, and pushes. Agents must preserve the existing dirty worktree, must not reset or clean unrelated changes, and must deliver a checkpoint plus binary patch when their allowed files overlap.

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane AX，先 cd 到 WORKDIR，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W17 Acceptance Closeout Dispatch 完成自己的整包交付；A5/A6/A7 按 HOLD 规则只检查是否有具体阻塞，不重复开发；自行整理 checkpoint，不 push。
```

### BUG-HUNT Follow-up Dispatch

> `logs/bug-hunt/BUG-HUNT-SUMMARY.md` has been reviewed. A0 directly fixed B9-1 (WebKitGTK hide deadlock recurrence) and B3-1 (normal script exit leaving inherited pipe readers/supervisor stuck). B5-1 is not a routine patch: it requires a tested persistence-before-spawn transaction boundary. Do not let a general lane rewrite it opportunistically.

| Lane | Status | Scope | Must deliver |
|---|---|---|---|
| A0 | DONE | `tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/platform/linux.rs`, `src-tauri/src/script_runner.rs` | Fixed B9-1 WebKitGTK hide deadlock recurrence and B3-1 normal-exit supervisor/reader hang. Rust tests and fmt PASS. |
| A2 | DONE | `src-tauri/src/security_policy.rs`, `scripts/check-security-policy.py` | Launch target hardening for wrappers, inline interpreters, and symlink resolution; focused tests and policy self-test PASS. |
| A3 | DONE | `src-tauri/src/workspace.rs` | Atomic writes plus `.corrupt` backup/error handling; workspace tests PASS. |
| A4 | DONE | `src-tauri/src/scheduler.rs`, `src-tauri/src/tasks.rs`, `src-tauri/src/bridge.rs`, `logs/checkpoints/` | Scheduler reserves and persists the trigger slot before spawn; scheduler/task commands share a storage transaction lock; crash-window and concurrent load-modify-save tests pass. |
| A5 | DONE | `src-tauri/src/tools.rs`, focused tests | User tool HTML reads are capped at 2 MiB before allocation; tools tests 4/4 PASS. |
| A6 | DONE | `src/stores/useSystemStore.ts`, `src/components/system/ClipboardPanel.vue`, `scripts/check-clipboard-persistence-logic.mjs` | Clipboard history is session-only, bounded, and redacted in the panel; focused test 16/16 PASS. |
| A7 | DONE | `src-tauri/src/domain.rs`, `src/types.ts`, database/agent/skill UI adapters, focused tests | Confirmed Rust/TypeScript DTO casing drift fixed; Rust fixtures and database UI checks PASS. |
| A8 | DONE | `src/stores/useBrowserStore.ts`, `scripts/check-grid-close-logic.mjs` | `closeGridAll` restores browser view, reactivates the tab, and repositions the webview; focused test 12/12 PASS. Native visual QA remains user-side. |
| A9 | DONE | `scripts/check-command-set-consistency.py`, `logs/checkpoints/` | Three-way command/ACL/source consistency gate added and PASS; known legacy drift is explicitly allow-listed. |
| A10 | DONE | `src/utils/markdown.ts`, affected renderers and tests | Confirmed Markdown link/XSS path fixed with focused logic checks. |
| A11 | DONE | `logs/bug-hunt/`, `logs/checkpoints/` | B8/B10/B12 recheck complete; confirmed, stale, unsupported, and unverified findings separated. |

BUG-HUNT order: `A2/A3/A4/A5/A6/A7/A8/A9/A10/A11 -> A0`. A0 has also fixed B10-b MainArea fallback pairing after A1/A11 confirmation. A4 scheduler review and implementation are complete; remaining work is A0 integration/commit/push. Simple follow-up items such as error-redaction spread, crash-log rotation, rel attributes, and UI guards remain ordinary-model work.

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane AX，读取 BUG-HUNT-SUMMARY.md 和 PARALLEL_COMMAND_BOARD.md，按 BUG-HUNT Follow-up Dispatch 做自己的整包任务；先确认当前代码未被其它 lane 改写，交付测试、checkpoint 和二进制补丁，不 push。
```

### Lane output template

```text
LANE: A?
STATUS: PASS | PASS_WITH_DEBT | BLOCKED
SCOPE: <files actually changed>
DELIVERED: <user-visible behavior>
VERIFY: <exact command>: <result>
METRICS: <result or N/A>
PATCH: <absolute patch path>
RISKS: <honest remaining risk>
NO_PUSH: confirmed
```

## M5-W16 M5 Closeout and M6 Charter Dispatch

> W15 is accepted locally by A0 pending this integration. W16 produces the only permissible next step: a reviewed M6 charter before any new runtime authority is implemented. Only A0 pushes.

| Lane | Scope | Must Deliver |
|---|---|---|
| A1 | Reconcile W15 acceptance and author the M6 WBS/ordering proposal. | Checkpoint + WBS draft. |
| A2 | Architecture boundaries for a candidate M6 runtime slice; enumerate non-goals and hard stops. | Decision note. |
| A3 | MCP runtime threat-model and fail-closed contract only. | Review note. |
| A4 | Privacy/stable-error contract for every candidate M6 surface. | Review note. |
| A5 | Agent/Skill execution authorization design only; no execution code. | Contract note. |
| A6 | Plugin/confirmation accessibility backlog and budget proposal only. | UI note. |
| A7 | Graph write/export threat model and readonly regression plan. | Review note. |
| A8 | Manual GUI/runtime acceptance matrix for the approved future slice. | Checklist. |
| A9 | Frozen backend contract inventory and migration risks. | Inventory. |
| A10 | Consolidated security release review and M6 entry gate. | Verdict. |
| A11 | M5 final verification matrix and M6 verification-plan skeleton. | Checkpoint. |

Hard stops: no product code, no new command/ACL/bridge/DTO, no plugin invoke/execution, dynamic loading, network download/listener, daemon, model call, Agent/Skill execution, MCP expansion, graph write/export, background worker, raw Tauri invoke, or sensitive rendering/persistence.

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane AX，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W16 M5 Closeout and M6 Charter Dispatch 完成自己的整包文档/审查交付，自行整理 checkpoint，不写产品代码，不 push。
```

## M5-W15 Release Readiness Dispatch

> W15 closes M5 evidence and GUI acceptance without expanding runtime authority. Only A0 pushes.

| Lane | Scope | Must Deliver |
|---|---|---|
| A1 | Docs only: reconcile W14 accepted, track the 25% metric limit and W15 status. | Checkpoint + patch. |
| A2 | Boundary review of W14 UI/store and W15 evidence. | Verdict. |
| A3 | MCP isolation regression review. | Verdict. |
| A4 | Plugin UI privacy and stable-error review. | Verdict. |
| A5 | Agent/Skill execution-lock regression review. | Verdict. |
| A6 | Narrow UI polish only: accessible confirmation/modal focus, empty/loading/error states, no new command or runtime surface. | UI patch + headless tests. |
| A7 | Graph non-regression review. | Note. |
| A8 | GUI/manual acceptance checklist and visual ergonomics review. | Checklist + verdict. |
| A9 | Frozen W13 backend-contract review; no backend changes. | Note. |
| A10 | Security release review, including raw-invoke and error rendering. | Verdict. |
| A11 | Final verification matrix and push readiness. | Checkpoint. |

Hard stops: no plugin invoke/execution, dynamic loading, network download/listener, daemon, model call, Agent/Skill execution, MCP expansion, graph write/export, background worker, raw Tauri invoke, or sensitive rendering/persistence.

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane AX，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W15 Release Readiness Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

## M5-W14 Plugin Manager UI Dispatch

> Added 2026-09-08 00:20 CST by A0 after W13 local acceptance. This wave consumes the eight existing local-only Stage-I commands; it does not expand runtime authority.

### W14 Hard Stops

- Work only in `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3` on `master`; pull fast-forward before starting when clean. Only A0 pushes.
- No `plugin_invoke`, code execution, dynamic loading, network download/listener, daemon, model call, Agent/Skill execution, MCP runtime expansion, graph write/export, or background worker.
- UI must call only `src/bridge.ts`; no raw Tauri `invoke`.
- Do not display or persist a raw signature, public-key material, resource path, manifest metadata, credentials, request/response bodies, stdout, or stderr.
- Existing command names and DTOs are frozen. Do not add lifecycle commands in W14.

### W14 Lane Table

| Lane | Status | Scope | Allowed Files | Must Deliver |
|---|---|---|---|---|
| A1 | START DOCS ONLY | Reconcile W13 accepted and W14 active; update M5-12/13/14 status without changing product scope. | Board, three main docs, `logs/checkpoints/M5-20260906/*.md`, W14 checkpoint | Docs package and patch. |
| A2 | START REVIEW ONLY | Check UI/store boundary: no domain duplication, raw bridge invoke, or lifecycle authority expansion. | `logs/assist/A2-M5-W14-*.md` | Boundary verdict. |
| A3 | START MCP REVIEW ONLY | Confirm no plugin lifecycle command becomes reachable through MCP stdio. | `logs/assist/A3-M5-W14-*.md`, MCP fixture only if concrete | Isolation verdict. |
| A4 | START PRIVACY REVIEW ONLY | Check rendered DTO/error/state paths for secret, signature, key, metadata, and path echo. | `logs/assist/A4-M5-W14-*.md`, privacy fixture only if concrete | Privacy verdict. |
| A5 | START AGENT/SKILL REVIEW ONLY | Confirm plugin UI contains no Agent/Skill/tool execution affordance. | `logs/assist/A5-M5-W14-*.md` | Lock verdict. |
| A6 | START PRODUCT CODE NARROW | Build a compact Plugin Manager panel/store that lists, filters, inspects, installs a supplied manifest, enables/disables, and manages trusted-key fingerprints through frozen `bridge.ts` methods. Show only redacted DTO fields; use explicit confirmation for state-changing actions; add headless UI logic tests and integrate the panel into existing workspace navigation. | `src/components/plugin/**`, `src/stores/usePluginStore.ts`, existing workspace/layout navigation files, `src/bridge.ts`/`src/types.ts` only for frozen-type corrections, `scripts/check-plugin-ui-logic.mjs`, checkpoint | Usable local-only UI, no raw invoke, no execution action, UI tests and npm build PASS. |
| A7 | SUPPORT DOCS ONLY | Graph non-regression review only. | `logs/assist/A7-M5-W14-*.md` | Note. |
| A8 | SUPPORT UI REVIEW ONLY | Review A6 UI ergonomics and existing graph/workspace non-regression; no plugin product code. | `logs/assist/A8-M5-W14-*.md` | UI review. |
| A9 | SUPPORT BACKEND REVIEW ONLY | Verify A6 consumes the frozen W13 contract without backend changes; document any missing read-only presentation field. | `logs/assist/A9-M5-W14-*.md` | Contract note. |
| A10 | START SECURITY REVIEW | Review raw-invoke bypass, confirmation bypass, source/ACL drift, and sensitive-rendering regressions after A6 output. | `logs/assist/A10-M5-W14-*.md` | Security verdict. |
| A11 | START VERIFICATION | Maintain W14 matrix: plugin-focused Rust, plugin/privacy/MCP/Agent-Skill/graph policies, UI logic, npm build, pre-merge, diff check. | `logs/checkpoints/A11-M5-W14-*.md`, `logs/assist/A11-M5-W14-*.md` | Push readiness evidence. |

### One-Line Prompts

```text
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane AX，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W14 Plugin Manager UI Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

Replace only `AX` with `A1` through `A11`.

## M5-W18-R Research and Replication Blueprint Dispatch

> Historical W18-R foundation, extended by R2 and the current R2B task cards. A0 is the controller; A1-A11 are research lanes. Earlier W18 implementation assignments remain superseded.

### Pinned reference inputs

- Obsidian behavior/configuration reference: `/home/ainfinit/Documents/Knowledge-Base/secondBrain/.obsidian` (`graph.json`, `app.json`, `core-plugins.json`) and Markdown content under the same vault. Borrow vault, wikilink, backlink/outgoing-link, orphan, filter, and local-graph interaction semantics; Obsidian itself is not an open-source code dependency.
- dbx implementation reference: `/home/ainfinit/Documents/极智简单/V3/research/dbx-src`; local unversioned snapshot with `Cargo.lock` SHA-256 `c0a7be12c05d8dffe867f1a70d4b82e881dec2da3bb622b5d3e3410c3d10e3a7`, Apache-2.0. Existing analysis lives in `/home/ainfinit/Documents/极智简单/V3/dbx-study/`.
- zvec-grep implementation reference: `/home/ainfinit/Documents/极智简单/V3/research/zvec-grep-src`, upstream `zvec-ai/zvec-grep` pinned at `52653951b24617762f4ab0c71c34d594e5001617` (Apache-2.0). Borrow workspace scoping, ignore rules, incremental freshness, exact/BM25/vector route separation, compact results, and explicit remote-embedding authorization. A0 decides adoption after A7/A8/A9 evidence and A10/A11 review; A3 owns UX, not engine authority. R2's explicit A7/A8 sandbox exceptions remain narrow research exceptions.

Reference code is evidence, not yet the product contract. W18-R exists to make later reuse mechanical and safe: identify exact source modules/functions/tests that can be transplanted, document required adaptations, and reject incompatible pieces before writing product code.

### Copy and replication policy

- dbx and zvec-grep are Apache-2.0. W19 may copy or adapt selected source only after A10 records provenance, license/NOTICE obligations, modified-file notices, dependency compatibility, and the exact destination architecture.
- Obsidian is not an open-source source-code donor. Reproduce only observable behavior and documented/local vault formats; do not copy proprietary code, icons, branding, or assets.
- No wholesale repository copy. Prefer the smallest proven modules and their tests. Preserve upstream attribution and record each transplanted file/function in a source ledger.
- A copied implementation is not accepted merely because it compiles. It must pass this product's source check, ACL, privacy, capacity, lifecycle, build-size, UI, and release-origin gates.

### Why the last client failed and the non-repeat gate

1. The debug main window is programmatically loaded from `http://localhost:1421/`, while release loads bundled `tauri://localhost` assets. Tauri authorization is origin-sensitive.
2. The commands in the screenshots were already registered and listed in ACL. The failure was not a missing command; the debug external origin did not match the local-only capability.
3. Restoring a global `build.devUrl` is forbidden: this repository previously produced a raw release binary that still depended on Vite because Tauri's build configuration exposed the dev path.
4. The accepted solution is the exact debug-only capability `src-tauri/dev-capabilities/main.json`, dynamically registered only under `#[cfg(debug_assertions)]` and kept outside `src-tauri/capabilities/` so release cannot auto-include it.
5. Never broaden default `remote.urls` to fix one screen. Never call a browser preview a native-client test. Never claim a new command works until implementation + source check + handler registration + ACL + typed bridge/types + policy tests all land together.
6. Mandatory regression gates for any lane touching commands, capabilities, startup, or native UI: `check-command-set-consistency.py`, `check-dev-startup.sh`, focused tests, `npm run build`, and `pre-merge.sh`.

### W18-R research-only boundary

- No edits under product-code paths: `src/`, `src-tauri/`, `tauri-browser-tabs/`, production `scripts/`, permissions/capabilities, package manifests, lockfiles, or build configuration.
- No dependency install, model download, daemon/server, MCP exposure, database connection, remote embedding, network upload, credential access, or mutation of the user's Obsidian vault.
- Read local source/configuration and run read-only tests or benchmarks only on synthetic, non-secret fixtures.
- Every proposed feature must be classified `COPY`, `ADAPT`, `REIMPLEMENT_FROM_BEHAVIOR`, `DEFER`, or `REJECT`, with reasons.
- Every report must include: current-product gap, upstream source map with exact files/symbols, data/control flow, persistence format, concurrency/lifecycle, security/privacy, performance/capacity, dependencies/licenses, target mapping, test reuse, and unresolved questions.
- Research is complete only when A0 can create implementation cards without asking agents to rediscover architecture.

### Dedicated lane worktrees

Each research lane uses `/home/ainfinit/.codex/worktrees/m5-w18-aN/mvp-browser-os-v3` on branch `codex/m5-w18-aN`. Do not edit the canonical master working copy. Rebase onto `origin/master`, then write only lane-owned reports under `logs/research/M5-W18/` and a lane checkpoint. Only A0 integrates and pushes.

### Lane assignments

| Lane | Status | Allowed scope | Long-package deliverable |
|---|---|---|---|
| A1 | RESEARCH | `logs/research/M5-W18/A1-*`, lane checkpoint | Baseline the current product end to end: graph/database Rust modules, DTOs, commands, stores, panels, policies, tests, known debt, build-size budget, and locked authorities. Produce the canonical gap inventory and a requirements checklist that all other reports must answer. |
| A2 | RESEARCH | `logs/research/M5-W18/A2-*`, lane checkpoint | Reverse-engineer Obsidian vault semantics from the local vault and public behavior: Markdown/wikilinks, aliases, headings/blocks, tags, frontmatter, attachments, unresolved links, backlinks/outgoing links, ignore filters, rename/delete behavior, and graph data derivation. Define behavior-compatible test fixtures without touching the vault. |
| A3 | RESEARCH | `logs/research/M5-W18/A3-*`, lane checkpoint | Reverse-engineer Obsidian graph/search UX: global/local graph, filters, groups, orphans, unresolved nodes, depth, selection, navigation, keyboard/accessibility, settings persistence, empty/error/loading states, and narrow-window behavior. Produce screen/state flows and a Vue component/state blueprint; no Obsidian assets or code copying. |
| A4 | RESEARCH | `logs/research/M5-W18/A4-*`, lane checkpoint | Map dbx backend architecture with exact Rust files/symbols/call paths for connection registry, driver capabilities, schema discovery, query execution, cancellation, history, export, SQL analysis/risk, errors, shutdown, and tests. Mark smallest transplantable Apache-2.0 modules and every adaptation required for this repository. |
| A5 | RESEARCH | `logs/research/M5-W18/A5-*`, lane checkpoint | Map dbx desktop workbench UX with exact components/stores/tests: connection tree, schema browser, editor tabs, execution toolbar, cancel/progress, result grids, paging/filter/copy/export, history, keyboard/accessibility, loading/empty/error states, and responsive layout. Produce a Vue-specific replication blueprint rather than copying framework-incompatible UI wholesale. |
| A6 | RESEARCH | `logs/research/M5-W18/A6-*`, lane checkpoint | Audit dbx security and lifecycle: credential storage, DSN redaction, production verdict/write confirmation, query timeout/cancel races, pool/session cleanup, bounded rows/bytes/files, history persistence, import/export, error/log sanitization, and shutdown. Compare every finding to current guards and produce mandatory W19 policy/test cases. |
| A7 | RESEARCH | `logs/research/M5-W18/A7-*`, lane checkpoint | Reverse-engineer zvec-grep ingestion/index architecture: workspace authorization, discovery/ignore rules, extractors/chunking, manifest/files/index formats, incremental update, watcher reconciliation, corruption/rebuild, concurrency, capacity, and compact output. Map exact source files/symbols/tests and identify reusable Apache-2.0 units. |
| A8 | RESEARCH | `logs/research/M5-W18/A8-*`, lane checkpoint | Reverse-engineer and benchmark zvec-grep retrieval on a synthetic corpus: managed ripgrep, FTS/BM25, vector, hybrid/RRF, filters, limits, ranking metadata, stale-index behavior, multilingual/code behavior, latency/memory/index size, and unavailable-model failure. Produce an evidence-based route-selection and adoption recommendation; no product dependency install. |
| A9 | RESEARCH | `logs/research/M5-W18/A9-*`, lane checkpoint | Audit zvec-grep trust boundaries: local/remote embeddings, workspace grants, API-key storage, MCP toolsets, bearer auth, server/daemon lifecycle, logs, query/content redaction, model download, and data egress. Produce a threat model and list what W19 may copy, must adapt, or must keep disabled. |
| A10 | RESEARCH | `logs/research/M5-W18/A10-*`, lane checkpoint | Build the source-transplant ledger and dependency BOM across dbx/zvec-grep plus behavior-only Obsidian replication. Record source path/symbol, upstream revision, license, notices, dependency license/native footprint, destination module, modification plan, copied tests, and reject reasons. Propose attribution/NOTICE files and a no-blind-copy review gate. |
| A11 | RESEARCH | `logs/research/M5-W18/A11-*`, lane checkpoint | Synthesize a verification architecture from A1-A10 evidence: target acceptance matrix, synthetic vault/database/search corpora, unit/integration/security/performance/GUI cases, debug/release tests, migration/rollback, implementation slice boundaries, merge order, and stop criteria. Identify contradictions for A0; do not declare W18 complete before all reports exist. |

### Dependencies and merge order

- All eleven lanes start immediately and write disjoint report files.
- Evidence dependency: A1 baseline informs everyone; A10 must consume A2/A4/A7-A9; A11 must consume A1-A10. If predecessors are unfinished, A10/A11 continue independent analysis and mark exact pending inputs instead of writing product code.
- A0 research integration order: `A1 -> A2/A3 -> A4/A5/A6 -> A7/A8/A9 -> A10 -> A11 -> A0 consolidated architecture decision`.
- W19 is not automatic. A0 opens it only after the source ledger, target architecture, test matrix, security boundaries, and license obligations are complete and mutually consistent.

### Required lane output

```text
LANE=A1..A11
STATUS=PASS | PASS_WITH_DEBT | BLOCKED
BASE=<origin/master sha>
HEAD=<lane research commit sha>
REFERENCE_EVIDENCE=<exact local paths and pinned revisions read>
FILES=<changed files>
SOURCE_MAP=<exact upstream and local files/symbols examined>
CLASSIFICATION=<COPY/ADAPT/REIMPLEMENT_FROM_BEHAVIOR/DEFER/REJECT counts>
VERIFY=<read-only tests/benchmarks and exact results>
CHECKPOINT=<path>
MERGE_NOTES=<dependencies/conflicts/security debt>
NEXT=<missing research input or proposed W19 slice>
```

### One-line prompt for all 11 agents

```text
LANE=A1；读取 /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3/WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按当前派发入口推导本lane目录/分支并进入，完成自己的整包任务；沿用已有成果，按卡自检并提交本lane，不改范围外文件、不push。
```

Replace only the declared lane value. Directory/branch derivation is specified once in the active R2B card.

## M5-W18-R2 Evidence Closure Dispatch

> Added after reviewing all eleven first-round branches. R2 evidence obligations remain required, supplemented by the current R2B cards. R1/R2 reports are inputs awaiting review, not implementation authority. Read `logs/checkpoints/A0-M5-W18-R1-audit-20260908.md` and the current R2B checkpoint.

### R2 common evidence contract

1. Rebase the lane branch onto the latest `origin/master`. Work only in the assigned lane worktree and commit locally; never edit the canonical master working copy and never push.
2. Do not modify product code, manifests, lockfiles, production scripts, capabilities, ACLs, or the user's Obsidian vault. R2 output is limited to the lane's existing `logs/research/M5-W18/A<N>-*` files and lane checkpoint.
3. Every factual claim must identify one of `CURRENT_PRODUCT`, `REFERENCE_SOURCE`, `OBSERVED_BEHAVIOR`, `OFFICIAL_DOC`, `EXECUTED_SYNTHETIC_TEST`, or `INFERENCE`, with exact path/symbol/line or command/result. Do not present inference as observed behavior.
4. Add a correction section that explicitly retracts or replaces every R1 statement named in the A0 audit. Preserve history; do not silently rewrite a false claim without noting the correction.
5. The final blueprint must name exact current destination files/symbols, proposed additions, dependency closure, data flow, lifecycle, capacity, stable errors, tests, migration, rollback, and hard stops. “Add a store/component/service” without mapping the current target is insufficient.
6. `COPY` means legally and technically eligible for a later reviewed transplant. It does not authorize copying in R2. Each COPY item needs upstream file + symbol + transitive local dependencies + external crates/packages + copied tests + required attribution + target destination. Otherwise classify it `ADAPT` or `REIMPLEMENT_FROM_BEHAVIOR`.
7. A report may be `PASS` only when it has no unresolved question that would force a W19 coding lane to rediscover architecture. Otherwise use `PASS_WITH_DEBT` and name the exact blocker.

### R2 lane assignments

| Lane | R1 verdict | R2 evidence-closure package |
|---|---|---|
| A1 | REWORK | Re-run the current-product inventory from `origin/master`. Record the exact 135/135/46 command facts, 29-Python/50-total policy counting rules, current Rust/frontend test inventory, real stores/components, build-size gate, locked authorities, and current duplicate `DbValue` definitions. Produce a machine-checkable fact appendix and a corrected gap list; remove stale W17 assumptions. |
| A2 | REWORK | Complete Obsidian semantics using read-only local-vault samples plus official Obsidian documentation. Build synthetic fixtures and expected outputs for wikilinks, path disambiguation, aliases, headings, block IDs, embeds, tags/frontmatter, attachments, unresolved links, rename/delete, case sensitivity, ignored files, backlinks/outgoing links, orphan detection, and duplicate basenames. Mark every rule observed/documented/inferred and never expose private vault content in the report. |
| A3 | REWORK | Consume A2 via `git show codex/m5-w18-a2:<path>`. Reconcile global/local graph, group/filter/orphan/unresolved semantics and distinguish verified behavior from design choice. Map every screen/state to the actual current `GraphPanel.vue`, graph store/types/bridge, layout navigation, and existing UI checks. Deliver exact component/state/event deltas, accessibility/keyboard/narrow-window behavior, and acceptance screenshots as a future test plan, not fabricated evidence. |
| A4 | REWORK | Correct the product baseline by reading current `database.rs`, `domain.rs`, `bridge.rs`, `main.rs`, ACL and all database tests. For each dbx COPY/ADAPT candidate, trace the complete function-level call/dependency closure, feature flags, async/runtime assumptions, Send/Sync constraints, error types, persistence schema, tests, and expected binary-size impact. Produce a minimal target diff against the existing database module, not a greenfield design. |
| A5 | REWORK | Re-inventory the real frontend (`useDatabaseStore.ts`, `DatabasePanel.vue`, `dbUi.ts`, `bridge.ts`, `types.ts`, layout lazy-loading and package dependencies). Remove nonexistent `useConnectionStore` and stale contract references. Compare each desired dbx behavior to current UI, then provide a minimum-change Vue component plan, exact state transitions, keyboard/accessibility/responsive behavior, empty/loading/error/cancel states, and bundle-cost alternatives. Do not select CodeMirror or another dependency without measured need and footprint. |
| A6 | REWORK | Draw source-to-sink credential flows for both current product and dbx: form/JS argument/Tauri serialization/Rust memory/pool/keyring or file secret store/log/error/audit/shutdown. Verify actual dbx `FileSecretStore`, encrypted-state, `save_password=false` session store, migration and cleanup call paths. Add cancellation/timeout/write-confirmation race timelines and mandatory fail-closed tests. Correct the false “never enters JS memory” claim. |
| A7 | REWORK | Resolve `@zvec/zvec@0.7.0` instead of leaving it open. In a disposable `/tmp/m5-w18-a7-zvec` workspace only, inspect the pinned npm tarball/native binding package, exported API, supported platforms, install scripts, ABI, license/NOTICE, and whether a public Rust crate/source or stable on-disk format exists. Prove or reject format/API compatibility. Then define a watcher-free, bounded, explicitly triggered first ingestion slice separately from later watcher/reconciliation work. |
| A8 | REWORK | Keep the useful managed-ripgrep test, fix the product license, and execute the missing route evidence on synthetic non-secret data. A0 authorizes dependency installation/model download only inside disposable `/tmp/m5-w18-a8-zvec`, using the pinned zvec-grep lock and local-only models; record package/model versions, hashes, disk size, peak RSS, cold/warm latency, indexing time, index size, result quality, stale-index and unavailable-model behavior. No daemon, MCP, remote embedding, user data, or product dependency changes. If a native/platform blocker occurs, capture the exact reproducible failure and benchmark every remaining executable route. |
| A9 | REWORK | Reconcile A7/A8 findings with the trust model. Correct COPY/REIMPLEMENT inconsistencies, inspect local daemon/socket/filesystem permissions and multi-user threats, trace all egress fields for remote embedding, verify API-key and grant storage/migration/logging, model-download integrity, MCP toolset boundaries, shutdown and denial behavior. Produce a target Rust/Tauri trust-boundary design with default-disabled authorities and exact policy tests. |
| A10 | REWORK | Verify the product MulanPSL-2.0 license and Apache-2.0 inbound compatibility/obligations. Consume A2/A4/A7/A8/A9 branch reports directly with `git show`; do not finalize while any is missing or superseded. Replace module-level COPY guesses with function-level provenance/dependency/test closures, reconcile all classification conflicts, identify native/package/bundle impact, and draft exact NOTICE/attribution/modified-file requirements. End with ACCEPT/REJECT per transplant unit, not a blanket “ready for W19”. |
| A11 | REWORK | Consume A1-A10 final R2 reports directly with `git show` and rebuild the verification architecture from corrected facts. Give exact synthetic corpora, unit/integration/security/performance/native-GUI/debug-release/migration/rollback cases, commands and expected assertions. Resolve the policy-count, DbValue and zvec-route contradictions. Propose only implementation slices whose prerequisites, destination files, merge order, rollback and stop criteria are fully known; issue final GO/NO-GO per slice. |

### R2 sequencing and exit gate

- A1-A9 start immediately. A3 must re-read A2 before finalizing. A9 must re-read A7/A8 before finalizing.
- A10 starts its own verification immediately but may finalize only after A2/A4/A7/A8/A9 R2 commits exist.
- A11 starts its matrix immediately but may finalize only after A1-A10 R2 commits exist.
- Agents read peer outputs from local branch refs with `git show`; they must not merge, cherry-pick, or edit peer files.
- A0 accepts R2 only when all eleven branches are clean, every correction is explicit, A10's ledger and A11's matrix agree, and no unresolved item would change the first implementation slice.
- Until A0 records `W19=OPEN`, all product-code work remains forbidden.

### R2 one-line prompt

```text
LANE=A1；读取 /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3/WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按当前派发入口推导本lane目录/分支并进入，完成自己的整包任务；沿用已有成果，按卡自检并提交本lane，不改范围外文件、不push。
```

Replace only the first declared `A1` with the assigned lane. Do not manually edit a second path or branch placeholder.
