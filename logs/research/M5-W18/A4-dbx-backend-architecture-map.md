# A4 — dbx Backend Architecture Research & Replication Blueprint

> **Lane**: A4 · `M5-W18-R Research and Replication Blueprint Dispatch` (board L1398)
> **Role**: Map dbx backend (data-plane only) with exact Rust files/symbols/call paths; mark smallest transplantable Apache-2.0 modules and every adaptation required.
> **Research-only**: No product-code edits, no dependency install, no DB connection, no push. Reports only.
> **BASE** = `78d2cfb8e90d323d35df920e9807d32189f867cd` (origin/master, "docs(M5-W18): switch lanes to research-first blueprint")
> **Reference evidence**:
> - dbx src: `/home/ainfinit/Documents/极智简单/V3/research/dbx-src` (Apache-2.0, Cargo.lock SHA `c0a7be12…`, workspace: `src-tauri`, `crates/dbx-core`, `crates/dbx-web`, `crates/dbx-mcp`, `crates/dbx-cli`)
> - Prior V5 analysis: `/home/ainfinit/Documents/极智简单/V3/dbx-study/` (`dbx-借鉴分析.md`, `dbx-功能借鉴清单.md`, `dbx-冲突风险.md`) — treated as prior art, **independently re-verified** below.
> - Current product: `src-tauri/src/{database,security_policy,domain,keyring_store,bridge}.rs`

---

## 1. Scope & boundary (W18-R L1398)

**In scope (backend data-plane)**: connection registry, driver capabilities, schema discovery, query execution, cancellation, history, export, SQL analysis/risk, errors, shutdown, tests.

**Out of scope (other lanes)**: AI/Agent/A2P/A2A (`agent_*`, `ai_*`), MCP server (`dbx-mcp`), WebView2 recovery (`webview2_recovery.rs`), governance connections (`consul/`, `mq/`, `nacos/`, `mqtt/`), front-end workbench UX (A5), security lifecycle audit (A6 — this report flags, A6 owns W19 policy/test cases).

**Hard red lines honored** (`dbx-冲突风险.md` §1–§5, re-affirmed):
- 🔴 Command ACL: dbx registers 200+ commands with **no capability whitelist**; this repo keeps `default-commands.toml` per-command allow-list. Never copy dbx's no-ACL habit.
- 🔴 Credential storage: dbx `FileSecretStore` (`connection_secrets.rs:37`) + `state_persistence.rs:476-547` (argon2id+AES-256-GCM) is **not** the main path; this repo uses system keyring (`keyring_store.rs`). REJECT as main path.
- 🔴 JDBC sidecar (`jdbc.rs`): JRE ~hundreds of MB, violates lightweight goal. REJECT.
- 🟡 Driver model: dbx `PoolKind` 23 variants on async deadpool/tokio; this repo uses **synchronous** `rusqlite(bundled)`+`mysql`+`postgres`, explicitly **no tokio runtime, no async** (M4-1.a/b contract). Do not import dbx's async pool wholesale.

---

## 2. Current-product gap baseline

The repo already自主 (M4-2 work) implemented several dbx backend faces, so dbx is a **cross-check reference**, not the sole source. Verified state:

| Capability | Repo status | Evidence |
|---|---|---|
| Config types `SupportedDb`/`DbConnectionConfig`/`DbErrorCode` (18-code closure) | ✅ Implemented | `domain.rs` via `database.rs:21` `use crate::domain::{DbConnectionConfig,DbErrorCode,DbSslMode,SupportedDb}` |
| Limits (SQL 64KiB / rows 1000 / result 4MiB / timeout 30s..600s / cancel-every-64-rows / export 16MiB / name 128B) | ✅ Constants | `database.rs:39-58` |
| Credential namespace `db:<conn_id>` via keyring | ✅ Implemented | `database.rs:65-70`, `keyring_store.rs` |
| Error type `DbError` (sanitized message, `discard_connection`) | ✅ Implemented | `database.rs:80-112` |
| Result DTO `DbQueryResult`/`DbValue` (BLOB→length only) | ✅ Implemented | `database.rs:120-162` |
| SQL risk classification | ✅ **Autonomous** impl | `security_policy.rs:1046 classify_sql_risk` + `SqlRiskClass:728` |
| Production detection | ✅ **Autonomous** impl | `security_policy.rs:1146 is_production_database` + `ProductionSignals:1118` + `build_db_signals:1250` |
| Message sanitization | ✅ Implemented | `database.rs:230 sanitize_message` (UTF-8-safe, redacts password/token) |
| Cancel L1 flag | ⚠️ Present but **no consumer** | `database.rs:170-195 QueryCancel` (`#[allow(dead_code)]`) |
| Timeout layering (soft=out-of-band cancel, hard=drop conn) | ✅ Mechanism | `database.rs:199-219 QueryDeadline` |
| SQLite path validation within roots | ✅ Implemented | `database.rs:333 validate_sqlite_path` |
| **MySQL/Postgres fetch channel + TLS** | ❌ **DB_NOT_SUPPORTED** | `database.rs:19` "取数通道与 TLS 未实现，一律返回 DB_NOT_SUPPORTED" |
| **`db_cancel` command** (L3 driver interrupt) | ❌ Name not frozen, no consumer | `database.rs:187 #allow(dead_code)` |
| **Schema discovery** | ❌ Absent | no `mod schema` in repo `src-tauri/src` |
| **Query history persistence** | ❌ Absent | no history module |
| **Export (CSV/XLSX/JSON)** | ❌ Placeholder only | `database.rs:56 DB_MAX_EXPORT_BYTES` (no consumer, A5 pending) |
| **Connection registry** (runtime pool map) | ❌ Absent | only static `DbConnectionConfig` type, no live registry |

**Gap verdict**: The repo's missing backend faces are exactly the data-plane execution layer — fetch channel, schema discovery, cancel L2/L3 registry, history persistence, export. dbx's `query_execution_sql.rs` / `schema.rs` / `query_cancel.rs` / `history.rs` / `csv_export.rs` are the relevant对照 sources.

---

## 3. Upstream source map (exact files/symbols) — A4 eleven dimensions

All paths under `crates/dbx-core/src/` unless noted. Verified by reading files (line numbers from current snapshot).

### 3.1 Connection registry
- `connection.rs:85 pub enum PoolKind` — **23 variants**: `Mysql(deadpool)/Postgres(deadpool_postgres)/Sqlite/…/Agent/ExternalDriver/MessageQueue/Nacos/Consul/Mqtt(feature-gated)` (full list L86-120).
- `connection.rs:122 impl PoolKind::clone_for_metadata` (L125) — clones only handles used by metadata ops so the global pool-map lock is released before any DB I/O.
- `connection.rs:1182 ConnectionRegistry::new(Storage)` + `new_with_plugin_dir` / `new_with_plugin_and_agent_dir_and_app_version` (L1186-1198) — registry ctor holding `Storage` + plugin/agent dirs.
- `connection_secrets.rs:37 FileSecretStore` — ⛔ REJECT main path.
- `database_capabilities.rs` — per-driver capability matrix (read, write, ddl, admin).
- `database_manifest.rs` + `build.rs:16-139` — YAML descriptors (`plugins/connection-types/*.yaml`, 115 files) generate `DatabaseType` enum + `database_manifest.json` at compile time.

### 3.2 Driver capabilities
- `database_capabilities.rs` + `driver_runtime.rs` — capability flags per `DatabaseType`; `driver_runtime.rs` selects the concrete async driver crate.
- Contrast: repo `SupportedDb` = {sqlite, mysql, postgres} (sync). dbx's 23-variant async map is **REIMPLEMENT_FROM_BEHAVIOR** at a much smaller scale.

### 3.3 Schema discovery
- `schema.rs` — async, `clone_metadata_pool` (L36), `dispatch_mysql!` / `extract_pool!` macros (L17-34), `mod kingbase` (L15). Discovers tables/columns via information_schema.
- `schema_diff.rs`, `table_structure_sql.rs` (16 files) — DDL/structure SQL generators.

### 3.4 Query execution
- `query.rs`, `query_execution_sql.rs`, `query_result_sql.rs` — async fetch + paging; `QueryExecutionOptions` threaded through `csv_export.rs:9`.
- Repo gap: synchronous fetch for sqlite/mysql/postgres is entirely unimplemented (returns `DB_NOT_SUPPORTED`).

### 3.5 Cancellation
- `query_cancel.rs` — `RunningQueries` registry (L84): `inner: Arc<Mutex<HashMap<String,RunningTask>>>`, `CancellationToken` (tokio_util), `register`/`register_task` (L98-152), `register_interrupt` (L173, driver-level KILL QUERY delivered post-registration), `cancel`/`cancel_and_wait`/`cancel_connection`/`cancel_client_session`/`cancel_owner_scope`/`cancel_all` (L187-256).
- `DETACHED_REGISTRATION_GRACE_PERIOD = 30min` (L13) bounds leaked resources after timeout. Careful lock discipline: interrupt closure co-located with task under one mutex (L83-92) prevents a leak window.
- Repo: only `QueryCancel` L1 atomic flag exists (`database.rs:170`); L2 (loop check every 64 rows) + L3 (driver interrupt registry) absent.

### 3.6 History
- `history.rs` — `HistoryEntry` (L5: id, connection_id, connection_name, database, sql, executed_at, execution_time_ms, success, error, activity_kind, operation, target, affected_rows, rollback_sql, details_json), `HistorySearchRequest`/`HistorySearchResult`/`HistoryCursor` (cursor pagination L52-72), `MAX_HISTORY=1000` (L88). Persistence layer in same file (sqlite-backed).
- `saved_sql.rs` — saved queries.

### 3.7 Export
- `csv_export.rs` — `push_csv_escaped`/`push_csv_escaped_content` (L33-47, **pure streaming CSV escape**, no external dep), `TableCsvExportOptions` (L16), `TABLE_DATA_EXPORT_PAGE_SIZE=10_000` (L12).
- `xlsx_export.rs`, `text_export.rs`, `database_export.rs`, `query_result_export.rs`, `table_export.rs`, `data_grid_extractors/` — CSV/JSON/XLSX extractors (COPY/ADAPT candidates, pure functions).

### 3.8 SQL analysis / risk
- `sql_risk.rs:18 pub enum SqlRisk`; `classify_sql_risk` (L679), `classify_sql_risk_for_database` (L686), `is_dangerous_sql_for_database` (L703), `mcp_sql_has_forbidden_database_switch` (L732). Pure classification, dialect-parameterized.
- `sql_analysis.rs`, `sql_diagnostics.rs`, `sql_dialect/` (25 files), `sql_parser/` — dialect parsing (partially COPY-able pure logic).

### 3.9 Errors
- `backend_error.rs` — error hierarchy (structural reference; repo already has `DbError` with sanitized message — same philosophy, COPY-only-as-inspiration).

### 3.10 Shutdown
- `update.rs` + src-tauri `app_settings.rs:83 complete_app_close` → `app.exit(0)` whole-reclaim. Repo: single true-exit入口 is a P0借鉴 (A3), not backend data-plane.

### 3.11 Tests
- `crates/dbx-core/tests/` — **51 `.rs` files** incl. `live_*` real-DB integration (live_mysql57, live_postgres_transfer), `api_contract_verification.rs` contract tests, JDBC Java driver tests. Repo currently has **no** DB-layer tests → reverse-borrow the test *discipline* (A11).

---

## 4. Data / control flow (dbx vs repo target)

dbx execution path:
`MCP/Tauri command → query_execution_sql::execute_sql_statement_with_options → PoolKind dispatch (extract_pool!/dispatch_mysql!) → driver async fetch → RunningQueries.register(execution_id) → register_interrupt(driver KILL) → stream rows → history::insert(HistoryEntry) → result DTO`.

Repo target (synchronous, to be built in W19):
`#[tauri::command] db_query → database::execute(config, sql, QueryCancel, QueryDeadline) → match SupportedDb { Sqlite(rusqlite) | Mysql | Postgres } → fetch rows, check DB_CANCEL_CHECK_EVERY_ROWS, enforce DB_MAX_ROWS/DB_MAX_RESULT_BYTES, sanitize → DbQueryResult → (W19) history persist → emit IPC`.

Cancellation: dbx uses `tokio_util::CancellationToken` + registry; repo must ADAPT to sync model — `QueryCancel` L1 flag polled every 64 rows, plus a `RunningQueries`-style `HashMap<exec_id, InterruptFn>` (sync `Arc<Mutex>`) for L3 driver interrupt (mysql `KILL QUERY` / postgres `pg_cancel_backend`).

---

## 5. Persistence format

- dbx history: local sqlite (`history.rs`), schema from `HistoryEntry` serde. Repo: persist into existing workspace sqlite or `workspace/*.json` following `Artifact`/`ImageRef` pattern (`domain.rs`), keeping `db:<conn_id>` keyring separation.
- dbx `state_persistence.rs` (argon2id+AES-256-GCM encrypted file) — ⛔ REJECT main path; repo uses keyring. Only headless/Docker keyring-fallback may借鉴 the encryption shape (explicitly non-main).
- dbx `database_manifest.json` (build-generated) — ADAPT: repo can generate `SupportedDb` from `plugins/db-types/*.yaml` via `build.rs` (P1借鉴, `dbx-借鉴分析.md` §1).

---

## 6. Concurrency / lifecycle

- dbx `PoolKind::clone_for_metadata` releases the global pool lock before I/O (L125) — pattern to copy for repo's registry.
- dbx `RunningQueries` single-mutex co-location of task + interrupt prevents leak window (L83-92) — **ADAPT essential** for repo's cancel registry.
- dbx `DETACHED_REGISTRATION_GRACE_PERIOD=30min` bounds post-timeout KILL handles — repo should adopt a bounded grace.
- Repo current: `QueryCancel` is `Arc<AtomicBool>` (cheap, cross-thread) — good L1 primitive; extend with L2 loop-check (already常量 `DB_CANCEL_CHECK_EVERY_ROWS=64`) and L3 registry.

---

## 7. Security / privacy

- 🔴 `FileSecretStore` / encrypted-file creds — REJECT main path (keyring only).
- 🔴 No-ACL command habit — REJECT.
- ✅ `sanitize_message` exists in repo (`database.rs:230`) — superior to dbx (dbx relied on driver error echo control). Keep.
- ✅ SQL risk + production detection already autonomous in `security_policy.rs` — **do not blindly移植 dbx `sql_risk.rs`/`production_safety.rs`; instead DIFF against repo impl** and adopt only deltas (e.g., dbx's `mcp_sql_has_forbidden_database_switch` @732 is a gap repo lacks; `targets_production_database` @184 multi-statement production targeting is a gap repo's `ProductionSignals` may not cover). A6 will formalize W19 policy cases.
- ✅ `is_production_database` as a **decoupled pure-function layer** (`production_safety.rs:98/184`) — repo already follows this shape (`security_policy.rs` pure funcs). Re-affirm, no copy needed.

---

## 8. Performance / capacity

- Repo limits already match dbx-scale intent: `DB_MAX_ROWS=1000`, `DB_MAX_RESULT_BYTES=4MiB`, `DB_MAX_SQL_BYTES=64KiB`, export `16MiB` (placeholder).
- dbx `TABLE_DATA_EXPORT_PAGE_SIZE=10_000` (csv_export.rs:12) — adopt as streaming page size for repo export.
- dbx bounded history `MAX_HISTORY=1000` — adopt same cap.

---

## 9. Dependencies / licenses

- dbx root `LICENSE` = Apache-2.0. **Transplanting selected files is permitted** under A10 ledger (provenance + NOTICE).
- ⚠️ dbx `[patch.crates-io]` vendors **must NOT** be blindly pulled: `wry` (WebView2 recovery, Windows-specific), `ctor`, `rumqttc`, `dirs-sys`, `pageant`, `tiberius`, plus git forks `tokio-postgres-gaussdb` (ac8ad476), `mysql_async` (zipg fork 4a9e60a), `tiberius` (vendored). These carry native footprints / fork maintenance burden. Repo uses **standard** `rusqlite`/`mysql`/`postgres` (sync) — do NOT switch to gaussdb/mysql_async forks.
- `sql_dialect/`, `sql_parser/`, `csv_export.rs` pure functions: low dependency risk, safe COPY/ADAPT.

---

## 10. Classification (W18-R L1383: COPY/ADAPT/REIMPLEMENT/DEFER/REJECT)

| dbx backend capability | Class | Reason / Repo target |
|---|---|---|
| `csv_export.rs::push_csv_escaped*` (pure) | **COPY** | No external dep; repo export (A5) reuses verbatim |
| `sql_dialect/` / `sql_parser/` pure parse | **COPY/ADAPT** | Vendor into repo `sql_dialect` module, adapt to `SupportedDb` |
| `sql_risk.rs::classify_sql_risk_for_database` / `is_dangerous_sql_for_database` | **ADAPT** | Repo already has `classify_sql_risk`; **DIFF**, adopt gaps (`mcp_sql_has_forbidden_database_switch`) |
| `production_safety.rs::is_production_database` / `targets_production_database` | **ADAPT** | Repo has `is_production_database`; DIFF multi-statement targeting gap |
| `query_cancel.rs::RunningQueries` registry + `register_interrupt` | **ADAPT** | Port to sync `Arc<Mutex>`; repo L1 flag already exists |
| `history.rs` `HistoryEntry` + pagination | **REIMPLEMENT_FROM_BEHAVIOR** | Re-implement against repo workspace store; keep struct shape |
| `schema.rs` information_schema discovery | **REIMPLEMENT_FROM_BEHAVIOR** | Sync rusqlite/mysql/postgres queries, mirror dbx behavior |
| `query_execution_sql.rs` fetch+paging | **REIMPLEMENT_FROM_BEHAVIOR** | Repo's biggest gap; sync fetch with limits |
| `connection.rs::PoolKind` 23-variant | **REIMPLEMENT_FROM_BEHAVIOR** | Repo only 3 sync drivers; build minimal pool abstraction |
| `database_manifest.rs` + `build.rs` YAML→enum | **ADAPT** | Repo `plugins/db-types/*.yaml` → `SupportedDb` |
| `backend_error.rs` hierarchy | **COPY (inspiration)** | Repo `DbError` already covers; no copy |
| `consul/` `mq/` `nacos/` `mqtt/` governance | **DEFER** | #6 first phase excludes governance |
| `mongo_*` `redis_ops` `hbase_ops` non-core drivers | **DEFER** | Out of first-phase scope |
| `jdbc.rs` sidecar | **REJECT** | 🔴 JRE bloat |
| `connection_secrets.rs` / `state_persistence.rs` file creds | **REJECT** (main path) | 🔴 keyring only |
| No-ACL command registration | **REJECT** | 🔴 keep `default-commands.toml` |

---

## 11. Smallest transplantable Apache-2.0 modules (for A10 ledger)

1. `crates/dbx-core/src/csv_export.rs` — pure CSV escape (`push_csv_escaped*`). **COPY**.
2. `crates/dbx-core/src/sql_dialect/` (subset) — pure dialect parsing. **COPY/ADAPT**.
3. `crates/dbx-core/src/sql_risk.rs` pure classification fns — **ADAPT** (diff vs repo).
4. `crates/dbx-core/src/production_safety.rs` pure fns — **ADAPT** (diff vs repo).
5. `crates/dbx-core/src/query_cancel.rs::RunningQueries` — **ADAPT** to sync.
6. `crates/dbx-core/src/history.rs` types + persistence — **REIMPLEMENT**.
7. `crates/dbx-core/src/schema.rs` info-schema queries — **REIMPLEMENT**.
8. `crates/dbx-core/src/query_execution_sql.rs` — **REIMPLEMENT** (sync).

Each requires A10 provenance record (source path, upstream rev `c0a7be12…`, Apache-2.0, NOTICE, modified-file header, destination module, copied tests) before any W19 product code.

---

## 12. Target mapping (dbx → repo)

| dbx module | Repo destination |
|---|---|
| `csv_export.rs` / `xlsx_export.rs` / `text_export.rs` | `src-tauri/src/db_export.rs` (new, A5 consumes) |
| `sql_risk.rs` / `production_safety.rs` | already in `security_policy.rs` (diff-only) |
| `query_cancel.rs` | extend `database.rs` `QueryCancel` + new `cancel_registry` |
| `history.rs` | `src-tauri/src/db_history.rs` (new) |
| `schema.rs` / `table_structure_sql.rs` | `src-tauri/src/db_schema.rs` (new) |
| `query_execution_sql.rs` | `src-tauri/src/database.rs` fetch fns (new) |
| `connection.rs PoolKind` | `src-tauri/src/db_pool.rs` (new, 3-variant sync) |
| `database_manifest.rs` + `build.rs` | repo `build.rs` + `plugins/db-types/*.yaml` |

---

## 13. Test reuse (reverse-borrow discipline, A11)

- dbx `tests/` 51 files prove: live-DB integration (`live_mysql57`/`live_postgres_transfer`), contract tests (`api_contract_verification.rs`). Repo has **zero** DB-layer tests.
- W19 should: (a) add synchronous fetch unit tests with an in-memory sqlite + a mock mysql/postgres; (b) add contract tests for `db_query`/`db_cancel` IPC; (c) add `live_*` gated integration tests requiring a real DB (cannot run under W18-R's no-DB-connection boundary).
- Reuse dbx's `HistoryEntry` round-trip serde tests shape for repo history.

---

## 14. Unresolved questions (for A0 / predecessor lanes)

1. **Async vs sync**: repo contract (M4-1.a) forbids tokio runtime. dbx is fully async. All移植 must be **synchronous ports** — confirm W19 keeps sync (else a runtime-model decision is needed, affecting every module above).
2. **`db_cancel` command name**: frozen? (`database.rs:187` says M4-1.c §3 unfrozen). Needed before L3 registry is reachable.
3. **MySQL/Postgres TLS**: repo returns `DB_NOT_SUPPORTED` for fetch+TLS. W19 must decide TLS lib (native-tls/rustls) within sync `mysql`/`postgres` crates.
4. **History store**: workspace sqlite vs `workspace/*.json`? Must not regress `Artifact`/`ImageRef` load path (`domain.rs:17-20` serde-default caveat).
5. **Diff reconciliation**: exact deltas between repo `security_policy.rs` and dbx `sql_risk.rs`/`production_safety.rs` need a line-by-line A6/A10 pass before any code (this report flags, does not implement).
6. **M4-1.c checkpoint state**: `database.rs:32-34` notes A0 marked `M4-1.c-20260905-2255.md` as `STOPPED_EMPTY_ARTIFACT`; constants are used from body text — A0 should re-adjudicate that checkpoint's status.

---

## 15. Lane output (W18-R L1416)

```
LANE=A4
STATUS=PASS_WITH_DEBT
BASE=78d2cfb8e90d323d35df920e9807d32189f867cd
HEAD=<set after commit of this report + checkpoint>
REFERENCE_EVIDENCE=/home/ainfinit/Documents/极智简单/V3/research/dbx-src (Apache-2.0, Cargo.lock SHA c0a7be12…); /home/ainfinit/Documents/极智简单/V3/dbx-study/*.md (V5); repo src-tauri/src/{database,security_policy,domain,keyring_store,bridge}.rs
FILES=logs/research/M5-W18/A4-dbx-backend-architecture-map.md, logs/research/M5-W18/A4-checkpoint.md
SOURCE_MAP=see §3 (connection.rs:85/1182/1186-1198; connection_secrets.rs:37; database_capabilities.rs; database_manifest.rs; build.rs:16-139; schema.rs; query_execution_sql.rs; query_cancel.rs:13/84/98-185/187-256; history.rs:5/52/88; csv_export.rs:12/16/33-47; sql_risk.rs:18/679/686/703/732; production_safety.rs:98/110/184; update.rs; app_settings.rs:83)
CLASSIFICATION=COPY=3 (csv_export pure, sql_dialect subset, backend_error inspiration); ADAPT=5 (sql_risk, production_safety, query_cancel, database_manifest, csv/xlsx pipeline); REIMPLEMENT_FROM_BEHAVIOR=4 (history, schema, query_execution, PoolKind-minimal); DEFER=2 (governance, non-core drivers); REJECT=4 (jdbc, FileSecretStore, no-ACL, governance)
VERIFY=read-only static mapping only (W18-R L1381 forbids DB connection); no live tests run. dbx tests/ inspected, not executed.
CHECKPOINT=logs/research/M5-W18/A4-checkpoint.md
MERGE_NOTES=Depends on A1 baseline (repo DB module inventory), A10 (provenance ledger + license/NOTICE), A6 (SQL-risk/production diff → W19 policy). Conflicts: async-vs-sync runtime model (repo forbids tokio), credential keyring vs FileSecretStore, ACL whitelist vs dbx no-ACL. Security debt: repo SQL-risk/production impl must be diffed against dbx before adoption.
NEXT=W19 slices: (1) sync fetch channel for sqlite/mysql/postgres + TLS decision; (2) cancel registry L2/L3; (3) schema discovery; (4) history persistence; (5) export pipeline. Each gated by A10 provenance + A6 policy cases + this repo's sync-runtime contract.
```
