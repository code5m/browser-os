# Lane A7 · M5-W18-R2 checkpoint

```text
LANE=A7
STATUS=PASS
BASE=78d2cfb8e90d323d35df920e9807d32189f867cd
HEAD=8ec92bbf5baace5df3a499169816e6c95293a121
R1_VERDICT=REWORK (resolved in R2)
R2_DISPATCH=PARALLEL_COMMAND_BOARD.md § "M5-W18-R2 Evidence Closure Dispatch" (A7 row line 1463)
REFERENCE_EVIDENCE=/tmp/m5-w18-a7-zvec (disposable) — npm @zvec/zvec 0.7.0 tarball (sha1 66b676c1...), @zvec/bindings-linux-x64 0.7.0 (.node ELF/N-API), crates.io zvec-rust 0.7.0 + zvec-rust-sys 0.7.0 (source-inspected build.rs), api.github.com alibaba/zvec + zvec-ai/zvec-rust; product facts: LICENSE (MulanPSL-2.0), src-tauri/src/workspace.rs (no grant model), src-tauri/Cargo.toml (no zvec/search/cmake), permissions/default-commands.toml (last line list_artifact_images).
FILES=logs/research/M5-W18/A7-zvec-grep-R2-evidence-closure.md (NEW), logs/research/M5-W18/A7-zvec-grep-ingestion-index-architecture.md (R2 corrections appended §11), logs/research/M5-W18/A7-lane-checkpoint.md (this file)
SOURCE_MAP=@zvec/zvec 0.7.0 (N-API addon, Node-only; REJECT for Rust); alibaba/zvec C++ engine exposing C API libzvec_c_api; zvec-rust 0.7.0 (official Rust crate, zvec-ai/zvec-rust) -> zvec-rust-sys 0.7.0 (links dylib=zvec_c_api); community zvec 0.1.0 / zvec-sys 0.4.1 (excluded). Product module targets: src-tauri/src/search_index.rs (NEW), main.rs generate_handler!, permissions/default-commands.toml (insert before list_artifact_images), capabilities/default.json, src/bridge.ts + src/types.ts, Cargo.toml (zvec-rust="=0.7.0", ignore, vendored libzvec_c_api), tests/search_index_fts.rs.
CLASSIFICATION=COPY(pattern, remote-embedding HMAC grant)=1; ADAPT(engine via zvec-rust / libzvec_c_api, ripgrep discovery via ignore crate, chunking, RootPath scope)=5; REIMPLEMENT_FROM_BEHAVIOR(ignore rules, extractors, incremental, watcher, corruption/rebuild, concurrency)=6; DEFER(watcher/ChangeSet, daemon/MCP)=2; REJECT(@zvec/zvec npm N-API for Rust, CLI/daemon first slice)=2.
VERIFY=read-only research. Artifacts downloaded only into /tmp/m5-w18-a7-zvec (no product/lockfile/capability/vault touched). Verified: npm tarball integrity (sha1 matches packument), .node is ELF shared object exporting only napi_* (Node N-API v8), zvec-rust-sys build.rs resolution order (env > sibling > submodule > vendor > GitHub prebuilt download > cmake auto-build) and dylib link. No zvec-grep build/install/daemon/model download into product. Product gap confirmed: zero local search/index subsystem (CURRENT_PRODUCT grep).
CHECKPOINT=logs/research/M5-W18/A7-lane-checkpoint.md
MERGE_NOTES=feeds A10 (BOM: zvec-rust 0.7.0 + vendored libzvec_c_api, Apache-2.0 -> MulanPSL-2.0, NOTICE/attribution, build-size ~36MB native, hermetic-build decision); feeds A9 (product has NO workspace grant today -> must INTRODUCE local scope gate + copy remote-embedding HMAC design when embeddings enabled); feeds A8 (vector/hybrid slice deferred behind A8 retrieval + A9 embedding auth); feeds A11 (tests/search_index_fts.rs + caps/timeout/rebuild assertions into acceptance matrix). No conflicts with A4/A5/A6 (dbx), A2/A3 (Obsidian), A8 (retrieval), A9 (trust) — disjoint scopes.
NEXT=A10 must finalize the vendored-libzvec_c_api provenance + hermetic-build approach; A9 must define the local workspace-scope authorization gate (product currently has none); W19 first slice = FTS-only search_index_build/query/status (no watcher/daemon/model/network).
```

## Notes

- Executed in the dedicated W18 worktree `m5-w18-a7` (branch `codex/m5-w18-a7`), per `PARALLEL_COMMAND_BOARD.md` § "Dedicated lane worktrees" — the canonical master working copy was **not** modified.
- Research-only: no product-code edit, no dependency install into the product, no daemon/model/download/network beyond read-only npm/crates/GitHub metadata + tarball inspection in `/tmp/m5-w18-a7-zvec`.
- `HEAD` above is the R2 research commit `8ec92bbf…`; the checkpoint itself is committed in the same lane branch immediately after.
- R2 resolves both A0-named open items for A7: (1) `@zvec/zvec@0.7.0` native package/API/storage-format feasibility — RESOLVED (npm = Node N-API REJECT for Rust; engine adoptable via official `zvec-rust` 0.7.0 C-API); (2) watcher-free bounded explicitly-triggered first ingestion slice — DEFINED in `A7-zvec-grep-R2-evidence-closure.md` §6.
- Full source map, evidence table, claim tags (CURRENT_PRODUCT/REFERENCE_SOURCE/OBSERVED_BEHAVIOR/OFFICIAL_DOC/INFERENCE), R1 correction table, updated classification, and the first-slice blueprint (destination files, dependency closure, lifecycle, capacity, stable errors, tests, migration, rollback, hard stops) are in `A7-zvec-grep-R2-evidence-closure.md`.
