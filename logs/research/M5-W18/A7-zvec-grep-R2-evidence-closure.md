# Lane A7 · M5-W18-R2 — Evidence Closure: `@zvec/zvec@0.7.0` feasibility + watcher-free first ingestion slice

> STATUS: PASS
> Lane: A7 | Mode: RESEARCH ONLY（未改任何产品代码，未 push）
> Dispatch: `PARALLEL_COMMAND_BOARD.md` § "M5-W18-R2 Evidence Closure Dispatch" (A7 row line 1463)
> Audit: `logs/checkpoints/A0-M5-W18-R1-audit-20260908.md` (A7 verdict = `REWORK`, with two open items)
> Companion R1 report: `logs/research/M5-W18/A7-zvec-grep-ingestion-index-architecture.md`

This document closes the two A0-named open items for A7:

1. "the native zvec package/API/storage-format feasibility … remain unresolved" → resolved by direct inspection of the pinned npm tarball, its native binding, and the upstream crate/engine ecosystem.
2. "a watcher-free first slice remain unresolved" → defined in §6 with exact destination files/symbols, dependency closure, lifecycle, capacity, errors, tests, migration, rollback, hard stops.

All factual claims are tagged after the R2 common evidence contract:
`CURRENT_PRODUCT` (verified in this repo), `REFERENCE_SOURCE` (verified in the pinned upstream snapshot / npm / crates.io), `OBSERVED_BEHAVIOR` (verified by inspecting the actual downloaded artifact), `OFFICIAL_DOC` (upstream README / docs), `INFERENCE` (clearly labelled).

---

## 1. Reference evidence (exact artifacts inspected, with integrity)

All artifacts were downloaded read-only into the disposable workspace `/tmp/m5-w18-a7-zvec` (no product, manifest, lockfile, or capability was touched).

| # | Artifact | Source | Integrity / version | Evidence tag |
|---|---|---|---|---|
| E1 | `@zvec/zvec` 0.7.0 npm tarball | `https://registry.npmjs.org/@zvec/zvec/-/zvec-0.7.0.tgz` | sha1 `66b676c10177df9eb573225c6a9dbe5b73fd5e93`; sha512 `MT/M1CMn…fqMWmA==`; unpackedSize 72027; 9 files | REFERENCE_SOURCE / OBSERVED_BEHAVIOR |
| E2 | `@zvec/bindings-linux-x64` 0.7.0 | `…/@zvec/bindings-linux-x64/-/bindings-linux-x64-0.7.0.tgz` | sha1 `7f3045ffaae171f5539fd711af96a387b2cddd4c`; unpackedSize 41750454; `main = zvec_node_binding.node` | REFERENCE_SOURCE / OBSERVED_BEHAVIOR |
| E3 | zvec `zvec_node_binding.node` | extracted from E2 | `file`: ELF 64-bit LSB shared object, x86-64, dynamically linked, stripped; 36147600 bytes; exports `napi_*` (N-API) | OBSERVED_BEHAVIOR |
| E4 | `zvec-rust` 0.7.0 crate | `crates.io/crates/zvec-rust` | repo `github.com/zvec-ai/zvec-rust`; Apache-2.0; depends `zvec-rust-sys` 0.7.0; default feature `bundled` | REFERENCE_SOURCE |
| E5 | `zvec-rust-sys` 0.7.0 crate | `crates.io/crates/zvec-rust-sys` | `links = "zvec_c_api"`; `rustc-link-lib=dylib=zvec_c_api`; resolution order incl. download prebuilt / auto-build | REFERENCE_SOURCE / OBSERVED_BEHAVIOR |
| E6 | `alibaba/zvec` core engine | `api.github.com/repos/alibaba/zvec` | C++; Apache-2.0; 15842 stars; exposes C API (`libzvec_c_api`); prebuilt SDKs per release (Linux glibc/musl, macOS, Windows, Android, iOS) | REFERENCE_SOURCE / OFFICIAL_DOC |
| E7 | zvec-grep snapshot (R1) | `/home/ainfinit/Documents/极智简单/V3/research/zvec-grep-src` | HEAD `5265395` (0.2.1), pins `@zvec/zvec ^0.7.0` | REFERENCE_SOURCE |

Crates.io also returned two **unofficial/community** crates — `zvec` 0.1.0 (`oly-wan-kenobi/zvec-rs`) and `zvec-sys` 0.4.1 (`igobypenn/zvec-rust-binding`, builds C++ core v0.5.0 from a git clone). These are NOT used by the official toolchain and are excluded from the adoption path (see §5).

---

## 2. What `@zvec/zvec@0.7.0` actually is (resolves "native package/API")

- **It is a Node.js N-API addon, not a Rust crate.** `package.json` `main = src/index.js`, `exports` point to `.mjs`/`.js`; `devDependencies` include `node-addon-api ^8.9.0`. The wrapper `src/index.js` `require()`s `zvec_node_binding.node` (E3), which is an **ELF shared object exporting only `napi_*` symbols** → an N-API v8 addon (OBSERVED_BEHAVIOR, E3). N-API addons can only be loaded by a V8/Node-compatible runtime; **they cannot be linked into Tauri's Rust backend** without embedding a Node runtime (libnode). (REFERENCE_SOURCE + OBSERVED_BEHAVIOR)
- **Supported platforms (from `optionalDependencies` + `prebuilt.js`):** `darwin-arm64`, `linux-arm64`, `linux-arm64-musl`, `linux-x64`, `linux-x64-musl`, `win32-x64`. Notably **no `darwin-x64`, no `win32-arm64`, no 32-bit Linux**. (REFERENCE_SOURCE)
- **Install script does NOT phone home.** `scripts/install.js` only resolves the platform-specific optional package (`@zvec/bindings-*`, already fetched via npm dependency install) or detects a source checkout; it fails closed (`exit 1`) if no prebuilt is present. No network fetch at `npm install`. (OBSERVED_BEHAVIOR, E1)
- **Engine identity:** the npm package wraps the C++ core `alibaba/zvec` via its **C API** (`libzvec_c_api`). README (E6, line 68) confirms the official Rust binding is `cargo add zvec-rust`. So the same engine (v0.7.0) is reachable from Rust through the C API — **this directly resolves the R1 open item "is there a Rust API or only Node?"**: there is a dedicated official Rust crate `zvec-rust`; the npm package itself is Node-only. (OFFICIAL_DOC + REFERENCE_SOURCE)

### Verdict on "API / format compatibility"

- **N-API npm package ↔ Rust backend: REJECT** (Node runtime binding, not linkable).
- **Engine (C API) ↔ Rust backend via `zvec-rust`: PROVE / ADOPT** (see §3–§5).
- **On-disk `.zvec` storage-format compatibility:** `@zvec/zvec` 0.7.0 (npm) and `zvec-rust` 0.7.0 (Rust) both bind engine v0.7.0, so `files.zvec` / `index.zvec` written by either are the **same engine-version format** → indexes are interchangeable between zvec-grep and a future Rust product indexer **iff the engine version is pinned to the same release**. No public "stable-forever format" guarantee was found in the README/docs; therefore adoption MUST pin the engine version and require an explicit `rebuild` on any version change (mirrors zvec-grep's `input.rebuild` gate). This is an **accepted invariant**, not a blocker. (INFERENCE, bounded by E4/E6/E7)

---

## 3. The official Rust path: `zvec-rust` 0.7.0

- `zvec-rust` 0.7.0 (repo `zvec-ai/zvec-rust`, Apache-2.0, 863 downloads) depends on `zvec-rust-sys` 0.7.0 and **defaults to the `bundled` feature** → `zvec-rust-sys/bundled`. (REFERENCE_SOURCE, E4)
- `zvec-rust-sys` (E5) links `dylib=zvec_c_api` (a **dynamic** library) and resolves `libzvec_c_api` via, in order: env `ZVEC_LIB_DIR`/`ZVEC_INCLUDE_DIR` → sibling `../zvec` → git submodule `vendor/zvec` → vendored `vendor/` → **download prebuilt from GitHub Release (`zvec-ai/zvec-rust`)** → **auto-build (clone `alibaba/zvec`, build with CMake)**. (OBSERVED_BEHAVIOR, E5)
- **Key build-repro finding:** `zvec-rust` does NOT transparently vendor a static lib. A successful hermetic build requires **`libzvec_c_api` to be supplied** (vendored prebuilt or built locally). `bundled` here means "the sys crate will *fetch or build* the C++ engine", which implies **network + a C++ toolchain (cmake, gcc/clang, C++17) at build time** unless `ZVEC_LIB_DIR` points at a pre-vendored `libzvec_c_api`. (INFERENCE from E5 + OBSERVED_BEHAVIOR)

**Adoption implication (for A10 BOM):** the product must ship `libzvec_c_api.so/.dylib/.dll` (Apache-2.0, from `alibaba/zvec` release artifacts) inside the Tauri bundle (the crate sets rpath), retain its NOTICE/attribution, and either (a) vendor the prebuilt into the repo, or (b) build the C++ engine in CI with cmake. Option (a) is preferred for reproducible offline builds. (INFERENCE)

---

## 4. Correction of R1 claims (explicit, history preserved)

Per R2 common contract §4, every A0-named R1 statement is retracted/replaced here. The R1 report body is **not** silently rewritten; this section supersedes it.

| R1 statement | R2 correction | Tag |
|---|---|---|
| §8 unresolved #1: "Is `@zvec/zvec` a native (napi/neon) addon with a Rust API usable from Tauri, or Node-only? A10 must resolve." | The npm package is **Node-only N-API** (E1/E3) — not linkable into Rust. But the engine HAS an official Rust crate `zvec-rust` 0.7.0 (E4) binding the same C API. So the right answer is: **adopt via `zvec-rust`, not the npm package.** Resolved by A7; A10 should record `zvec-rust` 0.7.0 (+ vendored `libzvec_c_api`) as the COPY/ADAPT unit. | REFERENCE_SOURCE |
| §8 unresolved #2: "DEFAULT_IGNORE contents should be read in full when porting" | Still valid as a port task, but it is now a **behavior-reimplementation** detail (use the Rust `ignore` crate to honor `.gitignore` + ripgrep default ignores). Not a blocker. | INFERENCE |
| §4 / §8 #3: "product's existing 'workspace authorization' concept must be reconciled with zvec-grep config-trusted RootPath" | **Correction:** the product has **no** OS-level workspace-grant model. `src-tauri/src/workspace.rs` only resolves app data dirs (`data_dir`, `workspace_dir`, `scripts_dir`, `images_dir`, `notes_dir`…) — there is no grant/authorization gate. (CURRENT_PRODUCT) Therefore adoption must **introduce** a workspace-scope authorization gate (reusing zvec-grep's HMAC `RemoteEmbeddingWorkspaceGrant` *design* for remote embedding only, and a local explicit "index these roots" consent for the local scope), not "reconcile" with an existing one. Reframed finding for A9. | CURRENT_PRODUCT |
| §1 / §5: "zvec engines are Apache-2.0 or MIT → adoption license-feasible" | The **product** root `LICENSE` is **木兰宽松许可证第2版 (MulanPSL-2.0)** (CURRENT_PRODUCT, `head -3 LICENSE`). Inbound `@zvec/zvec`, `zvec-rust`, `alibaba/zvec` are Apache-2.0. Apache-2.0 → MulanPSL-2.0 inbound is permissible but must be recorded by A10 with NOTICE/attribution and modified-file headers. A7's R1 wording "license-feasible" stands for the *engines* but must not be read as "product is Apache-2.0" (that was A8's false statement, not A7's). | CURRENT_PRODUCT + REFERENCE_SOURCE |
| §6 classification: `@zvec/zvec` = "COPY / ADAPT (verify napi vs sidecar)" | Updated: the npm package is **REJECT** for direct Rust use; the engine via `zvec-rust` (C API, dynamic lib) is **ADAPT** (Rust FFI + vendored/built `libzvec_c_api`, not a drop-in static COPY). See §5. | REFERENCE_SOURCE |

---

## 5. Updated classification with function-level closure (COPY items now carry dependencies + destination)

| Unit | Class | Function-level closure / deps | Destination (product) | Tag |
|---|---|---|---|---|
| `libzvec_c_api` (engine v0.7.0, Apache-2.0) | ADAPT | vendored `libzvec_c_api.so/.dylib/.dylib` + headers from `alibaba/zvec` release; linked via `zvec-rust-sys` (`links="zvec_c_api"`, dylib) | shipped in Tauri bundle (rpath set by crate) | REFERENCE_SOURCE |
| `zvec-rust` 0.7.0 crate (safe wrapper) | ADAPT | dep `zvec-rust-sys` 0.7.0; default `bundled`; needs `libzvec_c_api` supplied | `src-tauri/Cargo.toml` `zvec-rust = "0.7.0"` (pin exact `=0.7.0`) | REFERENCE_SOURCE |
| ripgrep discovery behavior | REIMPLEMENT_FROM_BEHAVIOR | use Rust `ignore` crate (ripgrep's, MIT/Apache) for `.gitignore` + default-ignore walking; do not port TS | `src-tauri/src/search_index.rs` | INFERENCE |
| Extractors/chunking (3600 chars / 15% overlap) | REIMPLEMENT_FROM_BEHAVIOR | Rust line/byte-window chunking for first FTS slice; tree-sitter structural chunking deferred | `src-tauri/src/search_index.rs` | INFERENCE |
| Ignore rules / RootPath scope (config-trusted) | REIMPLEMENT_FROM_BEHAVIOR | mirror `RootPath { exclude, globs, ignore_files, hidden, max_depth }` | `src-tauri/src/search_index.rs` | INFERENCE |
| Remote-embedding HMAC grant (design only) | COPY (pattern) | Rust reimplementation of `RemoteEmbeddingWorkspaceGrant`/`signature` | later slice, gated by A9 | REFERENCE_SOURCE |
| Watcher / ChangeSet / reconcile | DEFER | out of first slice (see §6) | W19 later wave | INFERENCE |
| CLI / daemon / MCP server | REJECT (first slice) | not required; A9/A11 decide exposure | — | INFERENCE |

**Net change from R1:** the engine is adoptable **natively in Rust via `zvec-rust`**, eliminating the Node-sidecar fallback as the *required* path (it remains an optional alternative). The npm `@zvec/zvec` package is explicitly **not** used by the Rust product.

---

## 6. Watcher-free, bounded, explicitly-triggered FIRST ingestion slice

This slice is intentionally minimal: **FTS-only** (no embedding model, no network, no daemon, no watcher). It proves the storage/index pipeline end-to-end and unblocks later hybrid/vector slices (gated by A8 retrieval + A9 embedding authorization).

### 6.1 Destination files / symbols (exact, current-product mapping)

| File | Action | Notes |
|---|---|---|
| `src-tauri/src/search_index.rs` | NEW module | types + 3 commands; add `mod search_index;` in `lib.rs`/`main.rs` |
| `src-tauri/src/main.rs` | EDIT (registration) | add `search_index_build`, `search_index_query`, `search_index_status` to `generate_handler!` |
| `src-tauri/permissions/default-commands.toml` | EDIT (ACL) | insert the 3 command names **before the final `list_artifact_images` line** (invariant from A6/A7 prior work) |
| `src-tauri/capabilities/default.json` | EDIT (capability) | add the 3 permissions with explicit scope |
| `src/bridge.ts` + `src/types.ts` | EDIT (typed exposure) | `searchIndexBuild(cfg)`, `searchIndexQuery(...)`, `SearchIndexConfig`, `RootPath`, `SearchHit`, `IngestReport` |
| `src-tauri/Cargo.toml` | EDIT (deps) | `zvec-rust = "=0.7.0"`; `ignore = "0.4"`; ensure cmake + C++ toolchain in build env; vendor `libzvec_c_api` |
| `src-tauri/tests/search_index_fts.rs` | NEW test | synthetic corpus, build, query known passage, caps enforced, no-watcher asserted |
| `scripts/check-search-index-policy.py` (optional) | NEW gate | reuse graph/database policy-script pattern (29 Python checks exist) |

### 6.2 Proposed API (Rust sketch, not committed)

```rust
// src-tauri/src/search_index.rs
pub struct RootPath {
    pub path: PathBuf,
    pub exclude: Vec<String>,
    pub globs: Vec<String>,
    pub ignore_files: Vec<String>,
    pub hidden: bool,
    pub max_depth: Option<usize>,
}
pub struct SearchIndexConfig { pub roots: Vec<RootPath>, pub force_rebuild: bool }
pub struct IngestReport { pub scanned: usize, pub indexed: usize, pub skipped: usize, pub errors: Vec<String>, pub duration_ms: u64 }
pub struct SearchHit { pub id: String, pub score: f32, pub fragment: String, pub source_path: PathBuf, pub line: u32 }

#[tauri::command]
pub fn search_index_build(cfg: SearchIndexConfig) -> Result<IngestReport, String> { /* explicit, bounded, no watcher */ }
#[tauri::command]
pub fn search_index_query(scope_root: PathBuf, query: String, top_k: Option<u32>) -> Result<Vec<SearchHit>, String> { /* FTS via zvec-rust */ }
#[tauri::command]
pub fn search_index_status(scope_root: PathBuf) -> Result<IndexStatus, String> { /* manifest read, no write */ }
```

### 6.3 Data flow / lifecycle

1. `search_index_build(cfg)` is **user-/command-triggered only** (no FS watcher). It honors `force_rebuild` to wipe `<root>/.zvec-grep/` and re-index; otherwise it is a no-op if a valid manifest exists (later incremental slices add ChangeSet, not this one).
2. Discovery: walk each `RootPath` with the `ignore` crate (`.gitignore` + ripgrep defaults + `exclude`/`globs`/`ignore_files`/`hidden`/`max_depth`).
3. Chunking: line/byte-window at `CHUNK_CHARS=3600` / `CHUNK_OVERLAP_PCT=15` (mirrors zvec-grep constants).
4. Index: insert chunks into a `zvec-rust` collection with **FTS field only** (no vector field) → `files.zvec` + `index.zvec` written under `<root>/.zvec-grep/`. zvec WAL guarantees crash-safety; single-process write lock is inherent.
5. Query: `search_index_query` opens the collection read-only and runs FTS; returns top-`DEFAULT_TOP_K=7` hits (zvec-grep `DEFAULT_LIMIT=7`).
6. No embedding step, no remote call, no background worker → satisfies "watcher-free, bounded, explicitly triggered".

### 6.4 Capacity constants (first slice)

`MAX_FILES = 5_000`, `MAX_BYTES_PER_FILE = 2 MiB`, `MAX_TOTAL_BYTES = 256 MiB`, `MAX_DEPTH = 20`, `CHUNK_CHARS = 3600`, `CHUNK_OVERLAP_PCT = 15`, `DEFAULT_TOP_K = 7`, build `TIMEOUT_MS = 60_000`. Over-cap → `SEARCH_INDEX_OVER_CAPACITY` error, partial index not committed.

### 6.5 Stable errors

Product codes (returned as `Err(String)`), zvec-native errors surfaced with their `ZVEC_*` prefix: `SEARCH_INDEX_INVALID_ROOT`, `SEARCH_INDEX_OVER_CAPACITY`, `SEARCH_INDEX_BUILD_TIMEOUT`, `SEARCH_INDEX_OPEN_FAILED`, `SEARCH_INDEX_REMOTE_EMBEDDING_DISABLED` (reject any embedding request in this slice), `SEARCH_INDEX_REBUILD_REQUIRED` (engine version mismatch).

### 6.6 Tests (synthetic, no secrets)

`tests/search_index_fts.rs`: create temp dir with N known files (including a file containing a unique passage); `search_index_build` with caps; assert query for the unique passage returns that file; assert `MAX_FILES` cap rejects an over-large corpus; assert no watcher thread/file is spawned (process has no inotify handle open after return). Reuse `zvec-rust` FTS test pattern. Plus a `cargo test` target in the new module.

### 6.7 Migration / rollback

- Indexes live under `<root>/.zvec-grep/` (separate from any product data). Engine-version-pinned: on any `zvec-rust` version bump, require `force_rebuild=true` (no auto-migration).
- Rollback = remove the 3 commands + ACL + capability + `cargo remove zvec-rust` + delete `search_index.rs`. Purely additive; no existing product schema touched.

### 6.8 Hard stops (for the W19 implementer)

1. **No daemon / FS watcher** in this slice.
2. **No remote embedding, no model download** — FTS-only; vector/hybrid deferred behind A8/A9 gates.
3. **Pin engine version** (`zvec-rust = "=0.7.0"`); never auto-upgrade; require explicit `rebuild` on version change.
4. **Hermetic build**: supply `libzvec_c_api` via `ZVEC_LIB_DIR` (vendored prebuilt) — do NOT rely on the crate's GitHub fetch/clone at build time in CI. Record provenance + NOTICE (A10).
5. **Atomic delivery**: every new command needs source-check + ACL + capability + typed `bridge.ts`/`types.ts` + policy/test (per `WORKSPACE_IDENTITY.md` Desktop Runtime Source Gate).
6. Respect the ACL invariant: new commands **before** the final `list_artifact_images` line.

---

## 7. Residual risk / hand-offs

- **A9**: local workspace-scope authorization must be *introduced* (product has none today) and the remote-embedding HMAC grant *copied* when embeddings are enabled later. (corrected from R1)
- **A10**: record `zvec-rust` 0.7.0 + vendored `libzvec_c_api` (Apache-2.0) into MulanPSL-2.0 product with NOTICE/attribution; decide vendored-prebuilt vs CI-build for `libzvec_c_api`; confirm build-size impact (engine ~36 MB native in the Node build; Rust build similar).
- **A8**: later hybrid/vector slice depends on A8's retrieval-route evidence + a local/remote embedding backend (gated by A9).
- **A11**: fold `tests/search_index_fts.rs` + capacity/timeout/rebuild assertions into the W19 acceptance matrix.
- **Engine format stability**: treated as version-pinned invariant; no cross-version guarantee found. Non-blocker.

## 8. Verification performed (this R2)

- `git fetch origin` + worktree clean + `git rebase origin/master` (already up to date at base `78d2cfb`, lane `codex/m5-w18-a7`, ahead 2).
- npm packument + tarball inspection (`@zvec/zvec` 0.7.0), `file`/`nm`/`strings` on `zvec_node_binding.node`, bindings metadata, crates.io queries (`zvec-rust`, `zvec-rust-sys`, community `zvec`/`zvec-sys`), GitHub API for `alibaba/zvec` and `zvec-ai/zvec-rust`, and source inspection of `zvec-rust-sys` `build.rs`/`Cargo.toml`.
- Current-product grep: confirmed `LICENSE` = MulanPSL-2.0, `workspace.rs` has no grant model, no zvec/search/tantivy/cmake in `Cargo.toml`, ACL last line is `list_artifact_images`.
- No product code, manifest, lockfile, capability, or vault was modified. No dependency installed into the product. No daemon/model/network beyond the read-only npm/crates/GitHub metadata + tarball downloads into `/tmp/m5-w18-a7-zvec`.
