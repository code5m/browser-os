# Lane A7 · M5-W18-R2B — Retrieval access feasibility & index-transaction evidence

> STATUS: PASS_WITH_DEBT (build/run PROVEN; cross-binding index interop + crash-safety tests NOT_AUTHORIZED in this research lane)
> Lane: A7 | Mode: RESEARCH ONLY（未改任何产品代码，未 push）
> Dispatch: `M5-W18-R2B-TASKS.md` §8 (Lane A7 line 132–146) + `A0-M5-W18-R2B-handoff-20260908.md` checkpoint items 5/6/9
> Companion reports: `A7-zvec-grep-ingestion-index-architecture.md` (R1), `A7-zvec-grep-R2-evidence-closure.md` (R2, now with §9 R2B corrections)
> Sandbox: `/tmp/m5-w18-a7-zvec` (the R2-authorized disposable sandbox; install/synthesis authorized, product untouched)

Evidence contract (per R2B §1): `CURRENT_PRODUCT` · `REFERENCE_SOURCE` · `OFFICIAL_DOC` · `OBSERVED_BEHAVIOR` · `EXECUTED_SYNTHETIC_TEST` · `INFERENCE` · `DESIGN_DECISION`.

---

## 1. The five sub-claims, separated (R2B §8 requirement)

"Can we use zvec-grep's engine in the product?" is **not** one claim. It is five, and they have different evidence statuses:

| # | Sub-claim | Status | Evidence |
|---|---|---|---|
| S1 | There **exists** a Rust wrapper for the engine | **PROVEN** | `zvec-rust` 0.7.0 crate, repo `zvec-ai/zvec-rust`. Built + ran in §3. `REFERENCE_SOURCE` + `EXECUTED_SYNTHETIC_TEST` |
| S2 | The wrapper has **official identity** (not a look-alike) | **CONFIRMED** (with caveat) | `alibaba/zvec` README line 68 lists `cargo add zvec-rust` as the official Rust binding; crate lives in the `zvec-ai` org that also owns the official `zvec-node`/`zvec-go`. `OFFICIAL_DOC` + `REFERENCE_SOURCE`. Caveat: "official" = published by the zvec-ai org + core README pointer, **not** asserted from crate name/download count (per A0 checkpoint). |
| S3 | The wrapper is **compilable / runnable** in our toolchain | **PROVEN** | §3: `cargo` 1.96 build + `libzvec_c_api` dynamic load + open/insert/query/close all succeeded. `EXECUTED_SYNTHETIC_TEST` |
| S4 | **Cross-binding index interop** (npm-built index readable by Rust, or vice-versa) | **UNPROVEN** | No round-trip test performed. Same engine *version* (0.7.0) does **not** prove on-disk format interchange between the N-API and C-API bindings. A0 explicitly forbids deriving compat from version equality. `INFERENCE` (rejected as proof). See §4. |
| S5 | The wrapper is **packagable / publishable** into the Tauri bundle | **PROVEN-with-condition** | `zvec-rust-sys` links a **dynamic** `libzvec_c_api`; the product must vendor that `.so/.dylib/.dll` (Apache-2.0, from `alibaba/zvec` release or the `zvec-ai/zvec-rust` prebuilt) into the bundle and set rpath. Hermetic CI must supply `ZVEC_LIB_DIR` (no build-time GitHub fetch). `REFERENCE_SOURCE` + §3 observation |

**Bottom line for A0:** S1/S2/S3/S5 are PROVEN; S4 is UNPROVEN. The prior R2 report's "ADOPT via `zvec-rust`" (C1) and "indexes interchangeable iff version pinned" (C2) were premature and are retracted in `A7-zvec-grep-R2-evidence-closure.md §9`.

---

## 2. Authoritative `zvec-rust` provenance (link / version / hash / API / deps / dynamic-lib source)

| Field | Value | Source / tag |
|---|---|---|
| Crate | `zvec-rust` 0.7.0 | crates.io/crates/zvec-rust (`REFERENCE_SOURCE`) |
| Repository | `github.com/zvec-ai/zvec-rust` | crates.io `repository` field (`REFERENCE_SOURCE`) |
| License | Apache-2.0 | crate metadata (`REFERENCE_SOURCE`) |
| Downloads (signal only, not authority) | 863 | crates.io (`REFERENCE_SOURCE`) |
| Dependency | `zvec-rust-sys` =0.7.0 (default feature `bundled`) | crate `Cargo.toml` (`OBSERVED_BEHAVIOR`) |
| `zvec-rust-sys` linkage | `links = "zvec_c_api"`; `rustc-link-lib = dylib=zvec_c_api` (dynamic) | `OBSERVED_BEHAVIOR` |
| **Dynamic-lib source (prebuilt)** | `https://github.com/zvec-ai/zvec-rust/releases/download/v0.7.0/zvec-prebuilt-x86_64-unknown-linux-gnu.tar.gz` (fetched + linked in §3) | `EXECUTED_SYNTHETIC_TEST` |
| **Dynamic-lib source (build from source)** | `alibaba/zvec` C++ core, built with CMake (gcc/clang + C++17) | `REFERENCE_SOURCE` |
| Public API (relevant) | `initialize`, `version`, `Collection::create_and_open`, `Doc`, `CollectionSchema::builder`, `FieldSchema::new`, `add_vector_field`, `IndexParams::{hnsw,fts}`, `SearchQuery::new`, `Collection::{insert,query,fetch,stats}`, `Fts`/`FtsQueryParams` | extracted source `src/lib.rs`, `src/schema.rs`, `src/query.rs` (`OBSERVED_BEHAVIOR`) |
| FTS support | `IndexType::Fts = 11`; `IndexParams::fts(...)`; `Fts` query payload — **FTS-only first slice is feasible** | `OBSERVED_BEHAVIOR` |

**Version-reporting discrepancy (flag to A0/A10):** `zvec_rust::version()` returned `v0.0.0` at runtime (§3), although the crate/engine are 0.7.0. The product's `SEARCH_INDEX_REBUILD_REQUIRED` invariant must key off the **crate version** (pinned `=0.7.0`), not the runtime `version()` string, until this is reconciled.

---

## 3. Minimal build / open / insert / query / close + dynamic-lib-load experiment (EXECUTED)

Environment: cargo 1.96.0, cmake 3.28.3, gcc/g++ present, x86_64-unknown-linux-gnu. The experiment used the **local extracted** crate sources (downloaded read-only in R2) wired via `[patch.crates-io]` so no crates.io fetch was needed for the wrapper; `zvec-rust-sys` still fetched its prebuilt `libzvec_c_api` from the `zvec-ai/zvec-rust` GitHub release (raw release download works; only the GitHub *API* endpoint was 403).

```bash
# /tmp/m5-w18-a7-zvec/exp
cat > Cargo.toml <<'EOF'
[package]
name = "a7-exp"
version = "0.1.0"
edition = "2021"
[dependencies]
zvec-rust = { path = "/tmp/m5-w18-a7-zvec/crates/zvec-rust-0.7.0/zvec-rust-0.7.0" }
[patch.crates-io]
zvec-rust-sys = { path = "/tmp/m5-w18-a7-zvec/crates/zvec-rust-sys-0.7.0/zvec-rust-sys-0.7.0" }
EOF

cat > src/main.rs <<'EOF'
use zvec_rust::*;
fn main() -> zvec_rust::Result<()> {
    println!("zvec version: {}", version());
    initialize(None)?;
    let dir = std::env::temp_dir().join("a7_zvec_exp");
    let _ = std::fs::remove_dir_all(&dir);
    let schema = CollectionSchema::builder("exp")
        .add_field(FieldSchema::new("id", DataType::String, false, 0)?)
        .add_vector_field("embedding", DataType::VectorFp32, 4,
            IndexParams::hnsw(MetricType::Cosine, 16, 200)?)
        .build()?;
    let collection = Collection::create_and_open(dir.to_str().unwrap(), &schema, None)?;
    println!("OPEN_OK");
    let mut docs = Vec::new();
    for i in 0..5u32 {
        let mut d = Doc::new()?;
        let pk = format!("doc_{}", i);
        d.set_pk(&pk);
        d.add_string("id", &pk)?;
        d.add_vector_f32("embedding", &[i as f32 * 0.1, 0.2, 0.3, 0.4])?;
        docs.push(d);
    }
    let refs: Vec<&Doc> = docs.iter().collect();
    let wr = collection.insert(&refs)?;
    println!("INSERT success={} error={}", wr.success_count, wr.error_count);
    let q = SearchQuery::new("embedding", &[0.1, 0.2, 0.3, 0.4], 3)?;
    let res = collection.query(&q)?;
    println!("QUERY hits={}", res.len());
    for r in res.iter() { println!("  pk={} score={:.4}", r.get_pk().unwrap_or(""), r.get_score()); }
    drop(collection);
    println!("CLOSE_OK");
    Ok(())
}
EOF

cargo run --offline
```

**Observed output (verbatim):**

```
   Compiling zvec-rust-sys v0.7.0 (...)
   warning: zvec-rust-sys@0.7.0: Using cached prebuilt library from .../out/zvec-prebuilt
   Compiling zvec-rust v0.7.0 (...)
   Compiling a7-exp v0.1.0 (...)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.65s
     Running `target/debug/a7-exp`
zvec version: v0.0.0
OPEN_OK
INSERT success=5 error=0
QUERY hits=3
  pk=doc_1 score=0.0000
  pk=doc_2 score=0.0148
  pk=doc_0 score=0.0168
CLOSE_OK
```

**Interpretation:**
- **Dynamic-lib load (S3/S5):** `zvec-rust-sys` fetched + linked the prebuilt `libzvec_c_api` (downloaded earlier from the `zvec-ai/zvec-rust` release). The process ran → the engine's C-API loaded and executed. PROVEN for x86_64-linux-gnu.
- **Lifecycle (S3):** `create_and_open` → `insert(5)` (success=5, error=0) → `query` (3 hits, non-zero scores) → `drop` (close) all succeeded.
- **FTS feasibility:** API source confirms `IndexParams::fts` + `Fts` payload exist, so a **vector-less FTS-only** collection (the intended first slice) is buildable.
- **Discrepancy:** runtime `version()` = `v0.0.0` (see §2 flag).

This is a **synthetic** lifecycle test (in-memory/temp collection, no files, no crash/cancel/concurrency). It proves the binding works; it does **not** prove crash-safety, cancel-safety, or cross-binding interop.

---

## 4. Cross-binding index interop = UNPROVEN (do not derive from same version)

- The npm package `@zvec/zvec` 0.7.0 (N-API addon) and the Rust crate `zvec-rust` 0.7.0 both bind engine v0.7.0, but they are **different bindings** (N-API vs C-API) with **different code paths** writing the on-disk `files.zvec` / `index.zvec`.
- **No round-trip test was run** (build an index with one binding, open it with the other). Therefore "interchangeable iff version pinned" (old R2 §2 claim, retracted as C2) is **NOT established**.
- A0 checkpoint item 5 explicitly forbids deriving compat from version equality. **Status: UNPROVEN.**
- Consequence for the product: if the product's index must be **shared/read by zvec-grep** (or vice-versa), the binding choice is load-bearing and must be settled by an explicit cross-binding test — **not** assumed. If the product keeps its own namespace (§6), interop is irrelevant and the binding choice is free.

---

## 5. Retraction of "WAL / write-lock guarantees crash safety" + the required design

**Retracted (C3):** the old R2 §6.3 statement "zvec WAL guarantees crash-safety; single-process write lock is inherent" over-claimed. WAL/locking at the engine layer does **not** by itself give our wrapper safe **cancel**, **crash recovery**, or **concurrent-write** semantics.

**Required design (PROPOSED_NOT_AUTHORIZED — design only, not implemented/run in this research lane):**

- **Manifest + generational commit.** Index state under the product's own namespace:
  `<app_cache>/search-index/<root-hash>/manifest.json` (generation counter, engine/crate version, config checksum, chunking params, build timestamp) + `files.zvec` + `index.zvec`.
- **Staging → atomic publish.** Build into `staging/`, `fsync`, then atomically `rename`/`replace` the `current/` pointer. A crash during `staging/` leaves the previous `current/` fully intact and queryable.
- **Cancel.** The build loop inserts in bounded batches and checks a cancellation token between batches (zvec-rust exposes no mid-build cancel). On cancel → discard `staging/`, keep `current/`.
- **Concurrent writes.** Single-writer mutex/lockfile for the product-owned index dir. Reads from `current/` are safe at any time. **Cross-binding concurrent access** (npm + Rust both writing the same dir) is UNPROVEN/unsafe → must not mix bindings on one index dir.
- **Tests to design (not run here):**
  1. `cancel-mid-build` → old index still queryable, no partial `current/`.
  2. `crash-during-staging` → old `current/` intact; no half-written index.
  3. `concurrent build + query` → query sees a consistent snapshot (old **or** new, never partial).
  4. `version-mismatch` → `SEARCH_INDEX_REBUILD_REQUIRED`.

These are explicitly **out of scope for R2B** (research lane, no product code) and must be owned by the W19 implementation lane + A11 acceptance.

---

## 6. First-slice candidates (corrected namespace; both lightweight)

The old R2 §6 described only the "bounded manual index" path and used `force_rebuild` to wipe `<root>/.zvec-grep/` (C4 — **corrected**: never delete user-owned `.zvec-grep`). R2B presents **two** explicit candidates; both use the **product's own cache namespace**, never the user's `.zvec-grep`.

### Candidate A — "managed-rg no-index"
- Use ripgrep-style discovery (Rust `ignore` crate, MIT/Apache) for **live FTS** over configured roots. **No zvec index at all.**
- Pros: zero index machinery, no daemon, no embedding, no crash-safety surface, smallest first slice.
- Cons: re-scans per query (O(files)), no ranking/hybrid, no persisted index.
- Nature: pure **behavior reimplementation** of rg discovery + result shaping. No engine dependency.

### Candidate B — "bounded manual index"
- Build a **zvec FTS index (no embedding)** over configured roots on explicit user trigger; bounded by caps (`MAX_FILES=5000`, `MAX_BYTES_PER_FILE=2MiB`, `MAX_TOTAL_BYTES=256MiB`, `MAX_DEPTH=20`, `CHUNK_CHARS=3600`, `CHUNK_OVERLAP_PCT=15`, `DEFAULT_TOP_K=7`, `TIMEOUT_MS=60000`).
- Stored under `<app_cache>/search-index/<root-hash>/` with **manifest + generational commit** (§5). `force_rebuild` only affects this product namespace.
- Query via `zvec-rust` FTS (`IndexParams::fts` + `Fts`).
- Cons: build cost; staleness (no watcher in first slice → manual/periodic rebuild).

**Either candidate** satisfies "watcher-free, bounded, explicitly triggered." A0 / the W19 implementation lane picks one. R2B does not adjudicate.

---

## 7. Source mapping & classification corrections

- **No invented ACL ordering (C5 retracted).** New commands follow the actual contract: register in `generate_handler!` → allow in `permissions/default-commands.toml` → add scoped permission in `capabilities/default.json` → typed `src/bridge.ts` + `src/types.ts` → `scripts/check-*-policy.py` + tests (per `WORKSPACE_IDENTITY.md` Desktop Runtime Source Gate). The old "must insert before the final `list_artifact_images` line" rule was a prior-lane convention, **not** a hard source requirement; it is not asserted here.
- **COPY reserved for real code units (C6 corrected).** Classification revised:
  - `libzvec_c_api` (engine) → **ADAPT** (vendored dynamic lib + FFI, not a drop-in static COPY).
  - `zvec-rust` 0.7.0 wrapper → **ADAPT**.
  - ripgrep discovery / extractors / RootPath scope / remote-embedding HMAC grant → **REIMPLEMENT_FROM_BEHAVIOR** (the HMAC grant is a *behavior reimplementation* of `RemoteEmbeddingWorkspaceGrant`, **not** "COPY(pattern) + Rust reimpl").
  - No third-party *code unit* is copied verbatim into the product in the first slice.

---

## 8. A7 / A8 reconciliation (no unresolved contradiction left to A0)

- **A7 (this lane):** proved the Rust binding (`zvec-rust`) builds and runs (§3). Concludes the Rust binding is *usable*.
- **A8 (summary):** currently states only the **Node sidecar** (`@zvec/zvec` N-API) is usable.
- **Reconciliation:** both routes are technically viable — Rust binding (proven build/run here) **and** Node sidecar (working N-API addon per R2 inspection). Neither is "the only way." The product decision depends on (a) whether indexes must be shared with zvec-grep (S4 UNPROVEN → if shared, binding choice is load-bearing; if product-own namespace, it is free) and (b) bundle/hermetic-build cost (Rust needs vendored `libzvec_c_api`; Node would need a sidecar V8 runtime inside Tauri's Rust backend — heavier).
- **A0 decision required:** this lane does **not** claim "adopt Rust binding" nor "Node sidecar is the only way." Both are documented; A0 adjudicates. Critically, **cross-binding index interop (S4) remains UNPROVEN**, so mixing bindings on a shared index dir is not yet safe.

---

## 9. Candidate implementation card (PROPOSED_NOT_AUTHORIZED)

> Not committed as product code. For A0 / W19 implementation lane.

- **Scope:** one lightweight first slice — pick Candidate A (no-index rg) **or** Candidate B (bounded manual FTS index). Both FTS-only, no embedding, no daemon, no watcher.
- **Namespace:** `<app_cache>/search-index/<root-hash>/` (product-owned). **Never** read or delete user `.zvec-grep`.
- **Engine (if Candidate B):** `zvec-rust = "=0.7.0"` + vendored `libzvec_c_api` via `ZVEC_LIB_DIR` (hermetic). Record provenance + NOTICE (A10, MulanPSL-2.0 inbound).
- **Durability (if B):** manifest + generational commit (§5). Cancel token between batches. Single-writer lock.
- **Contract:** command → ACL → capability (scoped) → typed bridge/types → policy gate + tests.
- **Hard stops:** no daemon/watcher; no remote embedding/model; pin engine version; hermetic build; atomic delivery.

---

## 10. What was executed vs not (honest boundary)

| Item | Status |
|---|---|
| `cargo` build of `zvec-rust` 0.7.0 + link prebuilt `libzvec_c_api` | **EXECUTED** |
| open / insert(5) / query(3) / close lifecycle | **EXECUTED** |
| FTS field/query API present in source | **EXECUTED (source inspection)** |
| Cross-binding npm↔Rust index round-trip | **NOT EXECUTED — UNPROVEN** |
| Crash-safety / cancel-mid-build / concurrent-write tests | **NOT EXECUTED — design only (§5)** |
| Capacity/caps enforcement in product | **NOT EXECUTED — design only** |
| ACL / capability / bridge / policy wiring | **NOT EXECUTED — design only** |

---

## 11. Consumed peers / produced files

- **Consumed:** R2B card §8, A0 R2B handoff (items 5/6/9), R1 `A7-zvec-grep-ingestion-index-architecture.md`, R2 `A7-zvec-grep-R2-evidence-closure.md` (now §9), `WORKSPACE_IDENTITY.md` Desktop Runtime Source Gate, `alibaba/zvec` README (line 68), `zvec-rust`/`zvec-rust-sys` 0.7.0 sources.
- **Produced this lane:** `A7-R2B-retrieval-access-index-transaction.md` (this), `A7-zvec-grep-R2-evidence-closure.md §9` (corrections), `logs/checkpoints/A7-M5-W18-R2B-checkpoint.md`.

## 12. Open decisions for A0

1. **Binding choice** (Rust `zvec-rust` vs Node sidecar) — A0 adjudicates (§8).
2. **First-slice candidate** (A managed-rg no-index vs B bounded manual index) — A0 / W19 picks (§6).
3. **Cross-binding interop** — requires an explicit round-trip test before any shared-index assumption (§4).
4. **`version()` = `v0.0.0` discrepancy** — decide version-pin key (crate vs runtime) (§2).
5. **Hermetic-build source for `libzvec_c_api`** — vendored prebuilt vs CI CMake build (A10).
