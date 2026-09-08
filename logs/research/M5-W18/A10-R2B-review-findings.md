# A10 — R2B Independent Review Findings (M5-W18-R2B)

> Lane: **A10** (Licensing, dependency BOM, source-transplant plan → R2B = independent review)
> Dispatch: **M5-W18-R2B** (`M5-W18-R2B-TASKS-20260908.md` §11)
> Date: 2026-09-08
> Branch: `codex/m5-w18-a10`; BASE: `origin/master` `434e63f`
> Companion (preserved R2): `A10-source-transplant-ledger.md`, `A10-dependency-bom.md`, `A10-notice-and-review-gate.md`, `A10-checkpoint.md`

This package **preserves all R2 results** and adds the R2B independent review required by §11:
verify peer claims with `证据 → 问题 → 影响 → 最小修正 → 归属 lane`, separate
`INDEPENDENTLY CONFIRMED / AUTHOR CLAIMS / UNVERIFIED`, and compile the
cross-cutting conflicts that need A0's ruling.

---

## 1. Method & consumed peers (fixed SHAs)

All peer reports read via `git show <SHA>:<path>` (fixed), not uncommitted files.

| Lane | HEAD | Consumed report(s) | Focus for A10 |
|---|---|---|---|
| A1 | `526e2ef` | `A1-product-baseline-R2-20260908.md`, `A1-checkpoint-R2-20260908.md` | shared stores / shell ownership |
| A2 | `bcdfc3b` | `A2-obsidian-vault-semantics.md`, `A2-fixtures.md` | Obsidian behavior ref |
| A3 | `877b4f5` | `A3-obsidian-graph-search-ux-20260908-0759.md` | graph store / dispatch typo |
| A4 | `b737e5d` | `A4-dbx-backend-architecture-map.md`, `A4-checkpoint.md` | dbx identity, DbValue, test count, sync drivers |
| A5 | `a42b915` | `A5-replication-blueprint.md`, `A5-workbench-ux-map.md`, `A5-checkpoint.md` | DB UI store, cancel DTO dependency |
| A6 | `38faa2d` | `A6-security-lifecycle-audit.md`, `A6-checkpoint.md` | credential keyring flow |
| A7 | `07fda2f` | `A7-zvec-grep-R2-evidence-closure.md`, `A7-zvec-grep-ingestion-index-architecture.md` | zvec binding/format proof |
| A8 | `f4a4f3b` | `A8-zvec-grep-retrieval.md`, `A8-checkpoint.md`, `A8-benchmark-*.mjs` | benchmark metric semantics |
| A9 | `28a934a` | `A9-threat-model.md`, `A9-source-map.md`, `A9-checkpoint.md` | egress default, apiKey at rest |
| A11 | `c3d4712` | `A11-verification-architecture-R2-20260908.md` | cross-lane status (for conflict routing) |
| **A10** | `b81fa69` (pre-R2B, 3 R2 commits) | ledger/bom/notice/checkpoint | this review |

Product facts (`CURRENT_PRODUCT`) verified directly in `src-tauri/src/database.rs`,
`security_policy.rs`, `LICENSE`.

---

## 2. Findings (evidence → issue → impact → minimal fix → owner)

Status legend: ✅ INDEPENDENTLY CONFIRMED · 🗣 AUTHOR CLAIMS · ❓ UNVERIFIED.

### F1 — DbValue serialization (A4 claim "✅ Implemented @ database.rs:120-162")
- **证据**: product `database.rs:118-168` — `enum DbValue { Null, Bool, I64, F64, Text, Binary { bytes } }`; BLOB returns length only; `DbQueryResult` carries `truncated`/`field_truncated`/`query_id`.
- **状态**: ✅ INDEPENDENTLY CONFIRMED.
- **问题/影响**: none — A4 classification correct.
- **修正**: none. **Owner: A4 (correct).**

### F2 — Database test count (R2B card: 23; A0 R1 audit: 22; A4 map: "repo has zero DB tests")
- **证据**: `grep -cE '^\s*#\[test\]' database.rs` = **23** (precise; comments/strings excluded). `security_policy.rs` has 80 `#[test]` separately.
- **状态**: ✅ INDEPENDENTLY CONFIRMED = 23.
- **问题**: A4 map §1.2/§2.x states "repo currently has zero DB-layer tests" — **factually wrong**; the repo `database.rs` already has 23 unit-level tests. A0 R1 audit's "22" should also be corrected to 23.
- **影响**: if W19 test strategy uses A4's "zero", it misstates the baseline; the real gap is **live-DB integration** tests, not all tests.
- **修正**: (a) correct R1 audit 22→23; (b) correct A4 "zero DB tests" → "database.rs has 23 unit `#[test]`; live-DB integration tests missing". **Owner: A0 audit doc (a); A4 (b). Low-risk → return to original lane, do not escalate to A0.**

### F3 — Synchronous driver model (A4 REJECT async dbx pool)
- **证据**: product `database.rs`/deps use sync `rusqlite(bundled)`+`mysql`+`postgres`, no tokio runtime; A4 map §1.x/§2.x confirms `SupportedDb={sqlite,mysql,postgres}` sync; dbx 23-variant async `PoolKind` = REIMPLEMENT_FROM_BEHAVIOR at smaller scale.
- **状态**: ✅ INDEPENDENTLY CONFIRMED; matches A10 R2 correction C1.
- **影响**: none — transplant must stay sync, no second runtime (A4/A7 agree).
- **修正**: none. **Owner: A4.**

### F4 — dbx upstream pin consistency (A10 ledger D1 vs A4)
- **证据**: A4 map §0/§2.x: dbx src `/research/dbx-src` (Apache-2.0, `Cargo.lock` SHA `c0a7be12…`). A10 ledger §1 already pins `c0a7be12` (authoritative). R1 audit fragment's stray `01a6e16` is superseded.
- **状态**: ✅ ALREADY ALIGNED — no correction needed.
- **修正**: none (note for reviewers: ignore R1 `01a6e16`). **Owner: A10 (ledger already correct).**

### F5 — zvec binding / format (A7 R2B evidence closure)
- **证据 (A7 E1–E7)**: `@zvec/zvec` 0.7.0 npm N-API addon (NOT a Rust crate); official `zvec-rust` 0.7.0 crate (Apache-2.0, depends `zvec-rust-sys` 0.7.0 linking `libzvec_c_api`); C++ core `alibaba/zvec` (Apache-2.0, C API + per-platform prebuilt SDKs); npm `bindings-linux-x64` 0.7.0 → `zvec_node_binding.node` (ELF N-API, 41.7 MB). Two unofficial crates excluded.
- **状态**: ✅ INDEPENDENTLY CONFIRMED + **corrects A10 R2 ledger Z8** (was "Rust API unresolved" → now official crate PROVEN, product path = npm N-API sidecar; both Apache-2.0 → no license conflict).
- **影响**: Z8 HOLD-(a) lifted; classify ADAPT-sidecar (whole sidecar, not per-function COPY) — matches A8 `COPY=0`. Native footprint: `.node` 41.7 MB + engine RSS ~294 MB (bundle impact → A3/A10 eval).
- **修正**: A10 ledger Z8 updated (§9). **Owner: A7 (evidence); A10 (ledger sync). Done.**

### F6 — Product license (A8 corrected A0 #7 to MulanPSL-2.0)
- **证据**: product `LICENSE` head = "木兰宽松许可证，第2版 (MulanPSL-2.0)"; A8 §4 line 167 "已修正产品许可证为 MulanPSL-2.0（修正 A0 #7）".
- **状态**: ✅ INDEPENDENTLY CONFIRMED.
- **影响**: confirms A10 ledger L1 (product MulanPSL-2.0, not changed); inbound Apache-2.0 (dbx/zvec) compatible; NOTICE obligation retained.
- **修正**: none. **Owner: A8 (corrected A0 #7); A10 ledger L1 aligned.**

### F7 — zvec benchmark metric semantics (A8)
- **证据**: A8 §2.x line 64 "recall@10 = 返回结果中属正确主题的比例" — this is the **precision@10** definition (fraction of returned results that are relevant), not recall (fraction of the relevant set retrieved). Line 73 "recall@10 = 1.00" under this definition is actually precision@10 = 1.0.
- **状态**: 🗣 AUTHOR claims "corrected", but the precision/recall denominator split required by R2B §9 is **not applied** — A8 still reports precision as recall@10.
- **问题**: if W19 cites A8 "recall=1.0" to argue retrieval quality, it overstates: only high precision (returned results all relevant) is proven; true recall needs a labeled complete relevant set (not provided).
- **影响**: medium — could mislead W19 retrieval-quality claims.
- **修正**: A8 should (a) relabel "returned-results-relevant-ratio" as precision@10; (b) keep recall UNPROVEN unless a labeled full relevant set exists; (c) state cold/warm sample counts + cache conditions. **Owner: A8. Recommend A0 ruling on metric naming before W19 use.**

### F8 — Egress default off (A9)
- **证据**: product debug-only IPC + no default `remote.urls`; A9 B1 remote-embedding authorization gate (`default=cancel`, TTL 10 min, max 4096, throws if non-TTY and not `--allow-remote`); A9 §: "refuses non-loopback … no network exposure by default".
- **状态**: ✅ INDEPENDENTLY CONFIRMED (consistent with product's existing "no network by default").
- **影响**: none — zvec-grep remote embedding disabled by default, matches threat model.
- **修正**: none. **Owner: A9 (strong confirmation).**

### F9 — Remote-embedding API key stored plaintext (A9 B3)
- **证据**: A9 B3 "API-key storage at rest ⚠️ PLAINTEXT (ADAPT — must use OS keychain)".
- **状态**: 🗣 AUTHOR claims; consistent with A4 REJECT FileSecretStore / A6 keyring flow.
- **问题**: if W19 reuses zvec-grep's plaintext apiKey-on-disk, it violates this product's keyring contract.
- **影响**: credential plaintext residue risk.
- **修正**: W19 remote-embedding creds must go through `keyring_store.rs`, not plaintext; align with A4/A6 credential flow. **Owner: A9 (found); A4/A6 (credential contract). Low-risk → original lanes.**

### F10 — Dispatch line 1358 attribution typo (A3 vs A8)
- **证据**: A3 §10-6 flags "dispatch 第1358行写『after A3's adoption verdict』但 zvec-grep 采用结论由 A8 负责 → 疑似笔误（应为 A8）". A10 review: R2B §8 gives zvec-grep *ingestion* to A7, *retrieval/route* to A8; A3 owns Obsidian graph UX only. A3's objection holds.
- **状态**: 🗣 AUTHOR claims (A3); ✅ A10 concurs.
- **影响**: literal reading would mis-assign zvec-grep adoption verdict to A3.
- **修正**: A0/A11 correct dispatch line 1358 "A3's adoption verdict" → "A8's adoption verdict". **Owner: A0/A11 (dispatch doc). ESCALATE.**

### F11 — Shared `ConfirmModal` / `useModalFocus` (A1 vs A5)
- **证据**: A5 §: write-confirm dialog reuses `ConfirmModal`/`useModalFocus` (existing `GitWriteConfirmDialog` pattern); A1 owns the shared component library.
- **状态**: 🗣 AUTHOR claims.
- **问题**: if A5 re-implements the confirm modal instead of referencing A1's shared lib, duplication/inconsistency.
- **修正**: single owner = A1 shared component lib; A5 only references. **Owner: A1 (owner); A5 (reference). Low-risk → original lanes.**

### F12 — DB store ownership / `db_cancel` freeze (A4 vs A5)
- **证据**: A5 depends on A4 `db_query` returning `query_id`/state + new `db_cancel(conn_id, query_id)`; A5 §: "A4 未给列表命令" (`refreshConnections` empty).
- **状态**: 🗣 AUTHOR claims.
- **问题**: A5 cancel/history UI is blocked on A4's unfrozen DTO/command.
- **影响**: cross-lane sequence dependency; A5 W19 needs A4 to freeze `db_cancel` signature + `DbQueryResult.query_id`.
- **修正**: A4 freezes `db_cancel(conn_id, query_id)` and `DbQueryResult.query_id` before A5 W19; A0 records the dependency SHA at W19 open. **Owner: A4 (freeze); A5 (consume). ESCALATE dependency to A0.**

---

### F13 — DbValue wire schema divergence (two Rust enums; frontend cannot decode numeric/binary) ★ HIGH-VALUE
- **证据**: product has **two** `DbValue` enums with different JSON tags:
  - `src-tauri/src/database.rs:122` `pub enum DbValue { Null, Bool, I64, F64, Text, Binary{bytes} }` + `serde rename_all="snake_case"` → wire tags **`null/bool/i64/f64/text/binary`**. `database.rs:128 DbQueryResult.rows: Vec<Vec<DbValue>>` uses **this** enum (the actual query DTO).
  - `src-tauri/src/domain.rs:1094` `pub enum DbValue { Null, Bool, Int, Float, Text, BlobLen }` → tags **`null/bool/int/float/text/blob_len`**. Tests `domain.rs:2433` assert `DbValue::Int(7)`→`{"int":7}`; `:2413-2425` (B8-1) states "DbValue JSON 键名是前后端唯一契约" and mandates `blob_len`.
  - TS mirror `src/types.ts:603-614` uses **`int/float/blob_len`** (matches `domain.rs`, NOT the wire `database.rs`).
  - `src/utils/dbUi.ts:273-289 decodeDbValue` only handles `rec.int/rec.float/rec.blob_len`; for a wire value `{"i64":7}` none match → falls to `JSON.stringify(value)` → cell renders raw `{"i64":7}`.
- **状态**: ✅ INDEPENDENTLY CONFIRMED defect. A4's "Ts 镜像须逐字对齐 domain.rs::DbValue" (R2B §11 校对项) is **NOT met on the wire**: the live `db_query` DTO serializes `i64/f64/binary`, but TS + `domain.rs` expect `int/float/blob_len`. B8-1 fixed only `domain.rs`, leaving `database.rs::DbQueryResult` on the divergent enum.
- **问题/影响**: every numeric and binary DB cell renders as raw JSON in the panel; the "single source of truth" for the core DTO is split. This is exactly the kind of cross-layer contradiction A11 §9 claimed to "resolve (DbValue)" — but the resolution was at the report level; the **product still ships a dual-enum mismatch**.
- **最小修正**: unify on ONE `DbValue` (recommend the `int/float/blob_len` form per B8-1 intent) and point `database.rs::DbQueryResult.rows` at it; add a serialization round-trip test (`database.rs` currently only tests `domain.rs` enum). **This is product code — A10 does NOT implement**; flag for W19 fix by A4 (DTO) + A5 (frontend).
- **归属**: A4 (DTO contract) + A5 (frontend decode) + W19 product fix. ESCALATE (correctness defect on core DTO).

### F14 — dbx credential live path is SQLite plaintext table, not dead `FileSecretStore` (A6 corrects A4 + A10 ledger D6)
- **证据**: A6 §1.1/§5: dbx `FileSecretStore` (`connection_secrets.rs:37`) is **dead code** (`grep -rn "FileSecretStore::new"` → 0 matches); dbx desktop live path = SQLite plaintext `connection_secrets` table (`storage.rs:326-329` `secret TEXT NOT NULL`, `save_password=true` writes `password` plaintext at `:2374-2380`). Product DB creds correctly use `KeyringStore` (bridge.rs:6024/6065/6109, key `db:<conn_id>`).
- **状态**: 🗣 AUTHOR claims (A6); ✅ consistent with product `keyring_store` usage (A10 verified bridge.rs flow).
- **问题**: A4 map §1 + A10 ledger **D6** both REJECT "FileSecretStore 主路径" — wrong premise (FileSecretStore is dead); the real risk is dbx's **SQLite plaintext `connection_secrets` table**.
- **影响**: classification rationale is based on a non-existent code path; the actual reject target (plaintext table) is mislabeled.
- **最小修正**: A10 ledger D6 → reject **dbx SQLite plaintext `connection_secrets` table** (storage.rs), note FileSecretStore is dead; align with A6 MERGE_NOTES. (Ledger D6 updated below.) Product itself uses keyring — OK.
- **归属**: A6 (found); A4 + A10 ledger (align). Low-risk ledger refinement, done in §9.

### F15 — A11 R2-provisional "no peer R2 commit" is stale
- **证据**: A11 R2-provisional (`c3d4712`) §1 line 7 / §1 line 77: "A1–A10 尚无任何 R2 commit" / "所有 peer 报告均为 R1 draft… 无任何 peer 产生 R2 commit". Current state: A10 has R2 + R2B commits; A4/A7/A8/A9 have R2/R2B reports.
- **状态**: ❓ STALE at A11's commit time; now superseded by peer R2/R2B.
- **问题**: if A11's matrix is taken as current, it understates peer R2 closure and could block W18 completion incorrectly.
- **影响**: low — A11's own R2B task (§12) is to refresh after peers; expected, but note it.
- **最小修正**: A11 refreshes the R2 status matrix against current peer HEADs (incl. this A10 R2B) before declaring W18 complete; must not claim "all peers have no R2".
- **归属**: A11. Low-risk, return to A11.

## 3. Cross-cutting conflicts requiring A0 ruling (consolidated)

| # | Conflict | Why A0 | Recommended option |
|---|---|---|---|
| F13 | DbValue wire dual-enum (`database.rs` i64/f64/binary vs `domain.rs`/TS int/float/blob_len) → numeric/binary cells render as raw JSON | core DTO correctness defect; not a report-only issue | unify on one `DbValue` (int/float/blob_len per B8-1); point `DbQueryResult.rows` at it; add round-trip test — W19 product fix by A4+A5 |
| F10 | dispatch 1358 A3↔A8 attribution typo | dispatch doc authority | change to A8 |
| F7 | zvec metric naming (precision vs recall@10) | W19 quality claims | A8 relabels; recall UNPROVEN until labeled set |
| F12 | A4 must freeze `db_cancel` before A5 W19 | cross-lane open dependency | record SHA at W19 open |
| F14 | dbx live cred path = SQLite plaintext `connection_secrets` (FileSecretStore dead) — A4/A10 D6 mislabeled | ledger accuracy | A10 ledger D6 updated; A4 align |
| F15 | A11 "no peer R2 commit" stale | W18 completion gating | A11 refresh matrix vs current peer HEADs |

Low-risk items (F2-b, F9, F11) returned to original lanes; not escalated.

---

## 4. Decision summary (≤2 pages)

**Bottom line:** A10's R2 license/transplant ledger stands; R2B independent review
confirms peers on the load-bearing claims and surfaces four items for A0 (F7/F10/F12/F13).

1. **Licensing — CLOSED, no conflict.** Product = MulanPSL-2.0 (re-confirmed from
   `LICENSE` + A8 fixing A0 #7). Inbound dbx (`c0a7be12`, Apache-2.0) and
   zvec-grep (`5265395`, Apache-2.0) are compatible; ship `LICENSE-APACHE-*` +
   NOTICE, keep MulanPSL-2 at file level. dbx pin already authoritative in ledger.
2. **zvec binding — RESOLVED (A7).** Official `zvec-rust` crate + npm N-API
   sidecar both Apache-2.0. Product adopts the **npm sidecar**; classify
   ADAPT (whole sidecar), not per-function COPY (matches A8 `COPY=0`). Lifts Z8 HOLD-(a).
   Bundle cost: 41.7 MB `.node` + ~294 MB engine RSS → evaluate with A3.
3. **DbValue / sync drivers — CONFIRMED.** A4's "✅ Implemented" at `database.rs:120-162`
   and "sync-only, reject dbx async pool" both verified against product source.
4. **DB tests — CORRECTION.** Product `database.rs` has **23** `#[test]` (not "zero",
   not 22). A4's "repo has zero DB tests" and A0 R1 audit's "22" are both wrong;
   fix at source. Gap = live-DB integration tests only.
5. **Egress — CONFIRMED safe by default.** A9 gate (default cancel) + product
   debug-only IPC = no network by default; remote-embedding apiKey must move to
   keyring (F9) before W19. DB creds already use OS keyring (F14 confirms product
   path correct; only dbx's plaintext `connection_secrets` table is rejected).
6. **DbValue — DEFECT FOUND (escalate).** Product ships two `DbValue` enums: wire
   `database.rs` uses `i64/f64/binary`, while `domain.rs` + TS mirror use
   `int/float/blob_len`. Numeric/binary cells therefore render as raw JSON in the
   panel; B8-1 only fixed `domain.rs`. Real cross-layer contradiction (not just a
   report claim) — needs W19 product fix by A4+A5 (F13).
7. **Escalate to A0:** (a) DbValue dual-enum unification (F13); (b) dispatch 1358
   A3→A8 typo (F10); (c) zvec metric naming must not be quoted as recall@10 (F7);
   (d) A4 freezes `db_cancel` before A5 (F12).

**Consequence of each open item:** F10 is a doc fix (no code impact); F7 only
affects how W19 cites retrieval quality (no correctness block); F12 is a W19
sequencing dependency (does not block low-risk first slices).

---

## 5. Candidate implementation slices (PROPOSED_NOT_AUTHORIZED)

> Research only. Not coding authorization. A0 opens W19 per-slice with owner + files + dep SHA.

- **P1 — dbx sync transplant (first wave):** ADAPT over existing sync `mysql`/`postgres`/`rusqlite`; COPY D5a (caps consts) + D11a (`format_csv` cluster); REJECT-as-COPY D3/D4/D6/D11b/D16. Needs A10 provenance + A6 SQL-risk policy + sync-runtime contract. Blocks: none for first slice.
- **P2 — zvec-grep npm sidecar:** ADAPT `@zvec/zvec` 0.7.0 sidecar; REIMPLEMENT route/RRF/chunking in `search.rs` (Z1–Z7,Z9,Z12,Z13). Needs A9 egress gate ported first. Blocks: F9 (keyring for apiKey).
- **P3 — remote-embedding credential keyring:** port A9 B3 fix so apiKey → `keyring_store.rs`. Blocks: none (standalone).

---

## 6. OPEN_DECISIONS / NEXT

- **OPEN_DECISIONS**: F13 (DbValue dual-enum → W19 product fix by A4+A5, escalate), F7 (metric naming → A0), F10 (dispatch typo → A0/A11), F12 (`db_cancel` freeze → A4, recorded by A0 at W19), F14 (ledger D6 dbx plaintext table → A10/A4 align, done), F15 (A11 refresh R2 matrix → A11).
- **WAITING_DEPENDENCY**: none that block this review; A2/A3 Obsidian Vue blueprint (O1–O6) still pending for full transplant closure (does not change first slice).
- **VERIFY (run)**: `grep -cE '^\s*#\[test\]' src-tauri/src/database.rs` → 23 (done, confirmed). Peer claims read from fixed SHAs (done). No product code executed; `NO_PRODUCT_CODE=true`, `NO_PUSH=true`.
- **NEXT**: hand `A10-R2B-review-findings.md` + ledger §9 to A11 for the integration manifest; A0 rules F7/F10/F12 at W19 open.
