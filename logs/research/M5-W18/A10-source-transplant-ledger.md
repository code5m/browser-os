# A10 — Source-Transplant Ledger (dbx + zvec-grep + Obsidian behavior)

> Lane: A10 (Licensing, dependency BOM, source-transplant plan)
> Wave: M5-W18-R Research and Replication Blueprint
> Date: 2026-09-08
> BASE: `78d2cfb8e90d323d35df920e9807d32189f867cd` (worktree `m5-w18-a10`)
> Companion docs: `A10-dependency-bom.md`, `A10-notice-and-review-gate.md`

Each candidate is classified **COPY / ADAPT / REIMPLEMENT_FROM_BEHAVIOR / DEFER / REJECT** with the required fields: source path/symbol, upstream revision, license, notices, dependency native footprint, destination module, modification plan, copied tests, reject reasons.

> **Upstream analysis dependency note (per dispatch §Dependencies):** A10 may proceed independently when A2/A4/A7–A9 reports are unfinished. Those lanes' detailed maps are **pending**; this ledger is built from direct source inspection + the existing `dbx-study/*` V5 analysis (cited as evidence). Entries marked `NEEDS-Ax` must be refined when the corresponding lane report lands. The ledger is already sufficient for A0 to draft implementation cards.

---

## 0. Current-product destination surface (where transplanted code would land)

| Existing module | Role | Reuse for |
|---|---|---|
| `src-tauri/src/graph.rs` | bounded graph store, `validate_*`, `bounded_neighbors`/`bounded_subgraph`, `to_json`/`from_json` | Obsidian local-graph semantics (REIMPLEMENT) |
| `src-tauri/src/database.rs` | single SQLite connection | dbx connection/schema/query concepts (ADAPT) |
| `src-tauri/src/domain.rs` | `GraphNodeKind`/`GraphEdgeKind`, 7 capacity constants | structured node/edge types |
| `src-tauri/src/bridge.rs` / `src/bridge.ts` / `src/types.ts` | IPC + typed DTOs + ACL | any new command surface |
| `src-tauri/src/mcp_server.rs` / `mcp.rs` | existing Rust MCP (W5-W10 skeleton) | dbx MCP *concept* (already present) |
| **(new) `src-tauri/src/search.rs`** | proposed | zvec-grep retrieval routes (REIMPLEMENT) |
| **(new) `src-tauri/crates/*`** | proposed workspace member | dbx-core curated subset (COPY) |

No dedicated search/index module exists today → zvec-grep maps to a **new** `search.rs`.

---

## 1. dbx ledger (Apache-2.0; snapshot `Cargo.lock` SHA verified)

Upstream revision: local snapshot at `/home/ainfinit/Documents/极智简单/V3/research/dbx-src` (Apache-2.0; `Cargo.lock` SHA `c0a7be12…` matches dispatch pin). License: **Apache-2.0**. NOTICE: **none shipped**. Native footprint: see BOM §2 (curated subset is pure-Rust; reject vendored OpenSSL/aws-lc-rs).

| # | Source path / symbol | Destination | Class | Modification plan | Copied tests | Reject reason |
|---|---|---|---|---|---|---|
| D1 | `crates/dbx-core/build.rs` + `plugins/connection-types/*.yaml` (115) → `DatabaseType` enum | `src-tauri/build.rs` + `plugins/db-types/*.yaml` | **ADAPT** | port YAML→type generator; first wave sqlite/mysql/postgres only | `build.rs` unit tests | — |
| D2 | `connection.rs:85` `PoolKind` (23 variants) | `enum DbPool { Sqlite, MySql, Postgres }` in `database.rs` | **ADAPT** | keep 3 data variants; drop governance (MQ/Nacos/Consul/Mqtt) + JDBC | `connection.rs` pool tests | JDBC sidecar = JRE 百MB (conflict §4) |
| D3 | `production_safety.rs` `is_production_database`(:98)/`targets_production_database`(:184) | new `db_guard.rs` pure fns | **COPY** | lift as standalone pure-function module; unit-testable | `production_safety` tests | — |
| D4 | `sql_risk.rs` `classify_sql_risk_for_database`(:686)/`is_dangerous_sql_for_database`(:703) | `db_guard.rs` | **COPY** | reuse classifier; wire into command gate | `sql_risk` unit tests | — |
| D5 | `agent_tools.rs` `AgentSqlPermissions` + result caps (`EXECUTE_QUERY_LIMIT=50`, `MAX_ALLOWED_ROWS=100`, cell 200/4000) | `db_guard.rs` + `bridge.rs` | **COPY** | fail-closed write model + result truncation constants | `agent_tools` tests | — |
| D6 | `connection_secrets.rs:37` `FileSecretStore` + `state_persistence.rs:476-547` (argon2id+AES-256-GCM) | (headless only) `vault_fallback` | **REJECT as primary** / ADAPT fallback | **DO NOT** use as primary path; only headless/Docker fallback behind keyring | — | violates keyring红线 (conflict §3) |
| D7 | `history.rs` | `database.rs` history table | **ADAPT** | reuse schema + bounded retention | `history` tests | — |
| D8 | `query_execution_sql.rs` / `query_result_sql.rs` / `query_cancel.rs` | `database.rs` exec + cancel | **ADAPT** | reuse execution+cancel; drop gaussdb fork specifics | `query_*` tests | gaussdb fork (`tokio-postgres-gaussdb`) is a git dep → use upstream `tokio-postgres` |
| D9 | `schema.rs` (477KB) / `database_capabilities.rs` | `database.rs` schema browser | **REIMPLEMENT (thin)** | schema browsing is product-specific; only borrow capability-enum idea | — | module far too coarse to copy (conflict §7) |
| D10 | `sql_parser/` + `sql_dialect/` (25 files) / `sql_analysis.rs` | `search.rs`? no — `db_guard.rs` | **COPY** (via `sqlparser` crate) | use `sqlparser` 0.62 directly; don't copy dbx wrappers | `sql_parser` tests | — |
| D11 | `csv_export.rs`/`xlsx_export.rs`/`text_export.rs`/`database_export.rs`/`table_export.rs`/`table_import.rs` | new `export.rs` | **COPY** (curated) | reuse CSV/text/calamine(xlsx); respect capacity caps | `export` tests | — |
| D12 | `database_manifest.rs` / `database_search_sql.rs` | `database.rs` | **ADAPT** | reuse manifest + search; drop 90-db breadth | — | — |
| D13 | `agent_kv.rs` (93KB) AI memory KV | `agent_memory.rs` (already exists) | **REIMPLEMENT (concept)** | borrow KV abstraction; our product has a成果库 as agent knowledge source | `agent_kv` tests | dbx `agent_kv` is AI memory, NOT user成果库 (per dbx-study) |
| D14 | `crates/dbx-mcp/src/server.rs` `#[tool_router]` 9 tools (:277–787) | `mcp_server.rs` (already Rust) | **ADAPT concept** | we already embed Rust `rmcp`; wrap our capabilities similarly; **do not** import npm `mcp.rs` | server contract tests | MCP via npm分包 = Node≥18.18 + network (conflict §8.5) |
| D15 | `webview2_recovery.rs` (47KB) | browser-tabs stability (new module) | **REIMPLEMENT (concept)** | port the *idea*: graded process recovery + rolling reload budget + structured log; platform differs (Win WebView2 → Linux GTK) | recovery unit tests | cannot copy Windows-specific code; only the pattern |
| D16 | `app_settings.rs:83` `complete_app_close` → `app.exit(0)` | `shutdown.rs` (exists) | **COPY (concept)** | adopt single true-exit entrypoint | — | — |
| D17 | `ssh_prompt.rs` mpsc+oneshot handshake + sweeper | `database.rs` SSH tunnel | **ADAPT** | reuse async-handshake pattern for #6 connections | — | — |
| D18 | `deep_link.rs:4-6` `dbx://` scheme | #8 `http/https` handler | **REIMPLEMENT** | reuse "protocol registration + arg parse" pattern, swap scheme | — | private `dbx://` not applicable |
| D19 | `update.rs` + `updater:default` | #8 packaging | **DEFER** | out of W18-R; note for W19 | — | — |
| D20 | `consul/` `nacos/` `mq/` `mqtt/` governance connectors | — | **REJECT** | out of scope for #6 first wave | — | governance connectors not needed |
| D21 | `vendor/wry` (Tauri fork) | — | **REJECT** | we already carry `tauri-browser-tabs` + Tauri 2 | — | second Tauri fork = conflict |
| D22 | `ai_cli_agent.rs` 7-dialect CLI Agent host | #7 A2A (future) | **DEFER** | phase-2 multi-agent adapter | — | — |
| D23 | `agent_loop.rs` / `agent_service.rs` (218KB) / `ai.rs` (317KB) | — | **REJECT** | modules far too coarse (conflict §7); AI loop is product-specific | — | — |
| D24 | `transfer.rs` (474KB) cross-DB transfer | — | **REJECT** | heavy, out of first wave | — | — |

**dbx classification tally:** COPY 6 (D3,D4,D5,D10,D16-concept,D11) · ADAPT 8 (D1,D2,D7,D8,D12,D14,D17,D6-fallback) · REIMPLEMENT 4 (D9,D13,D15,D18) · DEFER 2 (D19,D22) · REJECT 5 (D20,D21,D23,D24,D6-primary).

---

## 2. zvec-grep ledger (Apache-2.0; upstream `52653951…`, snapshot unversioned; 130 `.ts`)

Upstream revision: `/home/ainfinit/Documents/极智简单/V3/research/zvec-grep-src` (`package.json` version `0.2.1`, Apache-2.0). **Critical: this is a Node/TypeScript package** — it cannot be copied into a Rust/Tauri binary. Native footprint: `@vscode/ripgrep` (bundled `rg` binary), `@huggingface/transformers`+`tokenizers` (native addon + model download), `@zvec/zvec` (ONNX/WASM), optional `node-llama-cpp` (compiles C++). License: **Apache-2.0**. NOTICE: **none shipped**.

| # | Source path / symbol | Destination (proposed) | Class | Modification plan | Copied tests | Reject reason |
|---|---|---|---|---|---|---|
| Z1 | `src/engine/extraction/code/CodeExtractor` (17 files) + `tree-sitter-wasms`/`web-tree-sitter` | `search.rs` extraction | **REIMPLEMENT** | port symbol/signature/breadcrumb extraction in Rust; reuse tree-sitter `tree-sitter-*` crates if needed (Rust bindings exist) | extraction unit tests | TS + WASM grammar; not copyable as-is |
| Z2 | `src/engine/extraction/markdown/MarkdownExtractor` | `search.rs` | **REIMPLEMENT** | port heading-section + breadcrumb chunking | markdown tests | TS-only |
| Z3 | `src/engine/extraction/text/TextExtractor` | `search.rs` | **COPY (algorithm)** | trivial plain-text chunking; reimplement in Rust | text tests | — |
| Z4 | `src/engine/storage/zvec.ts` + `layout.ts` (`.zvec-grep/` index: `manifest.json`, `files.zvec`, `index.zvec`) | `search.rs` index store | **REIMPLEMENT** | design Rust index layout (SQLite/redb/sled) mirroring manifest+files+index split | storage tests | TS/ONNX-specific |
| Z5 | `src/engine/service/lexical.ts` (managed ripgrep) | `search.rs` exact route | **ADAPT** | shell out to system `rg` (already on Linux) with bounded flags; own the output format (reject `--json/--replace` per zg) | lexical tests | `@vscode/ripgrep` npm wrapper not needed |
| Z6 | `src/engine/service/zvec-grep.ts` + `pipeline/search/*` (RRF fusion) | `search.rs` hybrid route | **REIMPLEMENT** | port reciprocal-rank-fusion of BM25+vector; this is the core IP worth reimplementing | fusion tests | needs vector backend (see Z8) |
| Z7 | `src/engine/pipeline/indexing/*` (incremental + watcher reconciliation) | `search.rs` indexer | **REIMPLEMENT** | port incremental update + `fresh`/`possibly_stale` reconciliation | indexing tests | TS-only |
| Z8 | `@zvec/zvec` vector core + `@huggingface/transformers` embedding | (external) | **REJECT copy** | vector embeddings require model weights + runtime; **ADAPT as sidecar** (run zvec-grep server) OR use a Rust embedding crate evaluated by A8/A9 | — | model download / remote egress (dispatch forbids in W18-R) |
| Z9 | `src/authorization/*` (8 files: manager/planner/store/target) | `security_policy.rs` + ACL | **REIMPLEMENT concept** | port the "explicit one-time/workspace authorization before remote egress" trust boundary; we already have source-check + ACL | authz tests | TS/MCP-specific |
| Z10 | `src/daemon/*` (16 files, loopback server) | (external sidecar) | **DEFER/REJECT-in-product** | long-running server is exactly what W18-R forbids adding; only as future sidecar with A0 approval | — | daemon forbidden this wave |
| Z11 | `src/mcp/*` (10 files, MCP tools) | `mcp_server.rs` (Rust) | **ADAPT concept** | we already have Rust MCP; expose equivalent search tools there | — | npm MCP impl not needed |
| Z12 | `src/engine/config.ts` + `file-size-policy.ts` (type-aware limits: 1MiB code/256MiB md/16MiB struct/10MiB img) | `search.rs` policy | **COPY (constants)** | lift the type-aware size-limit table verbatim as Rust consts | — | — |
| Z13 | `src/engine/service/structure-enrichment.ts` | `search.rs` | **REIMPLEMENT** | port structure enrichment (symbol→location breadcrumbs) | — | TS-only |

**zvec-grep classification tally:** COPY 1 (Z12) · ADAPT 3 (Z5,Z9-concept,Z11-concept) · REIMPLEMENT 7 (Z1,Z2,Z3,Z4,Z6,Z7,Z13) · DEFER/REJECT-in-product 2 (Z8,Z10).

**zvec-grep headline verdict:** **REJECT direct copy** (Node/TS + model/runtime bound). The transplantable *value* is the **algorithm layer** (route selection, RRF fusion, type-aware chunking, incremental reconciliation, freshness, compact output, authorization-before-egress) — to be **REIMPLEMENTED in Rust** (Z1–Z7,Z9,Z12,Z13) or, if embeddings are required, **ADAPTED as a sidecar** (Z8,Z10) only with A0 approval in W19.

---

## 3. Obsidian ledger (behavior-only — NO code donor)

Upstream: `/home/ainfinit/Documents/Knowledge-Base/secondBrain/.obsidian` config (read `graph.json`, `app.json`, `core-plugins.json`). **Obsidian is NOT open-source; reproduce only observable behavior + documented/local vault formats. No code/icons/branding/assets.** License: N/A (behavior reference). NOTICE: n/a. Native footprint: none.

| # | Observed behavior (evidence) | Destination | Class | Modification plan |
|---|---|---|---|---|
| O1 | Local graph: `search` filter, `showOrphans`, `colorGroups`, `collapse-color-groups`, force params (`centerStrength`/`repelStrength`/`linkStrength`/`linkDistance`), `nodeSizeMultiplier`, `lineSizeMultiplier`, `scale` (`graph.json`) | `graph.rs` + future graph UI | **REIMPLEMENT_FROM_BEHAVIOR** | our `bounded_subgraph`/`bounded_neighbors` already cover depth+limits; add filter/group/orphan toggles |
| O2 | Backlinks / outgoing-links panels (`core-plugins.json`: `backlink`,`outgoing-link`) | graph DTOs | **REIMPLEMENT** | derive back/out edges from `GraphEdge` store (already present) |
| O3 | Wikilinks `[[page]]`, tags `#tag`, frontmatter YAML, aliases, heading/block refs | (future) notes module | **REIMPLEMENT** | parser-only; no Obsidian code |
| O4 | Orphan detection (`showOrphans:true`) | `graph.rs` query | **REIMPLEMENT** | nodes with zero edges |
| O5 | Tag pane / canvas / outline / page-preview (`core-plugins.json`) | UI layer | **REIMPLEMENT** | product-specific UI; borrow interaction semantics only |
| O6 | Graph color groups / depth / selection / keyboard nav | graph UI state | **REIMPLEMENT** | see A3 report (pending) for Vue blueprint |

**Obsidian classification tally:** REIMPLEMENT_FROM_BEHAVIOR 6 (O1–O6). No COPY/ADAPT (no source). **Pending A2/A3** for the detailed Vue blueprint and vault-format fixtures.

---

## 4. Consolidated classification counts

| Source | COPY | ADAPT | REIMPLEMENT | DEFER | REJECT |
|---|---|---|---|---|---|
| dbx | 6 | 8 | 4 | 2 | 5 |
| zvec-grep | 1 | 3 | 7 | 1 | 1* |
| Obsidian | 0 | 0 | 6 | 0 | 0 |
| **Total** | **7** | **11** | **17** | **3** | **6** |

\* zvec-grep Z10 is DEFER/REJECT-in-product; Z8 REJECT-copy (ADAPT-sidecar as future). Counted REJECT here for "copy" dimension.

---

## 5. Cross-cutting transplant constraints (must hold for every copied unit)

1. **License retention**: copied files keep Apache-2.0 at file level; ship `LICENSE-APACHE-DBX`, `LICENSE-APACHE-ZVEC-GREP`; product stays MulanPSL-2 (see BOM §1).
2. **Modification notice**: every modified file carries a `// MODIFIED FROM <upstream> <rev>` header (see review-gate doc).
3. **Native budget**: exclude `openssl=vendored`, `aws-lc-rs`, `sqlite-sqlcipher`; keep the curated pure-Rust subset (BOM §2.4).
4. **No behavioral conflict**: never import dbx habits — command-without-ACL, close=hide-not-release, FileSecretStore primary, JDBC sidecar, single-window-replaces-subwebview, MCP npm分包 (dbx-study §9).
5. **Gate compliance**: every new command lands with source-check + handler registration + ACL + typed bridge/types + policy tests together (BOM/board non-repeat gate).
6. **Capacity/privacy**: copied units must obey existing capacity constants, privacy double-scan, and release-origin gate.

---

## 6. Pending upstream inputs (refine when landed)

- **A2** — Obsidian vault semantics + behavior fixtures → refine O1–O6 destinations/parser plan.
- **A4** — dbx backend deep map → confirm D1–D12 symbol-level signatures + test reuse.
- **A7** — zvec-grep ingestion/index architecture → confirm Z1–Z7 module boundaries.
- **A8** — zvec-grep retrieval benchmark + route selection → confirm Z5/Z6 fusion thresholds.
- **A9** — zvec-grep trust boundary → confirm Z9 authorization model + Z8 sidecar egress policy.

This ledger is complete enough for A0 to open W19 implementation cards now; the above refine precision, not architecture.
