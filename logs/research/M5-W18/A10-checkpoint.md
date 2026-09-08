# A10 — M5-W18-R2 Lane Checkpoint

```text
LANE=A10
STATUS=PASS_WITH_DEBT (HOLD finalization pending peer A2/A4/A7/A8/A9 R2 commits)
BASE=78d2cfb8e90d323d35df920e9807d32189f867cd
REBASED_ONTO=origin/master d6127c4 (docs(M5-W18): dispatch evidence closure research)
HEAD=08e61c55679bd3de1b8bb45a864c241732302549
REFERENCE_EVIDENCE=
  - dbx: /home/ainfinit/Documents/极智简单/V3/research/dbx-src (Apache-2.0; Cargo.lock SHA-256 c0a7be12c05d8dffe867f1a70d4b82e881dec2da3bb622b5d3e3410c3d10e3a7, verified == dispatch pin)
  - zvec-grep: /home/ainfinit/Documents/极智简单/V3/research/zvec-grep-src (Apache-2.0; npm @zvec/zvec-grep 0.2.1; upstream rev 52653951b24617762f4ab0c71c34d594e5001617 per dispatch)
  - Obsidian: /home/ainfinit/Documents/Knowledge-Base/secondBrain/.obsidian/{graph,app,core-plugins}.json (behavior/configuration reference only; no source donor)
  - Product: /home/ainfinit/.codex/worktrees/m5-w18-a10/mvp-browser-os-v3 (root LICENSE=MulanPSL-2; src-tauri/Cargo.toml has NO license field; src-tauri/src/{graph,database,security_policy,domain,bridge,mcp_server,shutdown}.rs)
FILES=
  - logs/research/M5-W18/A10-dependency-bom.md (R2)
  - logs/research/M5-W18/A10-source-transplant-ledger.md (R2)
  - logs/research/M5-W18/A10-notice-and-review-gate.md (R2)
  - logs/research/M5-W18/A10-checkpoint.md (R2)
R1_VERDICT=A0: REWORK ("license/transplant contradictions, no function-level closure for COPY candidates")
R2_CORRECTIONS=C1(async drivers) C2(workspace member) C3(D3 COPY) C4(D4 COPY) C5(D16 COPY) C6(D11 split) C7(D10 COPY) C8(product license)
CONSUMED_PEERS=A4(R1 map: D3/D4 ADAPT diff-only; sync runtime D-A4-2), A7(R1: @zvec/zvec Rust API UNPROVEN -> Z8 REJECT-copy), A8(R1: managed-ripgrep kept), A9(R1: egress DISABLED-by-default, apiKey->keychain, B2/B3/B7 debt)
CLASSIFICATION=COPY:3 ADAPT:15 REIMPLEMENT:17 DEFER:3 REJECT:8
TRUE_COPY_UNITS=D5a(caps consts), D11a(format_csv cluster), Z12(type-aware size consts)
VERIFY=
  - sha256sum dbx Cargo.lock == c0a7be12... (dispatch pin) -> MATCH
  - product root LICENSE read -> MulanPSL-2 confirmed (23 "Mulan/木兰"); src-tauri/Cargo.toml NO license field (R2 gap)
  - product Cargo.lock (293 crates): regex 1.13.1, tokio 1.53.1 transitive, rusqlite 0.40.2, mysql 28.0.2, postgres 0.19.14 PRESENT; sqlparser/notify/csv/aes-gcm/argon2/calamine ABSENT
  - product security_policy.rs already has classify_sql_risk(:1046)/SqlRiskClass(:728)/ProductionSignals(:1118)/is_production_database(:1146) -> D3/D4 duplicate
  - dbx file:line symbols read for D3/D4/D5/D10/D11/D16 (function-level closure captured)
  - zvec-grep file-size-policy.ts consts + package.json deps read (Z12 self-contained; all else Node/model-bound)
  - no product code edited; only logs/research/M5-W18/ written (read-only research)
MERGE_NOTES=
  - Depends on A2/A3 (Obsidian Vue blueprint) and A7 (Z8 native proof) + A9 (egress policy) R2 commits to finalize (dispatch R2 sequencing)
  - License: inbound Apache-2.0 into MulanPSL-2 product -> dual attribution + NOTICE required; add license field to Cargo.toml at W19
  - Native budget: reuse existing sync mysql/postgres/rusqlite; reject openssl=vendored, aws-lc-rs, sqlite-sqlcipher, Tauri wry fork, Node embedding deps, async drivers (C1)
  - No-blind-copy gate: scripts/check-transplant-policy.py (8 gates) proposed for W19 pre-merge
  - W18-R boundary honored: no product code, no daemon/MCP/model/network, no vault mutation
NEXT=
  - AWAIT A2/A4/A7/A8/A9 R2 commits; then re-confirm Z8 (REJECT-copy / sidecar HOLD) and O1-O6 destinations
  - A0 decision needed: Z8 embeddings = REIMPLEMENT(Rust) vs ADAPT-sidecar (egress policy per A9)
  - Proposed W19 slices: (1) curated pure-Rust DB subset + db_guard.rs (D2,D3-diff,D4-opt,D5a,D7,D8,D9,D10,D12); (2) export.rs = D11a COPY; (3) search.rs REIMPLEMENT of zvec routes/RRF/chunking (Z1-Z7,Z9,Z12,Z13) or sidecar; (4) NOTICE + check-transplant-policy.py gate
```

## Status note

STATUS = `PASS_WITH_DEBT` (HOLD finalization). All license analysis and function-level provenance/dependency/test closures requested by R2 are complete and evidence-backed from `CURRENT_PRODUCT` + `REFERENCE_SOURCE`. The R1 contradictions named in `A0-M5-W18-R1-audit-20260908.md` are explicitly retracted and replaced (§0 of the ledger). The only remaining items are **peer-R2 confirmations** (A2/A3 Obsidian Vue blueprint for O1–O6, A7 native-binding proof for Z8, A9 egress policy for Z8 sidecar) which, per R2 sequencing, must land before A10 may finalize — but none would change the first implementation slice.

## R1 → R2 correction summary (audit items closed)

| Audit item | A10 R1 issue | R2 resolution |
|---|---|---|
| license contradiction | implicit product=Apache-2.0 risk; "workspace member" surface | product = MulanPSL-2 verified; transplant lands in existing `src-tauri/src/` modules (M5-1.a), not a new workspace member (C2) |
| transplant contradiction | recommended async `tokio-postgres`/`mysql_async` COPY | withdrawn — product uses sync `mysql`/`postgres`; M4-A2 F-1 forbids tokio direct dep; A4 D-A4-2 requires sync (C1) |
| no function-level closure | D3/D4/D5/D10/D11/D16 marked module-level COPY | split into self-contained COPY (D5a, D11a, Z12) vs ADAPT/REJECT using file:line symbol + transitive-dep closure |

## Deliverables in this lane (R2)

1. **`A10-dependency-bom.md`** (R2) — sync-driver reuse, `sqlparser` optional, crypto/export pure-Rust subset, license topology + Cargo.toml license-field gap.
2. **`A10-source-transplant-ledger.md`** (R2) — per-unit function-level closure, consolidated COPY:3/ADAPT:15/REIMPLEMENT:17/DEFER:3/REJECT:8, explicit ACCEPT/REJECT, hard stops.
3. **`A10-notice-and-review-gate.md`** (R2) — NOTICE + `check-transplant-policy.py` 8-gate design reconciled with A9 egress policy.

No product code changed. Not pushed (only A0 integrates/pushes).
