# A1 · M5-W18-R2 Checkpoint

```text
LANE=A1
STATUS=PASS_WITH_DEBT
BASE=d6127c4
HEAD=<pending commit>
REFERENCE_EVIDENCE=CURRENT_PRODUCT: src-tauri/src/*.rs, src/*.ts, src/components/*.vue, src-tauri/permissions/*.toml, src-tauri/capabilities/*.json, scripts/check-*, LICENSE, logs/checkpoints/M5-20260906/M5-14-debt-ledger.md; REFERENCE_SOURCE: WORKSPACE_IDENTITY.md, PARALLEL_COMMAND_BOARD.md L1439-1484, logs/checkpoints/A0-M5-W18-R1-audit-20260908.md
FILES=logs/research/M5-W18/A1-product-baseline-R2-20260908.md, logs/research/M5-W18/A1-checkpoint-R2-20260908.md
SOURCE_MAP=src-tauri/src/*.rs (28 modules, 30,832 lines); src/stores/*.ts (16); src/components/* (9 dirs); src-tauri/permissions/*.toml (2); src-tauri/capabilities/*.json (2) + dev-capabilities/main.json (1); scripts/check-* (50: 29 py + 20 mjs + 1 sh)
CLASSIFICATION=N/A (baseline lane; COPY/ADAPT decisions belong to A4/A7/A10)
VERIFY=read-only static scan (grep/wc/python3); no build or test executed (research-only boundary)
CHECKPOINT=logs/research/M5-W18/A1-checkpoint-R2-20260908.md
MERGE_NOTES=A1 is first in A0 integration order. R2 report corrects 6 R1 factual errors; machine-checkable fact appendix provided.
NEXT=W17-D1~D5 must be cleared by A11/A0; DbValue dual-source drift (G4) must be unified by A10/A11; frontend unit-test net (G5) bootstrapped by A11.
```

## What A1 R2 delivered

**Corrected end-to-end baseline of current product (HEAD `d6127c4`)** with 6 explicit R1 corrections:

| # | R1 error | R2 correction | Evidence |
|---|---|---|---|
| C1 | 137 commands / 137 ACL | **135** registered / **135** ACL / **46** bridge invocations (137 annotations include 2 unregistered) | `EXECUTED_SYNTHETIC_TEST` |
| C2 | DbValue triple-source drift, types.ts PascalCase | **Dual-source** drift: database.rs (live) vs domain.rs+types.ts (dead-code + frontend, snake_case aligned). B8-1 already fixed. | `CURRENT_PRODUCT` |
| C3 | 49 policy scripts | **50** total (29 Python + 20 MJS + 1 shell) | `EXECUTED_SYNTHETIC_TEST` |
| C4 | 32 debt rows, 4 P0 | **21** debt rows, **2** P0; W17-D1~D5 (not D1~D6; D6 belongs to A7/A0) | `EXECUTED_SYNTHETIC_TEST` |
| C5 | 29 Rust modules | **28** modules, 30,832 lines | `EXECUTED_SYNTHETIC_TEST` |
| C6 | 470 tests (undifferentiated) | **470** `#[test]` + **0** `#[tokio::test]`; database.rs has **23** `#[test]` | `EXECUTED_SYNTHETIC_TEST` |

## Key facts (machine-checkable, see report §7)

- Commands: 135/135/46 (registered/ACL/bridge)
- Policy scripts: 50 (29 py + 20 mjs + 1 sh)
- Tests: 470 Rust `#[test]`, 0 `#[tokio::test]`, 0 frontend
- Stores: 16; `useConnectionStore` does NOT exist
- DbValue: dual-source (database.rs live `I64/F64/Binary` vs domain.rs dead-code `Int/Float/BlobLen`); types.ts snake_case aligned with domain.rs
- Build-size limit: 25.2% (W17 measured 25.55%, over, pending A11 clean-tree retest)
- Capabilities: default.json=4, browser-remote.json=2, dev-capabilities/main.json=4
- Security blacklists: programs=12, wrappers=10, interpreters=12
- License: MulanPSL-2.0 (NOT Apache-2.0)
- Debt: 21 rows, 2 P0, W17-D1~D5

## Corrected Gap Inventory (8 gaps)

G1 Obsidian semantics on existing GraphStore (HIGH) · G2 dbx full workbench (HIGH) · G3 code-retrieval = ZERO (EXTREME) · **G4 DbValue dual-source drift (HIGH, corrected from triple)** · G5 frontend unit-test net = 0 (MED) · G6 size budget headroom exhausted (HIGH) · G7 permission surface zero-expansion (HIGH) · G8 uncleared debt (MED, corrected counts).

## Unresolved questions dispatched

- U1 (2 unregistered commands) → A11
- U2 (DatabasePanel.vue completeness) → A5
- U3 (metrics json field completeness) → A11
- U4 (graph snapshot vs Obsidian graph.json compatibility) → A2
- U5 (database.rs 23 vs audit 22 tests) → A0/A11

## Boundary compliance

- Zero product-code changes; no dependency install; no build/test run; no push.
- All output under `logs/research/M5-W18/A1-*` only.
- R1 report preserved as draft (not silently rewritten).
- Every factual claim tagged with evidence type per R2 contract §3.
