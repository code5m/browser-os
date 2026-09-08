# Lane A7 · M5-W18-R checkpoint

```text
LANE=A7
STATUS=PASS
BASE=78d2cfb8e90d323d35df920e9807d32189f867cd
HEAD=bd8bd52edc02dd869e595639e4316e602a9a43e0
REFERENCE_EVIDENCE=/home/ainfinit/Documents/极智简单/V3/research/zvec-grep-src (upstream zvec-ai/zvec-grep @ 52653951b24617762f4ab0c71c34d594e5001617; snapshot HEAD 5265395, version 0.2.1, Apache-2.0); product gap verified via read-only grep of mvp-browser-os-v3 src/ + src-tauri/ + scripts/.
FILES=logs/research/M5-W18/A7-zvec-grep-ingestion-index-architecture.md, logs/research/M5-W18/A7-lane-checkpoint.md
SOURCE_MAP=scanner/index.ts (scanRootPaths/scanFilePath/DEFAULT_IGNORE_RULES/MAX_GITIGNORE_CACHE_ENTRIES), authorization/types.ts + store.ts/manager.ts (RemoteEmbeddingWorkspaceGrant/HMAC/AsyncLocalStorage guard), extraction/* (extractForIndexing/CodeExtractor/DEFAULT_CODE_CHUNK_CHARS=3600), storage/layout.ts (files.zvec/index.zvec) + zvec.ts (ZVEC_UPSERT_BATCH_SIZE=1024/ZVEC_MAX_QUERY_TOPK=100000), pipeline/indexing/index.ts + input-budget.ts (DEFAULT_CHARS_PER_100_TOKENS=185/CHUNK_OVERLAP_PERCENT=15), daemon/change-set.ts (maxChangedPaths=1000/forceFullReconcile), daemon/watch-manager.ts (debounce 750/maxWait 5000/reconcile 60min/resume 90s), daemon/backend.ts (rebuild gate), engine/service/zvec-grep.ts (RRF_K=60), pipeline/search/index.ts (DEFAULT_LIMIT=7/RRF_K=60/RECALL_MAX_DEPTH=2000).
CLASSIFICATION=COPY=2 (@zvec/zvec engine, ripgrep core); ADAPT=3 (@zvec/zvec native/sidecar, local embeddings, manifest/storage format); REIMPLEMENT_FROM_BEHAVIOR=7 (ignore rules, extractors/chunking, incremental ChangeSet, watcher, corruption/rebuild, concurrency, remote-embedding auth pattern); DEFER=1 (CLI/daemon/MCP); REJECT=0.
VERIFY=read-only research; symbols confirmed by direct file read + grep (scanner/index.ts:137 DEFAULT_IGNORE_RULES, zvec.ts:73-74, code/extractor.ts:21, search/index.ts:76-79, watch-manager.ts:64). No zvec-grep build/install/daemon/model/download performed (dispatch forbids). Product gap confirmed: zero local search/index subsystem in mvp-browser-os-v3 (only database.rs SQLite + mcp.rs browser-context queries).
CHECKPOINT=logs/research/M5-W18/A7-lane-checkpoint.md
MERGE_NOTES=depends on A1 (product baseline gap inventory) for final gap wording; feeds A10 (source-transplant ledger / BOM — seeds in report §9); feeds A9 (threat model: remote-embedding auth is the ONLY egress gate; local path scope is config-trusted and must be reconciled with product's OS-level workspace authorization); feeds A11 (verification architecture — reuse upstream tests listed in report §7). Security debt: remote-embedding consent model must be copied; config-trusted RootPath scope must be unified with product workspace grant before any adoption. No conflicts identified with other lanes' scopes (A4/A5/A6 dbx, A8 retrieval, A9 trust, A2/A3 Obsidian are disjoint).
NEXT=A10 must resolve whether @zvec/zvec exposes a Rust API (napi/neon) usable directly from Tauri or is Node-only (decides COPY-vs-sidecar). A9 must unify workspace-grant vs config-trusted path scope. Proposed W19 slice: reimplement ingestion/index behavior in Rust using @zvec/zvec native engine + ripgrep crate + tree-sitter, preserving .zvec-grep on-disk contract; defer daemon/MCP.
```

## Notes

- Executed in the dedicated W18 worktree `m5-w18-a7` (branch `codex/m5-w18-a7`), per `PARALLEL_COMMAND_BOARD.md` § "Dedicated lane worktrees" — the canonical master working copy was **not** modified.
- Research-only: no product-code edit, no dependency install, no daemon/model/download/network; only report + checkpoint files written under `logs/research/M5-W18/`.
- `HEAD` above is filled with the commit SHA produced by the lane-local commit (see commit message); no push performed.
- Full source map, persistence format, concurrency/lifecycle, security/privacy, capacity table, dependency/license BOM, target mapping + per-unit classification, test-reuse list, and unresolved questions are in `A7-zvec-grep-ingestion-index-architecture.md`.
