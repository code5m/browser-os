# Lane A7 · M5-W18-R2B checkpoint

```text
LANE=A7
DISPATCH=M5-W18-R2B
STATUS=PASS_WITH_DEBT
WORKDIR=/home/ainfinit/.codex/worktrees/m5-w18-a7/mvp-browser-os-v3
BRANCH=codex/m5-w18-a7
BASE=434e63f8e90d323d35df920e9807d32189f867cd  (origin/master, "docs(M5-W18): define daily workbench and second research dispatch")
HEAD=5ab805673442dcb29bb99706842d2ca21f3b97be  (this lane's R2B report commit; checkpoint committed immediately after)
CONSUMED_PEERS=M5-W18-R2B-TASKS.md §8 (Lane A7); A0-M5-W18-R2B-handoff-20260908.md (checkpoint items 5/6/9); A7-zvec-grep-ingestion-index-architecture.md (R1); A7-zvec-grep-R2-evidence-closure.md (R2, §9 corrections appended); WORKSPACE_IDENTITY.md Desktop Runtime Source Gate; alibaba/zvec README (line 68); zvec-rust/zvec-rust-sys 0.7.0 sources.
FILES=logs/research/M5-W18/A7-R2B-retrieval-access-index-transaction.md (NEW); logs/research/M5-W18/A7-zvec-grep-R2-evidence-closure.md (§9 corrections appended, history preserved); logs/checkpoints/A7-M5-W18-R2B-checkpoint.md (this file).
CORRECTIONS=(retracted from prior R2 report) C1 "PROVE/ADOPT via zvec-rust" -> downgraded to build/run PROVEN, integration decision deferred to A0; C2 "indexes interchangeable iff version pinned" -> retracted, cross-binding interop UNPROVEN; C3 "WAL/write-lock guarantees crash safety" -> retracted, replaced by manifest+generational-commit design + cancel/crash/concurrent tests; C4 "force_rebuild wipes <root>/.zvec-grep" -> corrected, product-own namespace only, never delete user .zvec-grep; C5 "insert ACL before list_artifact_images" -> retracted as invented ordering rule; C6 "remote-embedding HMAC = COPY(pattern)" -> corrected to behavior reimplementation.
VERIFY=EXECUTED_SYNTHETIC_TEST: cargo 1.96 build of zvec-rust 0.7.0 + link prebuilt libzvec_c_api (from zvec-ai/zvec-rust release) succeeded; lifecycle initialize -> create_and_open -> insert(5,success=5) -> query(top-3 hits) -> drop(close) printed OPEN_OK/INSERT success=5 error=0/QUERY hits=3/CLOSE_OK in /tmp/m5-w18-a7-zvec/exp. FTS API (IndexParams::fts + Fts) confirmed in source. NOT EXECUTED (design only, out of research scope): cross-binding round-trip, crash/cancel/concurrent tests, capacity enforcement, ACL/capability wiring, policy gate.
PROPOSED_SLICES=(both FTS-only, no embedding/daemon/watcher) A "managed-rg no-index": live ripgrep-style (ignore crate) FTS over configured roots, zero index machinery; B "bounded manual index": zvec FTS index in product-own namespace <app_cache>/search-index/<root-hash>/ with manifest+generational commit, capped (MAX_FILES=5000/MAX_TOTAL_BYTES=256MiB/MAX_DEPTH=20/CHUNK_CHARS=3600/TOP_K=7/TIMEOUT=60s). A0/W19 picks one.
OPEN_DECISIONS=(1) binding choice Rust zvec-rust vs Node sidecar -> A0; (2) first-slice candidate A vs B -> A0/W19; (3) cross-binding interop needs explicit round-trip test before any shared-index assumption; (4) version() returns v0.0.0 vs crate 0.7.0 -> decide version-pin key; (5) hermetic libzvec_c_api source (vendored prebuilt vs CI CMake) -> A10.
NEXT=A0 adjudicates binding + first-slice candidate; A10 records zvec-rust 0.7.0 + vendored libzvec_c_api (Apache-2.0 -> MulanPSL-2.0, NOTICE) + hermetic-build approach; A11 folds cancel/crash/concurrent + capacity tests into W19 acceptance; W19 implements one lightweight slice per source gate (command->ACL->capability->typed bridge/types->policy/test), never touching user .zvec-grep.
NO_PRODUCT_CODE=true
NO_PUSH=true  (only A0 may push)
```

## Notes

- Executed in the dedicated W18-R2B worktree `m5-w18-a7` (branch `codex/m5-w18-a7`), rebased onto `origin/master` (434e63f). The canonical master working copy was **not** modified.
- Research-only: no product-code edit, no dependency installed into the product, no daemon/model download. Experiment artifacts confined to the disposable sandbox `/tmp/m5-w18-a7-zvec` (the R2-authorized sandbox).
- This R2B package **preserves** the prior R1/R2 research commits and **amends findings explicitly** (§9 of the R2 report + this checkpoint), per A0 checkpoint items 5/6/9 and R2B card §8.
- The prior R2 conclusion "ADOPT via zvec-rust" is **retracted** (C1): the Rust binding is now *proven buildable/runnable*, but the product integration decision is **deferred to A0** (reconcile with A8, which currently says "only Node sidecar"). A7/A8 contradiction resolved by documenting both routes as viable and leaving the choice to A0; **cross-binding index interop remains UNPROVEN** (S4).
- Full experiment transcript, the five separated sub-claims (S1–S5), authoritative `zvec-rust` provenance, the manifest/generational-commit design, both first-slice candidates, source-mapping/classification corrections, and the A7/A8 reconciliation are in `A7-R2B-retrieval-access-index-transaction.md`.
