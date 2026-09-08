# A10 — M5-W18-R Lane Checkpoint

```text
LANE=A10
STATUS=PASS_WITH_DEBT
BASE=78d2cfb8e90d323d35df920e9807d32189f867cd
HEAD=3d513032cf340afbb1a57287cc8fc3792018aaf2
REFERENCE_EVIDENCE=
  - dbx: /home/ainfinit/Documents/极智简单/V3/research/dbx-src (Apache-2.0; Cargo.lock SHA-256 c0a7be12c05d8dffe867f1a70d4b82e881dec2da3bb622b5d3e3410c3d10e3a7, verified == dispatch pin)
  - zvec-grep: /home/ainfinit/Documents/极智简单/V3/research/zvec-grep-src (Apache-2.0; npm @zvec/zvec-grep 0.2.1; upstream rev 52653951b24617762f4ab0c71c34d594e5001617 per dispatch; snapshot unversioned locally)
  - Obsidian: /home/ainfinit/Documents/Knowledge-Base/secondBrain/.obsidian/{graph,app,core-plugins}.json (behavior/configuration reference only; no source donor)
  - Prior analysis: /home/ainfinit/Documents/极智简单/V3/dbx-study/{dbx-借鉴分析,dbx-功能借鉴清单,dbx-冲突风险}.md (V5, cited as evidence)
  - Product: /home/ainfinit/.codex/worktrees/m5-w18-a10/mvp-browser-os-v3 (MulanPSL-2; src-tauri/Cargo.toml, src-tauri/src/{graph,database,domain,bridge,mcp_server}.rs, default-commands.toml)
FILES=
  - logs/research/M5-W18/A10-dependency-bom.md
  - logs/research/M5-W18/A10-source-transplant-ledger.md
  - logs/research/M5-W18/A10-notice-and-review-gate.md
  - logs/research/M5-W18/A10-checkpoint.md
SOURCE_MAP=
  - dbx-core: connection.rs(PoolKind:85), production_safety.rs(:98/:184), sql_risk.rs(:686/:703), agent_tools.rs, connection_secrets.rs(:37), state_persistence.rs(:476-547), history.rs, query_execution_sql.rs/query_result_sql.rs/query_cancel.rs, schema.rs, sql_parser/, sql_dialect/, sql_analysis.rs, csv_export.rs/xlsx_export.rs/text_export.rs/database_export.rs, database_manifest.rs, agent_kv.rs, dbx-mcp/src/server.rs(:277-787), webview2_recovery.rs, app_settings.rs(:83), ssh_prompt.rs, deep_link.rs, update.rs, build.rs, plugins/connection-types/*.yaml, vendor/{ctor,rumqttc,dirs-sys,pageant,tiberius,wry}
  - zvec-grep: src/engine/extraction/{code,markdown,text}/, storage/zvec.ts+layout.ts, service/{lexical,zvec-grep,structure-enrichment,workspace-index}.ts, pipeline/{indexing,search}/, authorization/*, daemon/*, mcp/*, cli/*, config.ts, file-size-policy.ts
  - Obsidian: graph.json (local-graph filters/groups/depth/orphans/forces), core-plugins.json (graph/backlink/outgoing-link/tag-pane/canvas), app.json
CLASSIFICATION=COPY:7 ADAPT:11 REIMPLEMENT:17 DEFER:3 REJECT:6
VERIFY=
  - sha256sum dbx Cargo.lock == c0a7be12c05d8dffe867f1a70d4b82e881dec2da3bb622b5d3e3410c3d10e3a7 (dispatch pin) -> MATCH
  - both upstream LICENSE files read in full -> Apache-2.0 confirmed
  - product LICENSE read -> MulanPSL-2 confirmed (dual-license handling required)
  - zvec-grep package.json parsed -> Node>=22, 130 .ts, deps are Node/model-bound (REJECT copy)
  - no product code edited; only logs/research/M5-W18/ written (read-only research)
CHECKPOINT=logs/research/M5-W18/A10-checkpoint.md
MERGE_NOTES=
  - Depends on A2/A4/A7/A8/A9 upstream reports for symbol-level precision (refinement only; architecture settled)
  - License: inbound Apache-2.0 into MulanPSL-2 product -> dual attribution + NOTICE required
  - Native budget: reject openssl=vendored, aws-lc-rs, sqlite-sqlcipher, Tauri wry fork, Node embedding deps
  - No-blind-copy gate: scripts/check-transplant-policy.py (8 gates) proposed for W19 pre-merge
  - W18-R boundary honored: no product code, no daemon/MCP/model/network, no vault mutation
NEXT=
  - Pending A2 (Obsidian Vue blueprint), A4 (dbx backend map), A7-A9 (zvec-grep maps) to finalize O1-O6 / D1-D24 / Z1-Z13 detail
  - A0 decision needed: zvec-grep embeddings = REIMPLEMENT (Rust) vs ADAPT-sidecar (egress policy per A9)
  - Proposed W19 slices: (1) curated pure-Rust DB driver subset + db_guard.rs (D2-D5,D8,D10,D11); (2) search.rs reimplement of zvec route/RRF/chunking (Z1-Z7,Z12,Z13) or sidecar; (3) NOTICE + check-transplant-policy.py gate
```

## Status note

STATUS = `PASS_WITH_DEBT` because the ledger is complete and sufficient for A0 to open W19 cards, but upstream lane reports (A2/A4/A7–A9) were not yet available at write time; their absence is a *refinement* debt, not a blocker. The research itself is internally consistent and evidence-backed (verified Cargo.lock SHA, read licenses, parsed package manifests, inspected source trees).

## Deliverables in this lane

1. **`A10-dependency-bom.md`** — full dependency BOM: dbx Rust crate table (COPY/AVOID/DEFER + native footprint), dbx workspace vendored forks, zvec-grep npm table (Node/model-bound), license topology (MulanPSL-2 ↔ Apache-2.0), provenance verification.
2. **`A10-source-transplant-ledger.md`** — per-candidate ledger for dbx (D1–D24), zvec-grep (Z1–Z13), Obsidian (O1–O6) with all required fields and consolidated classification counts.
3. **`A10-notice-and-review-gate.md`** — NOTICE/attribution artifacts proposal + `check-transplant-policy.py` 8-gate design + reviewer checklist + integration into pre-merge.

No product code changed. Not pushed (only A0 integrates/pushes).
