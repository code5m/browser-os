# Lane A5 — M5-W18-R3 Database Workbench (inside browser-first shell) · Checkpoint

- **Role:** RESEARCH / Replication Blueprint (per `PARALLEL_COMMAND_BOARD.md` Active Lanes table, A5 = RESEARCH)
- **Dispatch:** `M5-W18-R3-UX-TASKS-20260908.md` §A5 — revise the database prototype so connection tree, SQL documents, result grid, history and properties appear on demand and collapse cleanly; active result/editor regains full viewport in focus mode; include multi-document, cancel, truncation, error and right-click states.
- **Verdict source:** `M5-W18-PROTOTYPE-REVIEW-20260908.md` → REVISE: reject the first IDEA-style shell prototype, keep the Chrome-like browser-first model. This lane therefore builds the database workbench *inside* the browser-first shell (top chrome + nav + left rail + edges), not as a separate IDEA shell.
- **Hard constraints honored:** no product code changes, no other-lane file edits, no `git push`. Only `logs/research/M5-W18/A5-R3-*` added.

## Worktree / Branch
- Path: `/home/ainfinit/.codex/worktrees/m5-w18-a5/mvp-browser-os-v3`
- Branch: `codex/m5-w18-a5`
- BASE: `200f0f1` (rebased onto `origin/master`)
- Rebase note: A5's 4 prior R2B commits were byte-identical to what A0 already integrated into `origin/master`, so `git rebase origin/master` cleanly skipped them (`skipped previously applied commit`), leaving the branch at `200f0f1` with zero other-lane dirty files.

## Deliverables (all under `logs/research/M5-W18/`)
| File | Purpose | Verification |
|---|---|---|
| `A5-R3-state-model.mjs` | Normative source: layout constants, shell/tool-window state machine, multi-doc model, command registry (62 actions / 13 scopes), 78 assertions G1–G8 | `node A5-R3-state-model.mjs` → **ALL_PASS (78 checks)** |
| `A5-R3-prototype.html` | Self-contained clickable prototype (no IPC/network/persistence): 2-row chrome + nav, left rail, synthetic 400-table connection/structure tree, multi-SQL-doc subtabs, result grid (NULL/trunc/bin/error states), history, messages, properties tool windows, scoped right-click menus, focus-mode pill, live viewport readout | `node --check` syntax OK; G8 consistency vs state-model verified |
| `A5-R3-database-shell.md` | 15-section design report: R2B→R3 revision log (M1–M9), layout contract w/ measured budget table, tool-window state machine, on-demand/collapse, tree behavior, multi-doc, state matrix, command-registry table, feasibility ledger of 6 missing commands + 1 backend-rejected action, migration gaps w/ `file:line` cites, cross-lane handoffs, open questions Q1–Q5, proposed W19 gates | markdown, cited |
| `A5-R3-checkpoint.md` | This file | — |

## Key results
- **Layout budget (collapsed mode, strict active area) passes at all 4 required sizes** with the 2-doc worst case:
  - 1920×1080: 90.30% height / 98.54% width
  - 1440×900: 88.30% / 98.06%
  - 1366×768: 86.22% / 97.95%
  - 1024×768: 85.26% / 97.27%
  - Constraints: ≥85% post-titlebar height & ≥92% width at 1440×900, 2-row/80px top chrome — all met.
  - 800×600 (non-required) noted at 82.17% (2-doc) → open question Q3 (proposed D-A5-R3-3 for <768px).
- **On-demand / collapse**: every tool window supports open/close/collapse-all/restore/expand/pin/unpin/auto-hide/resize; exactly one primary tool window per edge; focus mode hides all chrome (top/nav/rail/edges) with one Esc/return action, returning the active result/editor to full viewport.
- **Multi-document**: `createSqlDoc`/`startRun`/`applyDocResult`/`cancelDoc` with `execId` stale-guard (a late-arriving result for a cancelled/replaced exec is dropped). Cancel maps to a still-missing backend `db_cancel`.
- **Command registry**: 62 actions across 13 scopes; right-click menus are scoped per-object via `[data-scope]` with identity / disabled-reason / keyboard / a11y-path / safety-class.
- **Backend feasibility ledger** (carried forward from R2B):
  - Missing commands: `db_cancel`, `db_list_schema`, `db_export_result`, `db_query_history`, `db_list_connections`, `db_row_write`.
  - Backend-rejected action: `db.table.truncate` (DDL) → `WriteDenied` via `require_write_confirmation` (`security_policy.rs:1206-1218`, WriteDenied for Ddl).
  - Frozen commands reused as-is: `db_connect`/`db_query`/`db_disconnect` (`bridge.rs:6009/6040/6097`).

## Cross-lane handoffs
- To **A0/A1**: W19 gate proposal — prototype-consistency (G8) must wire the `.mjs` normative source into `pre-merge.sh` before any product code lands.
- To **A4**: backend `db_cancel` command name freeze + `db_list_schema`/`db_export_result`/`db_query_history`/`db_list_connections`/`db_row_write` delivery (matches A4 W18-R classification ADAPT/REIMPLEMENT_FROM_BEHAVIOR).
- To **A9**: workspace grant unification needed for the connection tree scope (per A7 W18-R "workspace authorization" finding).
- To **A6/A8**: focus-mode return action + edges chrome-hidden contract must be consumed by the layout store when product code lands.

## Open questions (carried to W19)
- **Q1**: Should focus mode fully hide the left rail, or keep a 1px hover reveal? (prototype hides fully, single return action.)
- **Q2**: Truncation threshold for grid cells — borrow `DB_LIMITS` from `src/utils/dbUi.ts` or define a new `GRID_TRUNCATE_CHARS`?
- **Q3**: Sub-768px (e.g. 800×600) collapses below 85%; do we accept the D-A5-R3-3 debt or add a compact single-edge mode?
- **Q4**: History persistence — in-memory only in prototype; product needs `db_query_history` backend before persistence claims.
- **Q5**: Multi-document subtab cap — prototype allows unbounded; propose a soft cap (e.g. 12) with overflow menu.

## Compliance confirmation
- ✅ No product code edited (zero `.rs`/`.vue`/`.ts`/`.toml` changes).
- ✅ No other-lane files edited (only `logs/research/M5-W18/A5-R3-*` added).
- ✅ No `git push` performed (commit only; A0 is the sole integrator/pusher per board).
- ✅ Rebased onto `origin/master`; branch starts at `200f0f1`, ahead by this single commit.
