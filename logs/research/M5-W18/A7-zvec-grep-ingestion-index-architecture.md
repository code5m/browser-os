# Lane A7 · M5-W18-R — zvec-grep ingestion/index architecture（逆向研究）

> STATUS: PASS
> Lane: A7 | Mode: RESEARCH ONLY（未改任何产品代码，未 push）
> Reference: `zvec-ai/zvec-grep` pinned @ `52653951b24617762f4ab0c71c34d594e5001617` (Apache-2.0)
> Local snapshot: `/home/ainfinit/Documents/极智简单/V3/research/zvec-grep-src` (HEAD `5265395`, version `0.2.1`, LICENSE = Apache-2.0 confirmed)

This report reverse-engineers **how zvec-grep ingests and indexes a workspace** — the exact scope of Lane A7. It is evidence for A10 (transplant ledger) and A11 (verification architecture); it does **not** constitute product code, and every proposed adoption is classified `COPY`/`ADAPT`/`REIMPLEMENT_FROM_BEHAVIOR`/`DEFER`/`REJECT`.

---

## 1. Current-product gap（当前产品缺口）

Read-only grep across `src/`, `src-tauri/`, `scripts/` of the canonical product (`mvp-browser-os-v3`) finds **no local hybrid (ripgrep + BM25 + vector) search/index subsystem**:

- The only `query`/`search` references are `src-tauri/src/database.rs` (`pub fn query`, `query_sqlite_with_deadline` — SQLite data queries) and `src-tauri/src/mcp.rs` (`tab_query`, `history_query`, `bookmarks_query`, `downloads_query`, `console_query` — browser-context queries, not file/document retrieval).
- There is **no ripgrep/BM25/vector ingestion, no workspace index, no embedding pipeline** in the product today.

**Gap conclusion:** adopting zvec-grep would be a *net-new* capability (the W18 "search/replication blueprint" target). The product's existing query surface is unrelated and should not be conflated with workspace-file retrieval.

**Critical architecture mismatch (must be surfaced to A0/A10):** zvec-grep is a **Node.js ≥22 / TypeScript** project (CLI + daemon + MCP server). The product's desktop runtime is **Rust/Tauri**. Therefore zvec-grep's *orchestration* TypeScript is not directly linkable into the Rust backend; the reusable Apache-2.0 *engines* (native vector store, ripgrep core, tree-sitter grammars, local embedding runtime) are, and must be the unit of adoption (see §8–§9).

---

## 2. Upstream source map（按 A7 子主题）

### 2.1 Workspace authorization — `src/authorization/*`
- **Symbols:** `REMOTE_EMBEDDING_CAPABILITY = "remote_embedding"`; `RemoteEmbeddingAuthorizationScope = "once" | "workspace"`; `RemoteEmbeddingOperation = "query" | "index" | "query_and_index"`; `RemoteEmbeddingDataDisclosure { queryText, workspaceContent: "none"|"selected"|"changed"|"full" }`; `RemoteEmbeddingTarget { workspaceRoots, workspaceFingerprint, provider, model, endpoint, targetFingerprint }`; `RemoteEmbeddingWorkspaceGrant` (version:1, scope:"workspace", workspaceRoots, workspaceFingerprint, provider, model, endpoint, targetFingerprint, grantedAt, `signature`); `RemoteEmbeddingAuthorizationPlan`; `RemoteEmbeddingOperationPermit`.
- **Store/manager:** `RemoteEmbeddingAuthorizationStore` (persists a **sha256-HMAC-signed** `<root>/.zvec-grep/authorization.json`); `RemoteEmbeddingAuthorizationManager.grant()`; `remoteEmbeddingAuthorizationGuard()` (per-request check via `AsyncLocalStorage`-injected `RemoteEmbeddingOperationPermit`); `planRemoteIndexAuthorization` / `planRemoteSearchAuthorization`; `createRemoteEmbeddingTarget` / `workspaceFingerprint`.
- **Flow:** a workspace path is *indexed/searched* only after a signed `RemoteEmbeddingWorkspaceGrant` exists for its fingerprint; `hasGrant()` verifies signature + `targetFingerprint` before any remote embed.
- **Important nuance:** the authorization system gates **remote (egress) embedding only**. Local workspace-path scoping (which roots get indexed) is **config-trusted**, not permission-gated — see §2.2. This split is essential for A9's threat model.
- **Tests:** `test/authorization.test.mjs`, `test/unit/authorization-prompt.test.mjs`.

### 2.2 Discovery / ignore rules — `src/engine/pipeline/indexing/scanner/index.ts` (+ `engine/utils/file-selection.ts`, `engine/utils/glob.ts`, `engine/manifest.ts`)
- **Symbols:** `scanRootPaths` / `scanRootPath` / `scanDirectoryPath` / `scanFilePath`; `pathCanAffectIndex` (watcher→scanner filter gate); `matchesFileSelection` + `resolveFileTypePatterns` (ripgrep type globs); `ripgrepGlobMatches` / `ripgrepGlobMatchesCaseInsensitive`; `DEFAULT_IGNORE_RULES: readonly IgnoreRule[]` (scanner/index.ts:137) = `DEFAULT_IGNORED_DIRECTORY_NAMES` + `DEFAULT_IGNORED_FILE_PATTERNS`; `MAX_GITIGNORE_CACHE_ENTRIES = 4096`; `RootPath` fields `exclude`/`globs`/`ignoreFiles`/`noIgnore`/`hidden`/`maxDepth`.
- **Flow:** recursively walk each `RootPath`; apply (in order) `DEFAULT_IGNORE_RULES` (`.git`, `.zvec-grep`, `node_modules`, hidden files unless `hidden:true`, plus ignored file patterns), then `.gitignore`/`.zgignore` (`ignoreFiles`), then manifest `exclude`/`globs`, then ripgrep file-type filtering → candidate `FileInfo[]`.
- **Tests:** `test/unit/scanner-utils.test.mjs`, `test/path-indexing.test.mjs`.

### 2.3 Extractors / chunking — `src/engine/extraction/*`
- **Symbols:** `extract` / `extractForIndexing` (route by kind → code/image/markdown/text); `CodeExtractor.extractForIndexing` + `walkCodeNode` + `codeEntityToSearchFragments`; `DEFAULT_CODE_CHUNK_CHARS = 3600` (code/extractor.ts:21) with overlap `540` (=15% of chunk, see §6); `ChunkOptions { maxChunkChars, chunkOverlapChars }`; `LanguageAdapter` interface + per-language adapters (`c-family`, `js-ts`, `c`/`cpp`/`go`/`java`/`javascript`/`python`/`rust`/`typescript`); tree-sitter glue (`grammar.ts`/`nodes.ts`/`parser.ts`); `EntityFragment` (carries optional embedding text + source location).
- **Flow:** a `Source` is routed to a kind-specific extractor; code uses tree-sitter + language adapter to walk named entities and emit `EntityFragment`s; text/markdown/image extractors produce analogous fragments. Chunk size is budget-derived from `maxInputTokens` (§6).
- **Tests:** `test/unit/extraction/code.test.mjs`, `test/unit/extraction/core.test.mjs`.

### 2.4 Manifest / files / index formats — `src/engine/storage/*` (+ `engine/manifest.ts`)
- **Symbols:** `WorkspaceManifest` + `CURRENT_MANIFEST_VERSION` (v1; validated by `isWorkspaceManifest`); `resolveWorkspaceIndexStoragePaths` → `files.zvec` + `index.zvec` (`layout.ts`: `FILES_ZVEC="files.zvec"`, `ENTITIES_ZVEC="index.zvec"`); `WorkspaceIndexStorage` interface (`replaceFile` / `deleteFile` / `searchFts` / `searchVector` / `finalizeWrites`); `createWorkspaceIndexStorage`; `validateFragmentGroups` (write-time integrity check in `zvec.ts`).
- **On-disk layout (`<root>/.zvec-grep/`):** `manifest.json` (root paths, embedding schema, indexVersion, updatedTime, embeddingRuntime) + `files.zvec` (`FileInfo` records) + `index.zvec` (entities + fragments + vectors). Serialization is **custom binary via `@zvec/zvec`** (`zvec.ts`), not JSON/text.
- **Tests:** `test/unit/zvec-storage.test.mjs`, `test/integration/manifest.test.mjs`.

### 2.5 Incremental update — `src/daemon/change-set.ts` (+ `engine/pipeline/indexing/index.ts`, `scanner/index.ts`, `daemon/backend.ts`)
- **Symbols:** `ChangeSet` (`add` / `merge` / `snapshot` / `forceFullReconcile`; budget `maxChangedPaths = 1000`); `ChangeKind`; `indexWorkspace` / `indexWorkspacePaths`; `scanFilePath` classifies `added`/`modified`/`unchanged` using `info.mtimeMs` (`lastModifiedTime`) + `contentHash` (`sha256Text`); `replaceFile` / `deleteFile` apply deltas.
- **Flow:** a `ChangeSetSnapshot { touchedFiles, rescanDirectories, deletedPrefixes }` drives re-hashing (mtime+contentHash); only changed files are re-extracted and `replaceFile`'d. `enforcePathBudget` promotes leaf paths to parent dirs when over budget; still over → `forceFullReconcile`.
- **Tests:** `test/change-set.test.mjs`, `test/daemon-backend.test.mjs`.

### 2.6 Watcher reconciliation — `src/daemon/watch-manager.ts`
- **Symbols:** `WatchManager` (`start` / `close` / `flushPending` / `refreshPaths` / `checkForResume`); `queueRecord` / `recordPath` / `watchDirectoryTree`; `requiresDirectoryWatchers` (Linux → per-directory inotify, **recursive off**); debounce `debounceMs = 750` (default), `maxWaitMs = 5000`; `reconcileIntervalMs = 60 * 60_000` (60 min full reconcile); `checkForResume` threshold `90s`; add-watcher error recovery with exponential backoff + `queueFullReconcile`.
- **Flow:** FS events are recorded into a `ChangeSet` (skipping `.git`/`.zvec-grep`), deduped/collapsed, debounced, then flushed as a snapshot with reason `"watch"`; timeouts/overflow → reason `"reconcile"` (full). Output feeds `IndexCoordinator` → `JobScheduler`.
- **Tests:** `test/watch-manager.test.mjs`, `test/change-set.test.mjs`.

### 2.7 Corruption / rebuild
- **Symbols:** `readWorkspaceManifest` throws `ZVEC_GREP.ENGINE.MANIFEST.INVALID` (manifest.ts); `isWorkspaceManifest` shape validation; `validateFragmentGroups` (zvec.ts); `forceFullReconcile` (change-set.ts); backend `input.rebuild` gate (`backend.ts`) blocks model/endpoint mismatch (must `rebuild` to change embeddings).
- **Flow:** corruption is detected **at load** (manifest shape validation / fragment-group validation); rebuild is triggered by explicit `rebuild` flag or `forceFullReconcile` (watcher overflow/error/resume timeout), then `indexWorkspace` re-indexes from scratch.
- **Tests:** `test/integration/manifest.test.mjs`, `test/daemon-backend.test.mjs`.

### 2.8 Concurrency / lifecycle
- **Symbols:** `JobScheduler` (`queue` / `sortQueue` / `enqueue` / `mergeQueuedJob`; `JobReason` priority); `RootRuntime` (`activeOperations`, `activeWriterSearches`, `writerPending`, `generation`/`generationTail` revision gating, epoch `fullReconciliationEpoch`); `acquireReadWriteLock` (`engine/utils/lock.ts`); `WorkspaceIndex.embeddingConcurrency`; backend `Promise.all(...)` for parallel roots + `embeddingConcurrency` option.
- **Flow:** index jobs are queued/prioritized in `JobScheduler`, dispatched per-root by `RootRuntime` which serializes writers via revision epochs and bounds parallel embedding via `embeddingConcurrency`; storage writes are guarded by a read-write lock.
- **Tests:** `test/job-scheduler.test.mjs`, `test/index-coordinator.test.mjs`.

### 2.9 Performance / capacity（常量表，已核实）
| 常量 | 值 | 位置 |
|---|---|---|
| `DEFAULT_CODE_CHUNK_CHARS` | 3600 | `extraction/code/extractor.ts:21` |
| chunk overlap | 15% (`CHUNK_OVERLAP_PERCENT`) | `pipeline/indexing/input-budget.ts:8` |
| `DEFAULT_CHARS_PER_100_TOKENS` | 185 | `input-budget.ts:3` |
| local model2vec | `maxBatchSize 256`, `maxInputTokens 1024`, `defaultConcurrency 2` | model catalog/backends |
| `ZVEC_UPSERT_BATCH_SIZE` | 1024 | `storage/zvec.ts:73` |
| `ZVEC_MAX_QUERY_TOPK` | 100_000 | `storage/zvec.ts:74` |
| `MAX_GITIGNORE_CACHE_ENTRIES` | 4096 | `scanner/index.ts:47` |
| `MAX_SKIPPED_FILE_SAMPLES` | 20 | `storage/zvec.ts` |
| `DEFAULT_LIMIT` (search) | 7 | `pipeline/search/index.ts:76` |
| `RRF_K` | 60 | `pipeline/search/index.ts:77` |
| `CONTEXT_GROUP_RRF_K` | 60 | `engine/service/zvec-grep.ts:1845` |
| `RECALL_INITIAL_DEPTH` / `RECALL_MAX_DEPTH` / `RECALL_GROWTH_FACTOR` / `RECALL_TARGET_FACTOR` / `RECALL_MIN_TARGET_CANDIDATES` | 200 / 2000 / 2 / 5 / 50 | `pipeline/search/index.ts` |
| `maxChangedPaths` (ChangeSet budget) | 1000 | `daemon/change-set.ts` |
| watcher debounce / maxWait / reconcile / resume | 750ms / 5000ms / 60min / 90s | `daemon/watch-manager.ts` |

### 2.10 Compact output — `src/cli/format/*`
- **Symbols:** `dedupeAndRerankContextItems()` / `contextItemDedupeKey()` / `dedupeContextItems` (`engine/service/zvec-grep.ts`); `truncate()` (`context.ts:1133`); `HUMAN_PREVIEW_MAX_LINE_LENGTH`; `status.ts` `compact` table; `SearchHit.matchedBy: "fts"|"vector"|"fts+vector"` (`engine/types.ts`).
- **Flow:** recall lists from vector+FTS fused via RRF (`RRF_K=60`), cross-group dedupe, then `slice(0, limit)`. Ingestion reporting reuses the same status/diagnostics surfaces (`WorkspaceIndexStatus` / `AuthorizationStatus`).
- **Tests:** `test/unit/cli-format.test.mjs`, `test/unit/search.test.mjs`, `test/mcp.test.mjs`.

---

## 3. End-to-end ingestion data flow

```
[授权] 远程嵌入：planner → RemoteEmbeddingAuthorizationManager.grant
       → .zvec-grep/authorization.json (sha256-HMAC 签名)
       → AsyncLocalStorage 注入 RemoteEmbeddingOperationPermit
       → remoteEmbeddingAuthorizationGuard 在每个 embed 处校验
[入口] CLI `zg index` (cli/commands.ts) 或 daemon DaemonBackend.index (daemon/backend.ts)
       → ZvecGrepService.index (engine/service/zvec-grep.ts) 持 home 写锁
       → 读/准备 manifest；rebuild||!indexed → resetWorkspaceIndex；WorkspaceIndex(mode:write)
[工作区路径] 来自 manifest.rootPaths / RootPath 配置（发现阶段信任配置范围，非权限门）
[发现] scanRootPaths/walk → DEFAULT_IGNORE_RULES + .gitignore + ignoreFiles
       + exclude/globs + ripgrep fileTypes → FileInfo[]（按 size/mtime/hash/类型/二进制过滤）
[抽取/分块] extraction/runtime.ts extractForIndexing → text/markdown/code/image
       → EntityFragment[]（chunkOptions 由 maxInputTokens 经 input-budget 换算）
[嵌入/索引] pipeline/indexing/index.ts indexFiles → computeDiffFromFiles(size+mtime+hash)
       → 增量仅处理 added/modified/pending；AdaptiveEmbeddingScheduler 并发 embed
       → commitFile → WorkspaceIndexStorage.replaceFile (zvec.ts) 批量 upsert 片段+向量
[manifest 更新] IndexResult → 写回 manifest.json (updatedTime/embeddingRuntime)
[watcher 协调] WatchManager → ChangeSet → debounce → flush
       → IndexCoordinator.enqueue → JobScheduler.submit → runIndex(changedPaths|forceFullReconcile)
       → 重复 [发现→抽取→嵌入→提交]，推进 RootRuntime 修订号 / 最终状态扫描判定 reconciled
```

---

## 4. Security / privacy notes（供 A9 / A11）

- **Remote-embedding authorization is the only data-egress gate.** It is cryptographically signed (HMAC), scoped (`once`/`workspace`), operation-tagged (`query`/`index`/`query_and_index`), and discloses exactly `workspaceContent: none|selected|changed|full` + `queryText`. This is a strong pattern to **copy** into the product's future remote-embedding integration.
- **Local workspace-path scoping is config-trusted, not authorized.** Indexing scope = `RootPath` config (include/exclude/ignoreFiles/globs). In a desktop product this must be reconciled with the OS-level workspace grant model (the product's existing "workspace authorization" concept) — otherwise a user could be indexed into a scope they did not intend. **Open item for A9.**
- **Index is local-only** by default (`.zvec-grep/` under the indexed root); no network unless remote embedding is explicitly granted.

---

## 5. Dependencies / licenses（供 A10 BOM）

| Dependency | Role in ingestion/index | License |
|---|---|---|
| `@zvec/zvec` ^0.7.0 | vector storage/index engine (`files.zvec`/`index.zvec` binary) | Apache-2.0 |
| `@vscode/ripgrep` ^1.18.0 | file discovery / grep / file-type globs (ripgrep core is Rust) | MIT/Apache-2.0 (ripgrep) |
| `tree-sitter-wasms` ^0.1.13 + `web-tree-sitter` ^0.20.8 | code structure extraction | tree-sitter grammars mostly MIT |
| `@huggingface/transformers` ^3.8.1 + `@huggingface/tokenizers` ^0.1.3 | local embedding models (ONNX/WASM) | Apache-2.0 (transformers.js) |
| `jsonc-parser` ^3.3.1 | `.gitignore`/config parsing | MIT |
| `zod` 4.2.0 | config/schema validation | MIT |
| `node-llama-cpp` 3.18.1 (optional) | alternative local embedding backend | MIT |
| TypeScript orchestration (`src/authorization`, `extraction`, `pipeline`, `daemon`, `storage`) | the logic under study | Apache-2.0 (upstream zvec-grep) |

**Licensing takeaway:** all core engines are Apache-2.0 or MIT → adoption is license-feasible, **provided** A10 records provenance, preserves NOTICE/attribution, and adds modified-file headers where adapted.

---

## 6. Target architecture mapping（Rust/Tauri product）& classification

Because the product backend is Rust/Tauri, classify each zvec-grep unit as follows:

| Unit | Classification | Rationale / adaptation |
|---|---|---|
| `@zvec/zvec` vector engine | **COPY / ADAPT** | Native engine; most directly reusable. Verify whether it exposes a Rust API (napi/neon) usable from the Tauri Rust side, or must run as a sidecar. |
| `@vscode/ripgrep` / ripgrep core | **COPY** | Rust crate usable directly for discovery/grep; the TS wrapper is not needed. |
| tree-sitter grammars (code extraction) | **COPY** | Grammars are MIT; reuse for structural chunking in Rust. |
| `@huggingface/transformers` (local embeddings) | **ADAPT** | WASM/Node runtime; in Rust product prefer a native ONNX backend or a managed sidecar. |
| Ignore rules (`DEFAULT_IGNORE_RULES` + `.gitignore`/`.zgignore`) | **REIMPLEMENT_FROM_BEHAVIOR** | Recreate the behavior (default dirs + gitignore parsing) in Rust; do not port TS. |
| Extractors/chunking (`extractForIndexing`, `CodeExtractor`, `ChunkOptions`) | **REIMPLEMENT_FROM_BEHAVIOR** | Port the *behavior* (entity-walk + 3600-char/15%-overlap chunking) to Rust; reuse tree-sitter. |
| Manifest/storage format (`manifest.json` + `files.zvec`/`index.zvec`) | **ADAPT** | Keep the on-disk contract compatible (so indexes interop), but reimplement binary I/O via `@zvec/zvec` native API. |
| Incremental update (`ChangeSet`, mtime+`sha256Text`) | **REIMPLEMENT_FROM_BEHAVIOR** | Port the diff algorithm + 1000-path budget + `forceFullReconcile` escalation. |
| Watcher reconciliation (`WatchManager`, debounce 750/5000, 60-min reconcile, 90s resume) | **REIMPLEMENT_FROM_BEHAVIOR** | Port to `notify` (Rust) with equivalent debounce/backoff/reconcile cadence. |
| Corruption/rebuild (manifest + fragment-group validation) | **REIMPLEMENT_FROM_BEHAVIOR** | Port validation gates; reuse `validateFragmentGroups` semantics. |
| Concurrency (`JobScheduler`, `RootRuntime` epochs, RW lock, `embeddingConcurrency`) | **REIMPLEMENT_FROM_BEHAVIOR** | Port to Rust `tokio`/channels + RWLock + epoch gating. |
| Remote-embedding authorization (HMAC grant, fingerprint, scope) | **COPY (pattern)** | Adopt the *design* as the product's remote-embedding consent model; reimplement in Rust. |
| CLI / daemon / MCP server | **DEFER** | Not required for first ingestion/index adoption; A9/A11 decide exposure. W19 must not start a daemon / expose MCP / download a model without A0 approval (dispatch constraint). |

**Net:** the *engines* are copy/adapt; the *orchestration* (auth-except-remote, discovery, extraction, incremental, watcher, concurrency, storage I/O) must be **reimplemented from behavior** in Rust, while preserving the `@zvec/zvec` on-disk contract so indexes remain compatible. A Node sidecar running zvec-grep daemon is an alternative `ADAPT` path (run as-is, call over HTTP/MCP) that avoids reimplementation but adds a Node runtime dependency to a Rust product — a trade-off for A0/A10/A11.

---

## 7. Test reuse（移植时复用上游测试）

- `test/unit/scanner-utils.test.mjs`, `test/path-indexing.test.mjs` → discovery/ignore (reimplement in Rust, port assertions).
- `test/unit/extraction/code.test.mjs`, `test/unit/extraction/core.test.mjs` → chunking/extraction.
- `test/unit/zvec-storage.test.mjs`, `test/integration/manifest.test.mjs` → manifest/storage format.
- `test/change-set.test.mjs`, `test/daemon-backend.test.mjs` → incremental/rebuild.
- `test/watch-manager.test.mjs`, `test/job-scheduler.test.mjs`, `test/index-coordinator.test.mjs` → watcher/concurrency.
- `test/authorization.test.mjs`, `test/unit/authorization-prompt.test.mjs` → remote-embedding authorization.
- `test/unit/cli-format.test.mjs`, `test/unit/search.test.mjs`, `test/mcp.test.mjs` → compact output / RRF fusion.

A11 should fold these into the W19 acceptance matrix (unit + integration + a GUI smoke that indexes a synthetic workspace and verifies a known passage is retrievable).

---

## 8. Unresolved questions（交给 A0 / A10 / A11）

1. **`@zvec/zvec` native surface:** Is it a native (napi/neon) addon with a Rust API usable directly from Tauri, or Node-only? Determines COPY-vs-sidecar. **A10 must resolve.**
2. **Default ignore contents:** `DEFAULT_IGNORED_DIRECTORY_NAMES` / `DEFAULT_IGNORED_FILE_PATTERNS` exact entries should be read in full (scanner/index.ts:137 block) when porting behavior, to match upstream silently-ignored paths.
3. **Workspace grant reconciliation:** product's OS-level "workspace authorization" vs zvec-grep's config-trusted `RootPath` scope — must be unified (A9 threat model).
4. **Large-workspace capacity:** inotify watch limits on Linux for very large repos; the 60-min reconcile + `forceFullReconcile` escalation should be stress-tested (A11 perf case).
5. **Remote model unavailability:** A8 benchmarks the unavailable-model failure path; A7 notes the rebuild gate blocks model/endpoint change and must require explicit `rebuild`.
6. **MCP/daemon exposure:** deferred per dispatch; A9/A11 decide if the agent (MCP) surface is adopted.

---

## 9. Source ledger seeds（供 A10）

| Upstream symbol (file) | Kind | License | Proposed destination (product) |
|---|---|---|---|
| `@zvec/zvec` vector store (`storage/zvec.ts`) | engine | Apache-2.0 | Rust Tauri native / sidecar |
| `scanRootPaths` + `DEFAULT_IGNORE_RULES` (`pipeline/indexing/scanner/index.ts`) | behavior | Apache-2.0 | Rust reimplementation |
| `extractForIndexing` + `CodeExtractor` (`extraction/*`) | behavior | Apache-2.0 | Rust reimplementation (tree-sitter) |
| `ChangeSet` (`daemon/change-set.ts`) | behavior | Apache-2.0 | Rust reimplementation |
| `WatchManager` (`daemon/watch-manager.ts`) | behavior | Apache-2.0 | Rust (`notify`) reimplementation |
| `RemoteEmbeddingWorkspaceGrant` (`authorization/types.ts`) | pattern | Apache-2.0 | Rust consent model |
| `WorkspaceManifest` + `files.zvec`/`index.zvec` (`manifest.ts`, `storage/layout.ts`) | format | Apache-2.0 | Compatible reimplementation |

---

## 10. Reference evidence（精确本地路径 + 钉版本）

- Snapshot: `/home/ainfinit/Documents/极智简单/V3/research/zvec-grep-src` — `git rev-parse HEAD = 5265395` (pinned `52653951b24617762f4ab0c71c34d594e5001617`), `package.json` version `0.2.1`, `LICENSE` = Apache License 2.0.
- Key files read/confirmed: `src/engine/storage/layout.ts`, `src/authorization/types.ts`, `src/engine/pipeline/indexing/scanner/index.ts` (lines 130–224), `src/engine/pipeline/indexing/input-budget.ts`, `src/engine/storage/zvec.ts`, `src/engine/pipeline/search/index.ts`, `src/daemon/watch-manager.ts`, `src/daemon/change-set.ts`.
- Dispatch: `PARALLEL_COMMAND_BOARD.md` § "M5-W18-R Research and Replication Blueprint Dispatch" (A7 row line 1401).
