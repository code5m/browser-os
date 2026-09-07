# A1 · M5-W18-R Checkpoint

```text
LANE=A1
STATUS=PASS_WITH_DEBT
BASE=78d2cfb
HEAD=e7f1c4f
REFERENCE_EVIDENCE=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3 (pinned HEAD 78d2cfb); WORKSPACE_IDENTITY.md; PARALLEL_COMMAND_BOARD.md L1350-1437; logs/checkpoints/M5-20260906/M5-14-debt-ledger.md; scripts/measure-build-metrics.py
FILES=logs/research/M5-W18/A1-product-baseline-20260908.md, logs/research/M5-W18/A1-checkpoint-20260908.md
SOURCE_MAP=src-tauri/src/*.rs (29 modules, 30,985 lines); src/stores/*.ts (16); src/components/*; src-tauri/permissions/*.toml; src-tauri/capabilities/*.json + dev-capabilities/main.json; scripts/check-*.py|mjs (49)
CLASSIFICATION=N/A (baseline lane; COPY/ADAPT decisions belong to A4/A7/A10)
VERIFY=read-only static scan (grep/wc/sed/diff); no build or test executed (research-only boundary)
CHECKPOINT=logs/research/M5-W18/A1-checkpoint-20260908.md
MERGE_NOTES=A1 is first in A0 integration order (A1 -> A2/A3 -> A4/A5/A6 -> A7/A8/A9 -> A10 -> A11 -> A0). gap inventory + requirements checklist are mandatory inputs for A2-A11.
NEXT=A0 must clear W17 residual debt (W17-D1..D6) and DbValue triple-source drift (G4) before opening W19.
```

## What A1 delivered

**End-to-end baseline of the current product (HEAD `78d2cfb`)** across the 10 dimensions required by board L1395:
- graph/database Rust modules (graph.rs 733 / database.rs 1273 / domain.rs 2521)
- DTOs — **discovered DbValue triple-source drift** (database.rs live `I64/F64/Binary{bytes}` vs domain.rs dead-code `Int/Float/BlobLen(u64)` vs types.ts PascalCase) → root cause of B8-1
- 137 tauri::command (133 main-window + 4 grid-child, ACL releases all 137)
- 16 stores; panels present for graph (`src/components/graph/` 5 components) and database (`workspace/DatabasePanel.vue`)
- 49 policy/verification scripts
- 470 Rust tests; **0 frontend unit tests** (no vitest/jest) — gap G5
- M5-14 debt ledger: 32 debt rows incl. D23-D26 + W17-D1..D6, 4 P0-level marks
- build-size budget: limit `TOTAL_BYTES_GROWTH_LIMIT_PCT = 25.2`; W17 measured 25.55% (over)
- locked authorities: release capabilities = 4 (`default.json`) + 2 (`browser-remote.json`); dev-only `main.json` (4) under `#[cfg(debug_assertions)]`; runtime authority LOCKED with launch blacklists in security_policy.rs

## Canonical Gap Inventory (8 gaps: G1-G8)

G1 Obsidian semantics on top of existing GraphStore (HIGH, stack not rewrite) · G2 dbx full workbench (HIGH) · **G3 code-retrieval backend = ZERO (EXTREME, from scratch)** · G4 DTO single-source (HIGH, B8-1 root) · G5 frontend unit-test net = 0 (MED) · G6 size budget headroom exhausted (HIGH) · G7 permission surface zero-expansion (HIGH) · G8 uncleared debt (MED).

## Requirements Checklist (R1-R12)

Mandatory for every A2-A11 report: current-product gap, upstream source map (exact files/symbols + pinned revisions), data/control flow, persistence format, concurrency/lifecycle, security/privacy, performance/capacity, dependencies/licenses, target mapping, test reuse, unresolved questions, and (A4/A7/A10 only) COPY/ADAPT/REIMPLEMENT/DEFER/REJECT classification. Full text in `A1-product-baseline-20260908.md` §4.

## Debt carried into W19

- W17-D1..D6 must be cleared or explicitly carry-forwarded before W19 opens.
- DbValue triple-source drift (G4) must be unified to single source (`database.rs` live) before any new DTO is added.
- Frontend unit-test net (G5) must be bootstrapped for new Vue components/stores.

## Boundary compliance

- Zero product-code changes; no dependency install; no build/test run; no push.
- All output under `logs/research/M5-W18/` only.
