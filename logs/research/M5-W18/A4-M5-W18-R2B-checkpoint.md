# A4 M5-W18-R2B Checkpoint

```
LANE=A4
DISPATCH=M5-W18-R2B
STATUS=READY_FOR_REVIEW
WORKDIR=/home/ainfinit/.codex/worktrees/m5-w18-a4/mvp-browser-os-v3
BRANCH=codex/m5-w18-a4
BASE=434e63f3fc2cb9457d9fc15fbb3014ce4d9a48e6
HEAD=<set on commit of this report + checkpoint>
PRIOR_A4_R1=971378c (rebase后 b737e5d) — preserved, not rewritten
REFERENCE_EVIDENCE=dbx-src (Apache-2.0, local read-only, github.com/t8y2/dbx); repo src-tauri/src/{database,domain,bridge}.rs; src/{types.ts,utils/dbUi.ts}; src/components/workspace/DatabasePanel.vue
FILES=logs/research/M5-W18/A4-R2B-backend-evidence-closure.md, logs/research/M5-W18/A4-M5-W18-R2B-checkpoint.md
       (prior A4-dbx-backend-architecture-map.md, A4-checkpoint.md preserved)
CLASSIFICATION=COPY=1 proven (csv_export::push_csv_escaped, std-only); ADAPT=3 (query_cancel sync port, production_safety diff, db_export pipeline);
              REIMPLEMENT_FROM_BEHAVIOR=5 (history, schema, query_execution, PoolKind-minimal, sql_risk gap has_use_statement small);
              DEFER=2 (governance, non-core drivers); REJECT=4 (jdbc, FileSecretStore, no-ACL, governance)
VERIFY=read-only static mapping + grep (23 #[test] confirmed); cargo test NOT_RUN; no DB/IPC connection; no product code
CORRECTIONS=C1 tests 22->23 (declared); C2 DbValue drift -> confirmed B8-1 root cause; C3 dbx identity=t8y2/dbx (IDEA UNKNOWN);
            C4 push_csv_escaped closure+feasibility; C5 sql_risk gap reclassified to small REIMPLEMENT
CHECKPOINT=logs/research/M5-W18/A4-M5-W18-R2B-checkpoint.md
```

## Progress (R2B additions over R1)
- [x] Rebased `codex/m5-w18-a4` onto `origin/master` (`434e63f`); worktree clean, ahead 1.
- [x] Read `WORKSPACE_IDENTITY.md`, `PARALLEL_COMMAND_BOARD.md` (R2B entry + A4 §5), `M5-W18-R2B-TASKS-20260908.md` §5, `A0-M5-W18-R1-audit-20260908.md`, `A0-M5-W18-R2B-dispatch-20260908.md`.
- [x] Verified DB test count: **23** `#[test]` in `database.rs` (lines 814-1262) — corrects R1 audit's "22". Declared=23; executed=NOT_RUN.
- [x] Traced `DbValue` triple-source to confirmed root cause of B8-1: `database.rs::DbValue` wires `i64`/`f64`/`binary` (snake_case of I64/F64/Binary) but `types.ts`/`domain.rs` expect `int`/`float`/`blob_len` → `decodeDbValue` always falls back. Also `DbQueryResult` field drift (TS has `state`/`limit_hit` not produced by backend).
- [x] Confirmed dbx identity = `t8y2/dbx` (independent Rust OSS, Apache-2.0); IDEA Database Tools = UNKNOWN (experience reference only).
- [x] Proved `csv_export::push_csv_escaped`/`push_csv_escaped_content` are pure `std`-only (function-level closure + compile feasibility — fills R1 audit gap).
- [x] Reclassified `sql_risk::mcp_sql_has_forbidden_database_switch` from ADAPT to small REIMPLEMENT (it drags dbx's full SQL parser).
- [x] Produced function-level reuse-closure table for connection/schema/exec/cancel/timeout/history/export mapped to sync targets (§6).
- [x] Designed multi-SQL-document isolation contract (fixed `query_id`, no stale-overwrite) for A5/A6 (§7.1).
- [x] Split S0 (DbValue single-source + runnable baseline) and S2 (multi-doc DB loop) into independently-committable PROPOSED_NOT_AUTHORIZED cards (§7.2/§7.3).

## Debt / open items (A4-owned, not blocking review)
- **D-A4-R2B-1**: `cargo test` not re-run (research lane); declared=23, pass count NOT_RUN.
- **D-A4-R2B-2**: `db_cancel` command name freeze pending A0/M4-3.a (contract uses `query_id` param to avoid blocking).
- **D-A4-R2B-3**: async-vs-sync confirmed as sync (repo M4-1.a forbids tokio); all ports are synchronous.
- **D-A4-R2B-4**: IDEA precise-replication claim stays UNKNOWN (no evidence dbx == user's IDEA plugin).

## Conflicts / security notes (unchanged from R1, re-affirmed)
- 🔴 Credential storage: REJECT dbx `FileSecretStore`/`state_persistence.rs` as main path; repo uses keyring.
- 🔴 Command ACL: REJECT dbx no-whitelist habit; keep `default-commands.toml`.
- 🔴 JDBC sidecar: REJECT (JRE bloat).
- ⚠️ dbx `[patch.crates-io]` vendors + gaussdb/mysql_async forks: do NOT import (native-footprint/fork burden). Repo uses standard sync `rusqlite`/`mysql`/`postgres`.
- ✅ Repo's `sanitize_message` + `DbError.discard_connection` + `QueryCancel` L1 flag are sound; extend, don't replace.

## Merge notes
- Upstream dependency: A1 baseline (a54eed1, read-only reference). A4 consumed its own prior R1 report.
- Consumer lanes: A10 (transplant scope per §2 C5 / §6 — narrow sql_risk to small REIMPLEMENT; COPY only csv escape); A5 (multi-doc UI per §7.1); A6 (cancel race by fixed `query_id`); A11 (verification matrix).
- This lane wrote **no product code**; only `logs/research/M5-W18/` reports + checkpoint. Ready for A0 review (commit; later push by A0 only).

## Next (proposed W19 slices, PROPOSED_NOT_AUTHORIZED, gated by A0 slice-opening)
1. **S0**: unify `DbValue`/`DbQueryResult` on `domain.rs` single source (fixes B8-1, zero frontend change); add `state`/`limit_hit` to backend or trim TS.
2. **S2**: connection tree + independent SQL docs + query/cancel/result (A5 consumes §7.1).
3. Backend execution layer: sync fetch channel (sqlite/mysql/postgres) + TLS decision; cancel registry L2/L3; schema discovery; history persistence; export pipeline.
