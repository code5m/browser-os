# A9 — Lane Checkpoint (M5-W18-R Research and Replication Blueprint)

```text
LANE=A9
STATUS=PASS
BASE=78d2cfb
HEAD=55233e36e4f010e0945c9bbfbdc6605957ad55a1
REFERENCE_EVIDENCE=/home/ainfinit/Documents/极智简单/V3/research/zvec-grep-src @ 52653951b24617762f4ab0c71c34d594e5001617 (Apache-2.0); snapshot Cargo.lock SHA verified = c0a7be12c05d8dffe867f1a70d4b82e881dec2da3bb622b5d3e3410c3d10e3a7 (board pinned)
FILES=logs/research/M5-W18/A9-threat-model.md, logs/research/M5-W18/A9-source-map.md, logs/research/M5-W18/A9-checkpoint.md
SOURCE_MAP=src/authorization/*, src/engine/service/zvec-grep.ts:1448, src/mcp/tools.ts:724, src/mcp/request-state.ts, src/daemon/{config,http-server,runtime,server-controller,root-lease,logger}.ts, src/client/daemon-client.ts, src/engine/models/backends/qwen.ts, src/engine/models/backends/model2vec.ts, src/engine/models/catalog.ts, src/engine/config.ts, src/engine/manifest.ts, src/observability/trace-context.ts, src/cli/{auth,errors}.ts
CLASSIFICATION=COPY=5, ADAPT=2 (apiKey→keychain; daemon token→mandatory), REIMPLEMENT_FROM_BEHAVIOR=1 (index lease), DEFER=1 (remote egress transport), REJECT=2 (plaintext apiKey at rest; unauthenticated-daemon default; local-model download w/o checksum)
VERIFY=Static source audit + upstream test-suite review (no execution: W18-R forbids dependency install / model download / daemon). Coverage evidence: test/server-http.test.mjs (401/403/origin), test/mcp-request-state.test.mjs (replay+signature), test/authorization.test.mjs (grant), test/job-scheduler.test.mjs + test/daemon-logger.test.mjs (redaction), test/trace-context.test.mjs, test/mcp*.test.mjs, test/config*.test.mjs, test/stdio-bridge.test.mjs, test/root-lease.test.mjs
CHECKPOINT=logs/research/M5-W18/A9-checkpoint.md
MERGE_NOTES=Depends on A3 adoption verdict (board: no dependency until A0 approval after A3). A10 must record license/NOTICE + Rust port plan (authorization gate is TS → re-implement, not literal copy). B3 ADAPT blocked on mvp-browser-os secret-store choice. Security debt: B2 optional token, B3 plaintext, B7 no integrity check — all must be hardened before any zvec-grep capability ships.
NEXT=If A3 adopts: open W19 slice for (1) remote-embedding authorization gate port, (2) mandatory daemon token, (3) OS-keychain apiKey, (4) log/error redactor, (5) keep remote-egress + local-model-download DISABLED until checksum/verdict.
```

## A9 summary

Audit of zvec-grep trust boundaries for W19 reuse. The design is **strong** on the
highest-risk surface — remote-embedding egress is gated behind an explicit, per-workspace,
HMAC-signed grant with MCP elicitation consent, fail-closed default, and a signed
anti-replay requestState. The daemon binds loopback-only and uses timing-safe bearer
checks. Logs redact secrets. MCP exposes a search-only `agent` toolset by default.

**Three gaps must be fixed before any zvec-grep capability ships in mvp-browser-os:**
1. **B2** — daemon bearer token is *optional*; default = unauthenticated loopback. W19
   must generate + require a token (fail-closed).
2. **B3** — API key stored plaintext (0600) in global config **and** workspace manifest.
   W19 must use an OS keychain and never store the secret in the workspace.
3. **B7** — local model download has **no integrity verification** (size-only check,
   `redirect:"follow"`). W19 must add checksum + pinned mirror, or keep download disabled.

**COPY (5):** authorization gate, log/error redactor, MCP toolset split + compact results,
trace-header discipline, index-lease behavior. **ADAPT (2):** apiKey→keychain,
token→mandatory. **REIMPLEMENT (1):** index lease. **DEFER (1):** remote egress transport.
**REJECT/keep-disabled (2+):** plaintext apiKey, unauthenticated daemon, unverified model
download.

Deliverables: `A9-threat-model.md` (threat model + classification), `A9-source-map.md`
(exact files/symbols/lines for A10 ledger). No product code touched. Not pushed (A0
integrates).
