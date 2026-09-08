# A10 — Source-Transplant Ledger (dbx + zvec-grep + Obsidian behavior)

> Lane: A10 (Licensing, dependency BOM, source-transplant plan)
> Wave: M5-W18-R Research and Replication Blueprint
> Revision: **R2** (evidence-closure; rebased `origin/master` `d6127c4`)
> Date: 2026-09-08
> BASE: `78d2cfb8e90d323d35df920e9807d32189f867cd` → REBASED onto `origin/master` (`d6127c4`)
> Companion docs: `A10-dependency-bom.md`, `A10-notice-and-review-gate.md`

Each candidate is classified **COPY / ADAPT / REIMPLEMENT_FROM_BEHAVIOR / DEFER / REJECT** with the required fields: source path/symbol, upstream revision, license, notices, dependency native footprint, destination module, modification plan, copied tests, reject reasons.

---

## 0. R2 corrections — retract / replace R1 statements named in `A0-M5-W18-R1-audit-20260908.md`

R1 A10 verdict was `REWORK` ("useful ledger template, but finalized before source lanes, contains license/transplant contradictions, and has no function-level closure for COPY candidates"). The following R1 claims are **explicitly corrected** (history preserved, not silently rewritten):

| # | R1 claim (retracted) | R2 correction (evidence) | Evidence type |
|---|---|---|---|
| C1 | BOM §2.1 recommended `tokio-postgres` / `deadpool-postgres` / `mysql_async` as **COPY** drivers | Product `src-tauri/Cargo.toml` already depends on **sync** `mysql 28.0.2` / `postgres 0.19.14` / `rusqlite 0.40.2 bundled`. M4-A2 decision F-1 = "不引 tokio 直接依赖" (std::thread+Condvar). A4 `D-A4-2` confirms "all移植 must be sync; async-vs-sync must be confirmed by A0". → **ADAPT to existing sync drivers; DO NOT add async drivers** (runtime conflict). | `CURRENT_PRODUCT` + `INFERENCE`(M4-A2 F-1) |
| C2 | Ledger §0 proposed "new `src-tauri/crates/*` workspace member" as transplant surface | Product `M5-1.a` explicitly chose "**同 package 双 target**" (lib+bin in one package), not a workspace member, to minimize dependency-graph change. → Transplants land in existing `src-tauri/src/` modules (`database.rs`, `security_policy.rs`, new `db_guard.rs`/`export.rs`/`search.rs`), consistent with `M5-1.a`. | `CURRENT_PRODUCT` |
| C3 | D3 `production_safety.rs` = **COPY** | Product already implements equivalent `security_policy.rs::is_production_database`(:1146) + `ProductionSignals`(:1118) + `classify_sql_risk`(:1046) + `SqlRiskClass`(:728). dbx is a cross-check reference (A4 §2). → **ADAPT (diff-only) / NO COPY**; reconcile the diff with A6 before any adoption. | `CURRENT_PRODUCT` (A4 map §2/§10) |
| C4 | D4 `sql_risk.rs` = **COPY** | Same as C3 — product already has `classify_sql_risk`. dbx version needs `sqlparser` (NOT in product) + `models/connection.rs` (4,490 LOC). → **ADAPT (optional AST upgrade w/ new `sqlparser` dep)** / NO COPY. | `CURRENT_PRODUCT` |
| C5 | D16 `app_settings.rs::complete_app_close` = **COPY** | Product owns its own exit/shutdown path (`shutdown.rs`). → **REJECT-as-COPY** (adopt graded-close *concept* only). | `CURRENT_PRODUCT` |
| C6 | D11 `csv_export.rs` = **COPY (curated)** | Function-level split (see D11a/D11b). Only the pure `format_csv` cluster is self-contained; the file/export pipeline pulls 4 internal dbx deps (≈11k LOC). → **D11a COPY / D11b REJECT**. | `REFERENCE_SOURCE` |
| C7 | D10 `sqlparser` = **COPY** | External crate `sqlparser 0.62` (Apache-2.0) is **not** in product `Cargo.lock` (293 crates). It is a viable *new* dep only if A6 upgrades classification to AST-based. → **ADAPT (new dep, optional)**, not a blind COPY. | `CURRENT_PRODUCT` (Cargo.lock) |
| C8 | "product is Apache-2.0" ambiguous phrasing risk | Verified: product root `LICENSE` = **木兰宽松许可证第2版 (MulanPSL-2)**, 23 occurrences of "Mulan/木兰"; `src-tauri/Cargo.toml` has **no `license` field** (gap → add `license = "MulanPSL-2.0"` at W19). Inbound dbx/zvec-grep = Apache-2.0. → dual-attribution required. (A0 audit #7 resolves A8's false "Apache-2.0 product" claim; A10 states product is MulanPSL-2, not Apache-2.0.) | `CURRENT_PRODUCT` |

---

## 0b. Current-product destination surface (where transplanted code would land — R2 corrected)

| Existing module | Role | Reuse for |
|---|---|---|
| `src-tauri/src/graph.rs` | bounded graph store, `validate_*`, `bounded_neighbors`/`bounded_subgraph` | Obsidian local-graph semantics (REIMPLEMENT) |
| `src-tauri/src/database.rs` | single SQLite + sync `mysql`/`postgres` drivers (1,273 LOC, 23 `#[test]`) | dbx connection/schema/query concepts (ADAPT, sync) |
| `src-tauri/src/security_policy.rs` | `classify_sql_risk`(:1046), `SqlRiskClass`(:728), `ProductionSignals`(:1118), `is_production_database`(:1146) (2,357 LOC, 80 `#[test]`) | dbx D3/D4 diff-only reconciliation (A6) |
| `src-tauri/src/domain.rs` | `GraphNodeKind`/`GraphEdgeKind`, 7 capacity constants | structured node/edge types |
| `src-tauri/src/bridge.rs` / `src/bridge.ts` / `src/types.ts` | IPC + typed DTOs + ACL | any new command surface |
| `src-tauri/src/mcp_server.rs` / `mcp.rs` | existing Rust MCP (W5-W10 skeleton) | dbx MCP *concept* (reuse own, not npm) |
| `src-tauri/src/shutdown.rs` | exit/shutdown | dbx D16 concept (already owned) |
| **(new) `src-tauri/src/db_guard.rs`** | proposed | dbx D3/D4 diff + D5a caps (ADAPT) |
| **(new) `src-tauri/src/export.rs`** | proposed | dbx D11a `format_csv` cluster (COPY) |
| **(new) `src-tauri/src/search.rs`** | proposed | zvec-grep retrieval routes Z1–Z7,Z12,Z13 (REIMPLEMENT) |

No dedicated search/index module exists today → zvec-grep maps to a **new** `search.rs` (A7 confirmed product gap).

---

## 1. dbx ledger (Apache-2.0; snapshot `Cargo.lock` SHA verified `c0a7be12…`)

Upstream: `/home/ainfinit/Documents/极智简单/V3/research/dbx-src` (Apache-2.0; `Cargo.lock` SHA matches dispatch pin). Crate licenses: `crates/*/Cargo.toml` all `license = "Apache-2.0"`; **no root `NOTICE`** shipped.

**Function-level closure rule applied:** a unit is `COPY` only if it is self-contained (deps = std + already-present crates + product equivalents). Otherwise split or reclassified.

| # | Source path / symbol (file:line) | Transitive local deps | External crates | Destination | Class | Verdict (ACCEPT/REJECT) |
|---|---|---|---|---|---|---|
| D1 | `build.rs` + `plugins/connection-types/*.yaml` (115) → `DatabaseType` | YAML only | `serde_yaml_ng` (not present) | `src-tauri/build.rs` + `plugins/db-types/*.yaml` | **ADAPT** | ACCEPT (ADAPT) |
| D2 | `connection.rs:85` `PoolKind` (23 variants) | `models/connection.rs` (4,490 LOC) | — | `database.rs` sync pool | **ADAPT** | ACCEPT — keep `Sqlite/MySql/Postgres` over existing sync drivers; drop JDBC/governance (C1) |
| D3 | `production_safety.rs:98/110/184` `is_production_database`/`targets_production_database` (822 LOC, 8 tests) | `models/connection.rs` (4,490 LOC) + `regex` (present 1.13.1) | `regex` (✅ in lock) | `db_guard.rs` | **ADAPT (diff-only)** | **REJECT-as-COPY** — product `security_policy.rs:1146` already implements; adopt only the diff (A6) |
| D4 | `sql_risk.rs:679/686/703` `classify_sql_risk`/`classify_sql_risk_for_database`/`is_dangerous_sql_for_database` (1,392 LOC, 22 tests) | `models/connection.rs` | `sqlparser` (❌ not in lock) | `db_guard.rs` | **ADAPT (optional AST)** | **REJECT-as-COPY** — product `security_policy.rs:1046` already implements; AST upgrade needs new `sqlparser` dep (C4/C7) |
| D5a | `agent_tools.rs:16-34` caps: `LIST_TABLES_LIMIT=200`, `EXECUTE_QUERY_LIMIT=50`, `SAMPLE_DATA_LIMIT=20`, `BROWSE_COLLECTION_LIMIT=20`, `MAX_ALLOWED_ROWS=100`, `DEFAULT_QUERY_CELL_CHAR_LIMIT=200`, `MAX_QUERY_CELL_CHAR_LIMIT=4_000`, `MAX_QUERY_CELL_CHAR_OFFSET=1_000_000` | none (pure consts) | none | `db_guard.rs`/`bridge.rs` | **COPY** | **ACCEPT** (self-contained pure consts, no crate dep) |
| D5b | `agent_tools.rs:78/90/158/195/263/274/699/846` `AgentSqlPermissions` + permission fns | dbx `ToolDefinition` (mcp) | `serde_json` (✅) | `mcp_server.rs` | **ADAPT** | **ACCEPT** (reuse own Rust `rmcp`; do not import dbx MCP) |
| D6 | dbx live cred path = SQLite plaintext `connection_secrets` table (`storage.rs:326-329` `secret TEXT`, `save_password=true` writes `password` plaintext `:2374-2380`); `FileSecretStore` (`connection_secrets.rs:37`) is **dead code** (`FileSecretStore::new` has 0 callers, A6 §5) | — | `argon2`+`aes-gcm` (not in lock) | (none — do not adopt) | **REJECT** | **REJECT** — dbx stores creds in plaintext SQLite table; product MUST keep `keyring_store` (bridge.rs:6024/6065/6109, key `db:<conn_id>`). A6 R2B corrects A4 map + this ledger: the real reject target is the plaintext table, not dead `FileSecretStore`. |
| D7 | `history.rs` | `models/connection.rs` | `serde_json` | `database.rs` | **ADAPT** | ACCEPT |
| D8 | `query_execution_sql.rs`(1,674)/`query_result_sql.rs`/`query_cancel.rs` | `models/connection.rs` | sync drivers (✅) | `database.rs` | **ADAPT** | ACCEPT — but fetch channel not yet implemented (D-A4-2 gating) |
| D9 | `schema.rs` (477 LOC in `database_capabilities`) | heavy | — | `database.rs` | **REIMPLEMENT (thin)** | ACCEPT — borrow capability-enum idea |
| D10 | `sql_parser/`+`sql_dialect/` (25 files) | many | `sqlparser` (❌) | `db_guard.rs` | **ADAPT (new dep)** | ACCEPT-conditional — only if A6 upgrades to AST (C7) |
| D11a | `csv_export.rs:33/75/83/114/127/192/207/222/226` pure `format_csv` cluster (`push_csv_escaped_content`, `push_csv_value`, `push_tsv_escaped`, `ROWS_CAPACITY_ESTIMATE_MAX`, `format_csv_value`, `push_query_result_csv_rows`, `format_query_result_csv_rows`, `format_csv`, `format_query_result_csv`) (438 LOC, 10 tests) | none | `serde_json` (✅) + `std::fmt` | `export.rs` | **COPY** | **ACCEPT** — self-contained; deps = `serde_json::Value` + std only |
| D11b | `csv_export.rs:230/242` file-writing path + `write_csv_*` | `crate::connection::AppState`, `crate::models::connection::DatabaseType`, `crate::query::execute_sql_statement_with_options`, `crate::sql_dialect::build_table_data_select_sql` (query_execution_sql 1,674 + sql_dialect 9,206 LOC) | — | `export.rs` | **REJECT** | **REJECT-as-COPY** — pulls 4 internal dbx deps (~11k LOC); ADAPT pipeline only after fetch channel (D-A4-2) |
| D12 | `database_manifest.rs`/`database_search_sql.rs` | `models/connection.rs` | — | `database.rs` | **ADAPT** | ACCEPT |
| D13 | `agent_kv.rs` (93 LOC in AI memory KV) | heavy | — | `agent_memory.rs` | **REIMPLEMENT (concept)** | ACCEPT — dbx `agent_kv` is AI memory, not user成果库 |
| D14 | `crates/dbx-mcp/src/server.rs:277-787` 9 tools | dbx mcp | `rmcp` (we have own) | `mcp_server.rs` | **ADAPT concept** | ACCEPT — reuse own Rust `rmcp` |
| D15 | `webview2_recovery.rs` (47 LOC) | Win-specific | — | browser-tabs stability | **REIMPLEMENT (concept)** | ACCEPT — pattern only (Linux differs) |
| D16 | `src-tauri/src/commands/app_settings.rs:83` `complete_app_close` | dbx `CloseBehaviorState`/`AppState` | tauri (✅) | `shutdown.rs` | **REJECT-as-COPY** | **REJECT** — product owns exit (C5) |
| D17 | `ssh_prompt.rs` mpsc+oneshot handshake | — | `tokio` (transitive ✅) | `database.rs` SSH tunnel | **ADAPT** | ACCEPT |
| D18 | `deep_link.rs:4-6` `dbx://` scheme | — | `url` (✅) | `#8 http/https handler` | **REIMPLEMENT** | ACCEPT — swap scheme |
| D19 | `update.rs`+`updater` | — | — | #8 packaging | **DEFER** | DEFER — out of W18-R |
| D20 | `consul/``nacos/``mq/``mqtt/` | — | — | — | **REJECT** | REJECT — governance out of scope |
| D21 | `vendor/wry` (Tauri fork) | — | — | — | **REJECT** | REJECT — second Tauri fork conflict |
| D22 | `ai_cli_agent.rs` | — | — | #7 A2A (future) | **DEFER** | DEFER |
| D23 | `agent_loop.rs`/`agent_service.rs`/`ai.rs` | — | — | — | **REJECT** | REJECT — coarse, product-specific |
| D24 | `transfer.rs` (474 LOC) | — | — | — | **REJECT** | REJECT — heavy, out of first wave |

**dbx classification tally (R2 corrected):** COPY 2 (D5a, D11a) · ADAPT 11 (D1,D2,D3,D4,D5b,D7,D8,D9,D10,D12,D14,D17 — note D3/D4 are ADAPT diff-only) · REIMPLEMENT 4 (D13,D15,D18 + D9 thin) · DEFER 2 (D19,D22) · REJECT 7 (D6-primary,D11b,D16,D20,D21,D23,D24).

---

## 2. zvec-grep ledger (Apache-2.0; upstream `52653951…`, npm `@zvec/zvec-grep` `0.2.1`; `package.json` `engines.node>=22`)

Upstream: `/home/ainfinit/Documents/极智简单/V3/research/zvec-grep-src` (Apache-2.0; root `LICENSE` = Apache-2.0; **no NOTICE**). Deps: `@huggingface/transformers`@3.8.1, `@huggingface/tokenizers`@0.1.3, `@vscode/ripgrep`@^1.18, `@zvec/zvec`@^0.7.0 (ONNX/WASM), `tree-sitter-wasms`@^0.1.13, `web-tree-sitter`@^0.20.8, `@modelcontextprotocol/*`@2.0.0, `jsonc-parser`, `zod`, optional `node-llama-cpp`. **All Node/model-bound → cannot be copied into a Rust/Tauri binary.**

| # | Source path / symbol | Transitive deps | Destination (proposed) | Class | Verdict |
|---|---|---|---|---|---|
| Z1 | `src/engine/extraction/code/CodeExtractor` (17 files) | tree-sitter wasm | `search.rs` | **REIMPLEMENT** | ACCEPT — port in Rust w/ `tree-sitter-*` crates (A7: REIMPLEMENT=7) |
| Z2 | `src/engine/extraction/markdown/MarkdownExtractor` | none | `search.rs` | **REIMPLEMENT** | ACCEPT |
| Z3 | `src/engine/extraction/text/TextExtractor` | none | `search.rs` | **REIMPLEMENT** | ACCEPT |
| Z4 | `src/engine/storage/zvec.ts`+`layout.ts` (`.zvec-grep/`) | `@zvec/zvec` | `search.rs` index store | **REIMPLEMENT** | ACCEPT — Rust layout (SQLite/redb) |
| Z5 | `src/engine/service/lexical.ts` (managed ripgrep) | `@vscode/ripgrep` | `search.rs` exact route | **ADAPT** | ACCEPT — shell system `rg` (present on Linux) |
| Z6 | `src/engine/service/zvec-grep.ts`+`pipeline/search/*` (RRF) | `@zvec/zvec` | `search.rs` hybrid route | **REIMPLEMENT** | ACCEPT — port RRF fusion |
| Z7 | `src/engine/pipeline/indexing/*` (incremental + watcher) | — | `search.rs` indexer | **REIMPLEMENT** | ACCEPT — watcher-free first slice (A7) |
| Z8 | `@zvec/zvec` vector core + HF embedding | model weights + runtime | (external sidecar) | **ADAPT (whole-sidecar, not per-function COPY)** | **REJECT-as-COPY → RECLASSIFIED ADAPT (R2B, per A7 evidence closure E1–E7 + A8 §3/§4)**. Official `zvec-rust` 0.7.0 crate (Apache-2.0, depends `zvec-rust-sys` 0.7.0 linking `libzvec_c_api`) **exists** (A7 E4/E5); C++ core `alibaba/zvec` (Apache-2.0, exposes C API, per-platform prebuilt SDKs) (A7 E6). **Product adoption path = npm N-API sidecar** `@zvec/zvec` 0.7.0 + `@zvec/bindings-linux-x64` 0.7.0 (`zvec_node_binding.node`, ELF N-API, 41.7 MB) (A7 E1–E3). Both the official crate and the npm sidecar are **Apache-2.0 → no license conflict**; HOLD-(a) is **lifted**. HOLD-(b) remains: A9 egress policy (remote-embedding DISABLED by default, apiKey→OS-keychain) must be ported before W19 (see §8). Native footprint: `.node` 41.7 MB + engine RSS ~294 MB (bundle impact → A3/A10 eval). Matches A8 `COPY=0`. |
| Z9 | `src/authorization/*` (8 files) | TS/MCP | `security_policy.rs`+ACL | **REIMPLEMENT concept** | ACCEPT — port "explicit one-time/workspace grant before egress" (A9: COPY=5 includes this gate) |
| Z10 | `src/daemon/*` (16 files) | loopback server | (sidecar) | **DEFER/REJECT-in-product** | DEFER — daemon forbidden this wave (A7 DEFER=1) |
| Z11 | `src/mcp/*` (10 files) | npm mcp | `mcp_server.rs` (Rust) | **ADAPT concept** | ACCEPT — reuse own Rust `rmcp` (A9 ADAPT=2) |
| Z12 | `src/engine/file-size-policy.ts` consts: `DEFAULT_MAX_CODE_FILE_SIZE_BYTES=1*1024*1024`, `DEFAULT_MAX_TEXT=256*1024*1024`, `DEFAULT_MAX_DATA=16*1024*1024`, `DEFAULT_MAX_IMAGE=10*1024*1024`; `resolveMaxFileSizeBytes` + `FileKind` enum; `src/engine/config.ts` `GLOBAL_CONFIG_DIRECTORY_MODE=0o700`, `GLOBAL_CONFIG_FILE_MODE=0o600` | none (pure TS consts + trivial enum) | `search.rs` policy | **COPY** | **ACCEPT** — lift verbatim as Rust `const`s + reimplement `FileKind` (trivial enum) |
| Z13 | `src/engine/service/structure-enrichment.ts` | — | `search.rs` | **REIMPLEMENT** | ACCEPT |

**zvec-grep classification tally (R2, refined R2B):** COPY 1 (Z12) · ADAPT 4 (Z5, Z8-sidecar-ADAPT, Z9, Z11) · REIMPLEMENT 7 (Z1,Z2,Z3,Z4,Z6,Z7,Z13) · DEFER 1 (Z10) · REJECT-copy 0 (Z8 reclassified ADAPT).

**zvec-grep headline verdict:** **REJECT direct copy** (Node/TS + model/runtime bound). Transplantable *value* = algorithm layer (route selection, RRF fusion, type-aware chunking, incremental reconciliation, freshness, compact output, authorization-before-egress) → **REIMPLEMENT in Rust** (Z1–Z7,Z9,Z12,Z13). Embeddings (Z8) = **ADAPT-sidecar** (npm N-API `@zvec/zvec` 0.7.0, Apache-2.0), no per-function COPY (A8 `COPY=0`); HOLD-(a) lifted by A7 R2B evidence, HOLD-(b) egress policy still gates W19.

---

## 3. Obsidian ledger (behavior-only — NO code donor)

Upstream: `/home/ainfinit/Documents/Knowledge-Base/secondBrain/.obsidian` config (read `graph.json`, `app.json`, `core-plugins.json`). **Not open-source; reproduce only observable behavior + documented/local formats. No code/icons/branding.** License: N/A (behavior reference). Native footprint: none.

| # | Observed behavior | Destination | Class |
|---|---|---|---|
| O1 | Local graph: `search` filter, `showOrphans`, `colorGroups`, force params (`graph.json`) | `graph.rs` + graph UI | **REIMPLEMENT_FROM_BEHAVIOR** |
| O2 | Backlinks / outgoing-links (`core-plugins.json`) | graph DTOs | **REIMPLEMENT** |
| O3 | Wikilinks `[[page]]`, tags `#tag`, frontmatter, aliases, heading/block refs | (future) notes module | **REIMPLEMENT** |
| O4 | Orphan detection (`showOrphans:true`) | `graph.rs` query | **REIMPLEMENT** |
| O5 | Tag pane / canvas / outline / page-preview | UI layer | **REIMPLEMENT** |
| O6 | Graph color groups / depth / selection / keyboard nav | graph UI state | **REIMPLEMENT** |

**Obsidian tally:** REIMPLEMENT 6 (O1–O6). No COPY/ADAPT (no source). **Pending A2/A3** for detailed Vue blueprint + vault-format fixtures (A2/A3 R2 in progress).

---

## 4. Consolidated classification counts (R2 corrected)

| Source | COPY | ADAPT | REIMPLEMENT | DEFER | REJECT |
|---|---|---|---|---|---|
| dbx | 2 | 11 | 4 | 2 | 7 |
| zvec-grep | 1 | 4 | 7 | 1 | 1* |
| Obsidian | 0 | 0 | 6 | 0 | 0 |
| **Total** | **3** | **15** | **17** | **3** | **8** |

\* zvec-grep Z8 = REJECT-copy (ADAPT-sidecar as future, HOLD). Counted REJECT in copy dimension.

**Genuinely self-contained COPY-eligible units (the only true COPY targets):** D5a (caps consts), D11a (`format_csv` cluster), Z12 (type-aware size consts). Everything else is ADAPT/REIMPLEMENT/DEFER/REJECT. This resolves A0 audit "no function-level closure for COPY candidates".

---

## 5. Per-unit ACCEPT / REJECT summary (R2 final gate)

- **ACCEPT (self-contained COPY):** D5a, D11a, Z12.
- **ACCEPT (ADAPT):** D1,D2,D3(diff),D4(opt AST),D5b,D7,D8,D9,D10,D12,D14,D17; Z5,Z9,Z11; O1–O6.
- **ACCEPT (REIMPLEMENT):** D13,D15,D18; Z1–Z4,Z6,Z7,Z13.
- **DEFER:** D19,D22; Z10.
- **REJECT (no transplant):** D6-primary, D11b, D16, D20, D21, D23, D24, Z8-copy.

No blanket "ready for W19" — see §6 hard stops.

---

## 6. Cross-cutting transplant constraints (must hold for every copied unit)

1. **License retention**: copied files keep Apache-2.0 at file level; ship `LICENSE-APACHE-DBX`, `LICENSE-APACHE-ZVEC-GREP`; product stays MulanPSL-2 (add `license = "MulanPSL-2.0"` to `src-tauri/Cargo.toml` at W19 — see BOM §1 gap).
2. **Modification notice**: every modified file carries `// MODIFIED FROM <upstream> <rev>` header.
3. **Native budget**: exclude `openssl=vendored`, `aws-lc-rs`, `sqlite-sqlcipher`, second `wry`/`tauri` fork; keep curated pure-Rust subset; **reuse existing sync `mysql`/`postgres`**, do NOT add `tokio-postgres`/`mysql_async` (C1).
4. **No behavioral conflict**: no command-without-ACL, no FileSecretStore primary, no JDBC sidecar, no single-window取代 subwebview, no MCP npm分包.
5. **Gate compliance**: every new command lands with source-check + handler + ACL + typed bridge/types + policy tests.
6. **Capacity/privacy**: obey existing capacity constants, privacy double-scan, release-origin gate.

---

## 7. Pending upstream inputs (R2 finalization blockers)

- **A2 / A3** (Obsidian Vue blueprint + vault fixtures) — refine O1–O6 destinations. **Not yet landed in R2.**
- **A4** (dbx backend map) — consumed (R1); reconciled D3/D4/D10/D11/D16 (C3–C6). A4 R2 in progress.
- **A7** (zvec-grep ingestion/index) — **RESOLVED (R2B)**: official `zvec-rust` 0.7.0 crate (Apache-2.0, `zvec-rust-sys` linking `libzvec_c_api`) exists; C++ core `alibaba/zvec` (Apache-2.0, C API + per-platform SDKs). Product path = npm N-API sidecar `@zvec/zvec` 0.7.0 (Apache-2.0). Z8 HOLD-(a) lifted → ADAPT-sidecar. See `A10-R2B-review-findings.md` F5.
- **A8** (zvec-grep retrieval benchmark) — Z5/Z6 fusion thresholds. R1 consumed; A8 R2 in progress.
- **A9** (zvec-grep trust boundary) — Z8 sidecar egress policy + apiKey→keychain (B3). R1 consumed; A9 R2 in progress.

Per dispatch R2 sequencing, A10 may **finalize** only after A2/A4/A7/A8/A9 R2 commits exist. This R2 revision is therefore marked **PASS_WITH_DEBT** (HOLD finalization) — all function-level closures and license analysis are complete for the units A10 can verify from `CURRENT_PRODUCT` + `REFERENCE_SOURCE`; the only open items are peer R2 confirmations (Z8 native-binding proof, O1–O6 Vue blueprint) which would not change the first implementation slice.

---

## 9. R2B reconciliation (2026-09-08, A10 independent review)

R2B reframes A10 as **independent review** of peer lanes. Consumed fixed SHAs: A1 `526e2ef`, A2 `bcdfc3b`, A3 `877b4f5`, A4 `b737e5d`, A5 `a42b915`, A6 `38faa2d`, A7 `07fda2f`, A8 `f4a4f3b`, A9 `28a934a`, A11 `c3d4712`. Detailed findings in `A10-R2B-review-findings.md`.

- **Z8 HOLD-(a) lifted** (A7 R2B evidence closure): official `zvec-rust` crate + npm N-API sidecar both Apache-2.0; no license conflict; classify ADAPT-sidecar, not per-function COPY (matches A8 `COPY=0`).
- **Product license re-confirmed MulanPSL-2.0** (R2B: `LICENSE` head + A8 correction of A0 #7). Inbound Apache-2.0 (dbx `c0a7be12`, zvec-grep `5265395`) compatible; NOTICE obligation retained.
- **dbx pin `c0a7be12` already authoritative** in this ledger (matches A4's `Cargo.lock` SHA) — no correction needed vs R1 audit fragment.
- **Correction surfaced to peers (R2B F2):** A4 map claims "repo currently has zero DB-layer tests" (§1.2/§2.x) — **false**; product `database.rs` contains **23** independent `#[test]` (verified `grep -cE '^\s*#\[test\]' database.rs` = 23; R2B card also states 23, correcting A0 R1 audit's 22). Gap is live-DB integration tests only, not all tests.
- **Open items for A0 (R2B F7/F10/F12):** zvec metric semantics (A8 labels precision as recall@10), dispatch line 1358 A3↔A8 attribution typo, A4 must freeze `db_cancel(conn_id, query_id)` before A5 W19. See review-findings §Cross-cutting.
- **R2B F13 (DbValue dual-enum, escalate):** product ships two `DbValue` enums — wire `database.rs` serializes `i64/f64/binary`, while `domain.rs` + TS mirror use `int/float/blob_len`; B8-1 fixed only `domain.rs`. Numeric/binary cells render as raw JSON (`decodeDbValue` fallback). Unify on one enum (recommend `int/float/blob_len`) and point `DbQueryResult.rows` at it; add serialization round-trip test. **Product fix (W19) by A4+A5 — A10 does not edit product code.**
- **R2B F14 (ledger D6 corrected):** dbx live cred path = SQLite plaintext `connection_secrets` table (`storage.rs`); `FileSecretStore` is dead code. D6 reject rationale updated; product `keyring_store` path confirmed correct. **G4 addendum:** product keyring *usage* correct but cleanup imperfect (`let _ =` swallows delete error, no orphan `db:<conn_id>` key cleanup) — A6 G4 (ADAPT) to fix; separate from the dbx-reject rationale.
- **R2B F16 (A2↔A3 low-risk):** A3 reconciled against A2 R1 draft; A2 R2 (`bcdfc3b`) now published and consistent on orphan/unresolved/backlink/alias. A3 should re-sync colorGroups/search-DSL schema vs A2 R2 and retire debt D-A3-1. Return to A3/A11.
- **R2B F17 (db_cancel contract, escalates F12):** product `QueryCancel::cancel` is `#[allow(dead_code)]` (no trigger); MySQL/Postgres fetch `NotSupported` (no server-side KILL). A4's `db_cancel(conn_id, query_id)` freeze must absorb A6 §4.2 race contract: `RunningQueries` registry (exec_id↔token+interrupt, single-lock), driver `register_interrupt` + server-side KILL, `DETACHED_REGISTRATION_GRACE_PERIOD=30min` orphan reclamation; `DB_CANCEL_SERVER_SIDE` = W19 fail-closed gate. A5 consumes.

---

## 8. Hard stops before any W19 transplant

1. A0 must record `W19=OPEN` in writing (dispatch §exit gate).
2. Z8 (embeddings) must not enter product as COPY; if sidecar, A7 must prove Rust/NAPI binding OR A0 approves Node sidecar, and A9 egress policy (remote-embedding DISABLED by default, apiKey→OS-keychain, integrity check) must be ported first.
3. `src-tauri/Cargo.toml` must declare `license = "MulanPSL-2.0"` before shipping any attribution.
4. `sqlparser` (D4/D10 AST upgrade) only if A6 decides AST-based classification; otherwise keep product's regex `classify_sql_risk`.
