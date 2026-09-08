# A4 M5-W18-R Checkpoint

```
LANE=A4
STATUS=PASS_WITH_DEBT
BASE=78d2cfb8e90d323d35df920e9807d32189f867cd
HEAD=<set on commit>
REFERENCE_EVIDENCE=/home/ainfinit/Documents/极智简单/V3/research/dbx-src (Apache-2.0, Cargo.lock SHA c0a7be12c05d8dffe867f1a70d4b82e881dec2da3bb622b5d3e3410c3d10e3a7); /home/ainfinit/Documents/极智简单/V3/dbx-study/*.md (V5 prior analysis); repo src-tauri/src/{database,security_policy,domain,keyring_store,bridge}.rs
FILES=logs/research/M5-W18/A4-dbx-backend-architecture-map.md, logs/research/M5-W18/A4-checkpoint.md
CLASSIFICATION=COPY=3, ADAPT=5, REIMPLEMENT_FROM_BEHAVIOR=4, DEFER=2, REJECT=4
VERIFY=read-only static source mapping only (W18-R L1381 forbids DB connection / dependency install); dbx tests/ inspected not executed
CHECKPOINT=logs/research/M5-W18/A4-checkpoint.md
```

## Progress
- [x] Read `WORKSPACE_IDENTITY.md` + `PARALLEL_COMMAND_BOARD.md` M5-W18-R dispatch (L1350-1438).
- [x] Confirmed W18-A4 worktree `m5-w18-a4` (branch `codex/m5-w18-a4`) exists, HEAD = BASE `78d2cfb`, clean, rebased to origin/master.
- [x] Read prior V5 analysis (`dbx-study/*.md`) as prior art; independently re-verified every cited symbol by reading dbx-core source.
- [x] Mapped dbx backend data-plane across all 11 W18-R dimensions with exact file:line symbols (report §3).
- [x] Established current-product gap baseline from `database.rs`/`security_policy.rs`/`domain.rs` (report §2) — repo already autonomously implemented SQL-risk + production-detection + sanitization; dbx is a cross-check reference, not sole source.
- [x] Produced per-capability classification COPY/ADAPT/REIMPLEMENT/DEFER/REJECT (report §10) and smallest-transplantable-module list for A10 (§11).
- [x] Target mapping dbx→repo (§12), test-reuse discipline (§13), unresolved questions for A0 (§14).

## Debt / open items (A4-owned, not blocking)
- **D-A4-1**: SQL-risk/production-detection **diff** between repo `security_policy.rs` and dbx `sql_risk.rs`/`production_safety.rs` not yet line-by-line executed — flagged for A6/A10 before any adoption. This report classifies dbx `sql_risk`/`production_safety` as ADAPT (diff-only) precisely because repo already implements them.
- **D-A4-2**: Async-vs-sync runtime decision (repo M4-1.a forbids tokio) must be confirmed by A0 before W19 ports `query_cancel`/`query_execution` — all移植 must be sync.
- **D-A4-3**: `db_cancel` command name frozen-status (database.rs:187) blocks L3 interrupt reachability — needs A0/M4-1.c adjudication.

## Conflicts / security notes
- 🔴 Credential storage: REJECT dbx `FileSecretStore`/`state_persistence.rs` as main path; repo uses keyring.
- 🔴 Command ACL: REJECT dbx no-whitelist habit; keep `default-commands.toml`.
- 🔴 JDBC sidecar: REJECT (JRE bloat).
- ⚠️ dbx `[patch.crates-io]` vendors + gaussdb/mysql_async forks: do NOT import into repo (native-footprint/fork burden). Repo uses standard sync `rusqlite`/`mysql`/`postgres`.
- ✅ Repo's `sanitize_message` + `DbError.discard_connection` + `QueryCancel` L1 flag are sound foundations; extend, don't replace.

## Merge notes
- Predecessor dependency: A1 baseline (repo DB module inventory) — A4 assumed A1's inventory; if A1 diverges, re-check §2 gap table.
- Consumer lanes: A10 (provenance ledger + NOTICE for every transplanted file), A6 (W19 SQL-risk/production policy cases from the diff), A5 (export pipeline consumer of `csv_export` COPY), A11 (verification matrix).
- This lane wrote **no product code**; only `logs/research/M5-W18/` reports. Ready for A0 integration (commit + later push by A0 only).

## Next (proposed W19 slices, not executed here)
1. Sync fetch channel for sqlite/mysql/postgres (+ TLS lib decision). 2. Cancel registry L2/L3. 3. Schema discovery. 4. History persistence. 5. Export pipeline. Each gated by A10 provenance + A6 policy + repo sync-runtime contract.
