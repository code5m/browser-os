# A10 — Dependency BOM (dbx + zvec-grep + Obsidian behavior)

> Lane: A10 (Licensing, dependency BOM, source-transplant plan)
> Wave: M5-W18-R Research and Replication Blueprint
> Date: 2026-09-08
> Author: Lane A10 (research only — no product code changed)
> BASE: `78d2cfb8e90d323d35df920e9807d32189f867cd` (origin/master, worktree `m5-w18-a10`, branch `codex/m5-w18-a10`)

This document is the **dependency bill of materials** consumed by the source-transplant ledger (`A10-source-transplant-ledger.md`). It records every external crate / npm package we would pull in, its license, its **native (C/C++/binary) footprint**, and a compatibility verdict for this product.

---

## 0. Provenance verification (read-only)

| Reference | Pinned value | Measured | Result |
|---|---|---|---|
| dbx `Cargo.lock` SHA-256 | `c0a7be12c05d8dffe867f1a70d4b82e881dec2da3bb622b5d3e3410c3d10e3a7` | `c0a7be12c05d8dffe867f1a70d4b82e881dec2da3bb622b5d3e3410c3d10e3a7` | ✅ MATCH |
| zvec-grep upstream rev | `52653951b24617762f4ab0c71c34d594e5001617` | snapshot unversioned; documented in dispatch | ⚠️ snapshot not git-pinned locally (see §4) |
| Obsidian | local vault `.obsidian` (config only, no source donor) | read `.obsidian/{graph,app,core-plugins}.json` | ✅ behavior reference only |

The dbx snapshot hash matches the dispatch pin exactly, so every crate version below is the one we would actually transplant from.

---

## 1. License topology — the central finding

| Work | License | Type |
|---|---|---|
| **dbx** (incl. `crates/*`, `src-tauri`, `vendor/*`) | **Apache-2.0** (full text in `LICENSE`) | permissive, patent grant, NOTICE obligation |
| **zvec-grep** (`@zvec/zvec-grep`) | **Apache-2.0** (`package.json` + `LICENSE`) | permissive, patent grant, NOTICE obligation |
| `@zvec/zvec` (zvec-grep dependency, Alibaba) | Apache-2.0 | permissive |
| **This product** (`mvp-browser-os`) | **木兰宽松许可证第2版 (MulanPSL-2)** | permissive, explicit patent grant |

**Implication (must be surfaced to A0):** the product is **not** Apache-2.0. Copying Apache-2.0 code into a MulanPSL-2 tree is permitted by both licenses, but the Apache-2.0 files keep their license at the **file level**, and Apache-2.0 §4(a)(b)(c)(d) still applies:

- (a) recipients must get a copy of the Apache-2.0 license (we ship `LICENSE-APACHE-DBX`, `LICENSE-APACHE-ZVEC-GREP`);
- (b) every modified file must carry a prominent "changed" notice;
- (c) retain existing copyright/attribution notices;
- (d) if the upstream ships a `NOTICE` file, its attribution must be reproduced (currently **neither dbx nor zvec-grep ships a NOTICE** — see `A10-notice-and-review-gate.md`).

MulanPSL-2 §3 likewise requires preserving the original license text of combined components. The combined work stays MulanPSL-2 overall; the copied files remain Apache-2.0 with dual attribution. **No file may be re-licensed wholesale to MulanPSL-2.**

---

## 2. dbx-core Rust dependency BOM

Source: `crates/dbx-core/Cargo.toml` (read in full). Classification: **COPY** = pure-Rust, tractable, recommend adopting; **AVOID** = heavy native build or platform-specific fork; **DEFER** = optional feature / out of first-wave scope.

### 2.1 Pure-Rust drivers & engines (recommended COPY subset)

| Crate | Version | License | Native footprint | Verdict |
|---|---|---|---|---|
| `tokio-postgres` | 0.7 | MIT/Apache | none (pure Rust) | ✅ COPY (with `deadpool-postgres`, `rustls` feature, **not** `postgres-openssl`) |
| `deadpool-postgres` | 0.14 | MIT/Apache | none | ✅ COPY |
| `tokio-postgres-rustls` | 0.13 | MIT/Apache | none | ✅ COPY (use rustls, avoid openssl variant) |
| `rusqlite` | 0.32 | MIT | **compiles SQLite C** (we already use 0.40.2 `bundled`) | 🔁 ADAPT to our 0.40.2; reuse existing bundled SQLite |
| `mysql_async` | 0.37 (fork `zipg/mysql_async`) | MIT | none (pure Rust) | ⚠️ COPY-but-pin: fork has legacy-cert compat; prefer upstream `mysql_async` if equivalent |
| `tiberius` | 0.12 (vendored at `vendor/tiberius`) | MIT | none (MSSQL TDS over pure Rust) | ✅ COPY if MSSQL needed; drop vendored Win7 patch |
| `redis` | 0.32 | MIT/Apache | none | ✅ COPY if needed |
| `mongodb` | 3.2 | Apache-2.0 | none | ✅ COPY if needed |
| `sqlparser` | 0.62 | Apache-2.0 | none | ✅ COPY (SQL risk/analysis — see ledger) |
| `tokio` / `tokio-util` / `futures` | 1.x | MIT | none | ✅ already in product |

### 2.2 Crypto / security (pure Rust, recommend COPY)

| Crate | License | Native | Verdict |
|---|---|---|---|
| `argon2` | Apache-2.0/MIT | pure (some asm) | ✅ COPY (credential KDF) |
| `aes-gcm` / `aes` / `cbc` | Apache-2.0/MIT | pure | ✅ COPY (vault/file encryption fallback) |
| `sha2` | MIT/Apache | pure | ✅ already in product |
| `jsonwebtoken` | MIT | pure | ✅ COPY if JWT needed |
| `rust-decimal` | Apache-2.0/MIT | pure | ✅ COPY (numeric fidelity) |

### 2.3 Files / IO / export (pure Rust, recommend COPY)

| Crate | License | Native | Verdict |
|---|---|---|---|
| `notify` | 7 (MIT/Apache) | none (fs events) | ✅ COPY (index watcher) |
| `calamine` | 0.30 | none | ✅ COPY (xlsx import) |
| `csv` | MIT/Apache | none | ✅ COPY |
| `zip` / `flate2` / `tar` / `zstd` | MIT/Apache | none/miniz | ✅ COPY (import/export) |
| `quick-xml` / `serde_yaml_ng` / `json5` | MIT/Apache | none | ✅ COPY |
| `minijinja` | 2 (MIT) | none | 🔁 DEFER (templating, not first wave) |
| `sysinfo` | 0.32 | none (proc) | ✅ COPY (resource introspection) |
| `portpicker` / `base64` / `percent-encoding` | MIT | none | ✅ COPY trivial |

### 2.4 Heavy native footprint — AVOID / REJECT for first wave

| Crate | License | Native footprint | Verdict |
|---|---|---|---|
| `openssl = { features = ["vendored"] }` | Apache-2.0 | **compiles OpenSSL from source (heavy, slow, fragile)** | ⛔ AVOID vendored; use `rustls` (ring) or system `native-tls` instead |
| `rustls` + `aws-lc-rs` | Apache-2.0/ISC | **`aws-lc-rs` compiles C/asm** | ⛔ AVOID `aws-lc-rs`; use `ring`-backed rustls or `native-tls` |
| `postgres-openssl` | MIT | pulls vendored openssl | ⛔ AVOID (use `tokio-postgres-rustls`) |
| `rusqlite` `sqlite-sqlcipher` + `vendored openssl` | — | double native | ⛔ AVOID SQLCipher (we have no sqlcipher requirement) |

**Build-impact assessment:** the product already compiles `rusqlite=0.40.2 bundled` (SQLite C) and `git2` (libgit2 C). Adding the *curated pure-Rust subset* above adds **no new heavy C build** — it is within our existing native budget. The danger is exclusively `openssl=vendored` + `aws-lc-rs`, which we must exclude.

### 2.5 Workspace-level vendored forks (in `vendor/` + `[patch.crates-io]`)

| Vendored crate | Purpose | Verdict for our product |
|---|---|---|
| `ctor` | Win7 rust-ctor patch | ⛔ REJECT (we don't target Win7) |
| `rumqttc` | MQTT 3.1.1 patch | ⛔ REJECT (governance/MQ, out of scope) |
| `dirs-sys` / `pageant` | Win7 COM/WinRT patches | ⛔ REJECT |
| `tiberius` | unpaired UTF-16 surrogate fix | 🔁 ADAPT only the surrogate fix, not the full vendored copy |
| `wry` (Tauri fork) | WebView2 null-folder + F6 + ProcessFailed recovery | ⛔ REJECT — **we already carry `tauri-browser-tabs` + Tauri 2**; do not import a second Tauri fork |

### 2.6 dbx workspace members

| Member | Verdict |
|---|---|
| `crates/dbx-core` | ✅ source of the COPY/ADAPT subset (DB logic) |
| `crates/dbx-web` (Axum) | ⛔ REJECT (duplicates our Tauri runtime) |
| `crates/dbx-mcp` (rmcp) | 🔁 ADAPT concept — but we already have `mcp_server.rs`/`mcp.rs` in Rust; reuse our own |
| `crates/dbx-cli` | ⛔ REJECT (separate binary; we use Tauri commands) |
| `src-tauri` | ⛔ REJECT as a whole (Tauri app shell); cherry-pick only `webview2_recovery.rs` *concept* for #3 |

---

## 3. zvec-grep npm dependency BOM (TypeScript / Node ≥22)

Source: `package.json` (read in full). **Every zvec-grep runtime dep implies a Node.js host** — this is the single most important BOM fact (see ledger §zvec).

| Package | Version | License | Native footprint | Verdict |
|---|---|---|---|---|
| `@vscode/ripgrep` | ^1.18 | MIT | **bundles a prebuilt `rg` executable** | 🔁 ADAPT: invoke system `rg` (already available on Linux) or bundle; do not take the npm wrapper |
| `@huggingface/transformers` | ^3.8.1 | Apache-2.0 | WASM + **downloads model weights at runtime** | ⛔ REJECT direct (model download / remote egress) |
| `@huggingface/tokenizers` | ^0.1.3 | Apache-2.0 | native Node addon (compiles/loads) | ⛔ REJECT |
| `@zvec/zvec` | ^0.7 | Apache-2.0 (Alibaba) | WASM / ONNX runtime | ⛔ REJECT (vector core needs model) |
| `tree-sitter-wasms` / `web-tree-sitter` | — | MIT | WASM grammar | 🔁 ADAPT concept (AST chunking) — heavy to port |
| `@modelcontextprotocol/*` (client/core/node/server) | 2.0.0 | MIT | pure JS | 🔁 ADAPT concept (we have Rust `rmcp`) |
| `jsonc-parser` | ^3.3.1 | MIT | pure JS | ✅ COPY trivial if needed |
| `zod` | 4.2.0 | MIT | pure JS | ✅ COPY trivial if needed |
| `node-llama-cpp` (optional) | 3.18.1 | MIT | **compiles llama.cpp C++** | ⛔ REJECT |

**Native footprint summary for zvec-grep:** dominated by the `rg` binary, HF native addons, `@zvec/zvec` ONNX runtime, and (optional) `node-llama-cpp`. All require a Node.js runtime and/or model downloads. **No part of zvec-grep can be "copied" into a Rust/Tauri binary**; options are REIMPLEMENT (Rust ports of the routing/RRF/chunking algorithms) or ADAPT (run zvec-grep as a sidecar/subprocess/MCP server — forbidden to actually add in W18-R without A0 approval).

---

## 4. Open provenance gaps

1. **zvec-grep snapshot is unversioned** — the dispatch cites upstream `52653951…` but the local `zvec-grep-src` is not a git checkout. Before any W19 transplant, A0 must re-pin from `zvec-ai/zvec-grep@52653951…` and re-verify the tree hash. The npm `version` field reads `0.2.1`.
2. **No NOTICE files** in dbx or zvec-grep → nothing to propagate today, but our future NOTICE must still record both upstreams (see review-gate doc).
3. **dbx `LICENSE` carries no embedded copyright line** (generic Apache appendix). Copyright must be gathered from per-file headers / `package.json` author at transplant time; this is a gap the review gate must catch.

---

## 5. BOM verdict (one line per source)

- **dbx**: adopt a **curated pure-Rust subset** (drivers + sqlparser + crypto + notify + export); **reject** vendored OpenSSL/aws-lc-rs, the Tauri `wry` fork, and all governance/MQ/Win7 patches.
- **zvec-grep**: **reject copy**; plan REIMPLEMENT (Rust) or ADAPT (sidecar). All deps are Node/model-bound.
- **Obsidian**: behavior-only; no dependency BOM applies (no code donor).
- **License**: all inbound is Apache-2.0 into a MulanPSL-2 product → dual-license attribution required (file-level Apache retention + NOTICE).
