# A10 — Attribution / NOTICE Proposal & No-Blind-Copy Review Gate

> Lane: A10 · Wave: M5-W18-R · Date: 2026-09-08
> Companion: `A10-dependency-bom.md`, `A10-source-transplant-ledger.md`

This document proposes (a) the attribution/NOTICE artifacts required when dbx / zvec-grep (both Apache-2.0) code is transplanted into this **MulanPSL-2** product, and (b) a **no-blind-copy review gate** — a static policy script + checklist that blocks any transplant that skips provenance/license/modification tracking.

---

## 1. License interaction (recap from BOM §1)

- Product license: **MulanPSL-2** (木兰宽松许可证第2版).
- Inbound: **Apache-2.0** (dbx, zvec-grep, `@zvec/zvec`, `sqlparser`, `mongodb`, `redis`, most pure-Rust crates).
- Apache-2.0 §4 survives the combination: copied files keep Apache-2.0 at file level; we must ship the Apache license text + retain notices + mark modified files. MulanPSL-2 §3 similarly preserves combined-component license text.

**Rule:** a transplanted file is dual-attributed — it keeps its upstream Apache-2.0 header and we add a project-level NOTICE entry pointing back to it. The project as a whole remains MulanPSL-2; the Apache files remain Apache-2.0.

---

## 2. Proposed attribution artifacts

### 2.1 New license files (ship in repo root)

- `LICENSE-APACHE-DBX` — verbatim Apache-2.0 text, header note: *"dbx source copied under Apache-2.0 from local snapshot pinned at Cargo.lock SHA-256 c0a7be12… (upstream t8y2/dbx). Copyright holders per upstream file headers."*
- `LICENSE-APACHE-ZVEC-GREP` — verbatim Apache-2.0 text, header note: *"zvec-grep source copied under Apache-2.0 from upstream zvec-ai/zvec-grep @ 52653951b24617762f4ab0c71c34d594e5001617 (npm @zvec/zvec-grep 0.2.1). Copyright holders per upstream file headers."*
- Keep existing `LICENSE` (MulanPSL-2) and `tauri-browser-tabs/{LICENSE-APACHE,LICENSE-MIT}` untouched.

### 2.2 New `NOTICE` (root) — required by Apache-2.0 §4(d) spirit + MulanPSL-2 §3

```
mvp-browser-os
Copyright (c) <our entity>
Licensed under the 木兰宽松许可证第2版 (MulanPSL-2).

This product includes portions derived from third-party works, each retained
under its original license:

1. dbx (https://github.com/t8y2/dbx)
   Apache License 2.0. Snapshot pinned at Cargo.lock SHA-256
   c0a7be12c05d8dffe867f1a70d4b82e881dec2da3bb622b5d3e3410c3d10e3a7.
   See LICENSE-APACHE-DBX. Copyright per upstream file headers.
   Transplanted modules: <list D1–D16 paths at transplant time>.

2. zvec-grep (@zvec/zvec-grep, https://github.com/zvec-ai/zvec-grep)
   Apache License 2.0. Upstream rev 52653951b24617762f4ab0c71c34d594e5001617.
   See LICENSE-APACHE-ZVEC-GREP. Copyright per upstream file headers.
   Reimplemented (not copied) concepts: route selection, RRF fusion,
   type-aware chunking, incremental reconciliation, authorization-before-egress.

3. tauri-browser-tabs (vendored) — Apache-2.0 / MIT. See tauri-browser-tabs/.

This NOTICE does not modify any license. Modifications to Apache-2.0 files
are marked with a `// MODIFIED FROM <upstream> <rev>` header in each file.
```

### 2.3 Per-file modification header (enforced by the gate)

Every file that adapts/copies an upstream unit MUST begin (after any existing license header) with:

```rust
// MODIFIED FROM dbx <snapshot-Cargo.lock-SHA> — <original path/function>
// Transplanted under Apache-2.0; see LICENSE-APACHE-DBX and NOTICE.
// <one-line description of the adaptation>
```

For REIMPLEMENT_FROM_BEHAVIOR (Obsidian / zvec algorithms), use:

```rust
// REIMPLEMENTED FROM BEHAVIOR of <obsidian|zvec-grep rev> — <behavior/algorithm>
// No upstream source copied; Apache-2.0 provenance recorded for the idea only.
```

---

## 3. No-blind-copy review gate

**Goal:** make "copy first, ask later" structurally impossible. Any transplant PR must pass this gate before merge.

### 3.1 Proposed script: `scripts/check-transplant-policy.py`

Run in `pre-merge.sh` (gate mode) and as a standalone review aid. Active checks (each a hard FAIL unless satisfied):

| Gate ID | What it checks | Why |
|---|---|---|
| `TRANSPLANT_PROVENANCE` | Every new/changed `.rs` under `src-tauri/` that is NOT in the allowlist of original product modules must carry a `MODIFIED FROM` or `REIMPLEMENTED FROM BEHAVIOR` header | no silent copy without provenance |
| `TRANSPLANT_LICENSE_FILE` | If any `MODIFIED FROM dbx`/`zvec` header present → `LICENSE-APACHE-DBX` / `LICENSE-APACHE-ZVEC-GREP` exist in repo root | Apache §4(a) |
| `TRANSPLANT_NOTICE_ENTRY` | `NOTICE` exists and contains a line referencing the upstream + pinned rev for every header seen | Apache §4(d) |
| `TRANSPLANT_NATIVE_BUDGET` | `Cargo.toml` diff must NOT add `openssl` `vendored`, `aws-lc-rs`, `sqlite-sqlcipher`, or any `vendor/` Tauri fork | BOM §2.4 hard rejection |
| `TRANSPLANT_NO_NODE_DEP` | `package.json` / lockfiles must NOT add `@zvec/zvec-grep`, `@huggingface/*`, `node-llama-cpp`, `@vscode/ripgrep` as product deps | zvec-grep is Node-bound; reject copy |
| `TRANSPLANT_NO_TAURI_FORK` | `Cargo.toml` must NOT reference a second `wry`/`tauri` fork path | BOM §2.5 |
| `TRANSPLANT_LEDGER_XREF` | Every `MODIFIED FROM` header's source path must appear in `logs/research/M5-W18/A10-source-transplant-ledger.md` (class COPY/ADAPT) | keep ledger as single source of truth |
| `TRANSPLANT_ACL_PARITY` | Every new Tauri command in the transplant also adds ACL + bridge/types (reuse `check-command-set-consistency.py`) | board non-repeat gate |

The script is **fail-closed**: unknown upstreams (not dbx/zvec/Obsidian) default to FAIL with "provenance unknown — add to ledger". This directly enforces "no blind copy".

### 3.2 Reviewer checklist (human gate, alongside the script)

- [ ] Upstream license confirmed Apache-2.0 (or compatible) and recorded in `NOTICE`.
- [ ] `LICENSE-APACHE-*` shipped; `MODIFIED FROM` / `REIMPLEMENTED` headers present on every affected file.
- [ ] Entry exists in `A10-source-transplant-ledger.md` with destination + classification.
- [ ] Curated dependency subset only (no vendored OpenSSL / aws-lc-rs / sqlcipher / Tauri fork / Node embedding deps).
- [ ] Behavioral red lines respected (no command-without-ACL, no FileSecretStore primary, no JDBC sidecar, no single-window取代 subwebview, no MCP npm分包).
- [ ] New command lands with source-check + handler + ACL + typed bridge/types + policy tests together.
- [ ] Capacity/privacy/release-origin gates still pass.

### 3.3 Integration into existing gates

- Add `check-transplant-policy.py` invocation to `pre-merge.sh` `run_pre_merge` and `run_self_test`.
- Cross-reference from `check-command-set-consistency.py` (ACL parity) and `check-core-boundary.py` (core purity) so transplant code cannot bypass them.

---

## 4. What this gate explicitly REJECTS (the "blind copy" shapes)

1. A `git diff` that drops a 200KB `.rs` from dbx into `src-tauri/src/` with no header → `TRANSPLANT_PROVENANCE` FAIL.
2. Adding `openssl = { features = ["vendored"] }` to shrink build time → `TRANSPLANT_NATIVE_BUDGET` FAIL.
3. `npm i @zvec/zvec-grep` to "just use zg" → `TRANSPLANT_NO_NODE_DEP` FAIL.
4. Importing `vendor/wry` because dbx has WebView2 recovery → `TRANSPLANT_NO_TAURI_FORK` FAIL.
5. Copying `agent_kv.rs` wholesale as a user成果库 → `TRANSPLANT_LEDGER_XREF` FAIL (not in ledger as user-vault; it is AI memory only).

---

## 5. Open items for A0

- Finalize `<our entity>` copyright line in `NOTICE`.
- Decide whether REIMPLEMENT (Rust) or ADAPT-sidecar is the W19 path for zvec-grep embeddings (Z8) — this determines whether `TRANSPLANT_NO_NODE_DEP` stays absolute or gains a sidecar exception with its own egress policy (A9 input).
- Approve `check-transplant-policy.py` as a W19 pre-merge gate before any transplant lands.
