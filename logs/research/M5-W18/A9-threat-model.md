# A9 — zvec-grep Trust-Boundary Audit & Threat Model (M5-W18-R)

> Lane **A9** (RESEARCH, no product-code changes). Reference: `zvec-ai/zvec-grep`
> pinned at `52653951b24617762f4ab0c71c34d594e5001617` (Apache-2.0), local snapshot
> `/home/ainfinit/Documents/极智简单/V3/research/zvec-grep-src`. This report audits the
> trust boundaries the board enumerated for A9: **local/remote embeddings, workspace
> grants, API-key storage, MCP toolsets, bearer auth, server/daemon lifecycle, logs,
> query/content redaction, model download, and data egress** — and produces a threat
> model plus a `COPY / ADAPT / REIMPLEMENT_FROM_BEHAVIOR / DEFER / REJECT` classification
> for W19.

## 0. Scope & method

- **Static source audit only.** Per the W18-R research boundary: no dependency install,
  no model download, no daemon/MCP server started, no network egress, no vault mutation.
  I read the TypeScript sources and the upstream test suite to confirm *intended* behavior
  and *existing* test coverage. No code was executed against a real embedding provider.
- "Measured" = read directly from source. "Inferred" = reasoned from source but not
  dynamically verified (flagged as such).
- W19 must still obtain A0 approval + A3's adoption verdict before any zvec-grep
  dependency or copied module enters product code (board: "Do not … add the dependency in
  this wave without A0 approval after A3's adoption verdict").

## 1. Assets & actors

| Asset | Where it lives (upstream) | Sensitivity |
|---|---|---|
| Remote-embedding **API key** (Qwen/DashScope) | env `ZVEC_GREP_API_KEY`/`DASHSCOPE_API_KEY`/`QWEN_API_KEY`, global config `providers.qwen.apiKey`, workspace manifest `embeddingRuntime.apiKey` | **Secret** — bearer credential to a paid remote service |
| **Workspace content** (source files) | the indexed workspace roots | Confidential to user; must not leave machine without consent |
| **Query text** | search input | Confidential to user; sent to remote only on semantic route |
| **Daemon bearer token** | `<home>/daemon/token` (0600) or env | Secret — gates local MCP/control plane |
| **Remote-embedding workspace grant** | `<workspace>/.zvec-grep/authorization.json` (0600) | Authorization evidence (signed) |
| **Index / model cache** | `<home>/.zvec-grep/` | Not secret, but integrity matters (model weights) |

| Actor | Capability |
|---|---|
| Local user (CLI) | Runs `zg`, grants/revokes, configures providers |
| Agent host (MCP client over stdio bridge → `/mcp`) | Calls `agent` toolset (search-only by default) |
| CLI over `/mcp/admin` | Calls `full` toolset (index/drop/status/rg) |
| Remote embedding provider (DashScope) | Receives workspace content / query text + bearer API key |
| HuggingFace CDN | Serves local model weights (safetensors/tokenizer/GGUF/ONNX) |
| Other local processes | May reach `127.0.0.1:7999` if daemon runs without a token |

## 2. Trust-boundary findings (measured from source)

### B1 — Remote-embedding authorization gate  ✅ STRONG (COPY candidate)
Files: `src/authorization/{types,store,manager,operation,planner,prompt,target}.ts`,
`src/engine/service/zvec-grep.ts:1448`, `src/mcp/tools.ts:724` (`resolveRemoteEmbeddingAuthorization`).

- Every remote embedding call is wrapped by `remoteEmbeddingAuthorizationGuard`
  (`operation.ts:36`). The guard reads a `RemoteEmbeddingOperationPermit` from an
  `AsyncLocalStorage` context and **throws `ZVEC_GREP.ENGINE.AUTH.REMOTE_EMBEDDING_REQUIRED`**
  unless (a) a permit exists, (b) `permit.target.{provider,model,endpoint}` exactly
  matches the request, and (c) for `scope==="workspace"`, a valid HMAC-signed grant
  exists in `<workspace>/.zvec-grep/authorization.json`.
- Grants are **per-workspace, per-target** (`targetFingerprint = sha256(workspaceFP +
  provider + model + endpoint)`), signed with HMAC-SHA256 over a random 32-byte key at
  `~/.zvec-grep/authorization-signing.key` (0600, created `wx`). Verification uses
  `timingSafeEqual`. A grant for one endpoint cannot authorize another.
- Consent is explicit and fail-closed: MCP path uses signed `requestState` (32-byte key
  at `<home>/mcp-request-state.key`, 0600) + a persistent replay guard (0600 tombstones,
  TTL 10 min, max 4096); the elicitation `default` is `cancel`. CLI path prompts on a TTY
  and throws if non-TTY and not `--allow-remote`.
- Planner (`planner.ts`) computes a **disclosure** struct (`queryText`, `workspaceContent`
  = none/selected/changed/full) surfaced verbatim to the user in the consent prompt. Only
  the `qwen` provider triggers remote embedding.

**Conclusion:** This is the canonical "explicit remote-embedding authorization" pattern the
board told W18 to borrow. Directly reusable as a capability/consent primitive for any
remote egress in mvp-browser-os.

### B2 — Daemon HTTP auth & loopback binding  ✅ STRONG, but token is OPTIONAL (ADAPT/REJECT)
Files: `src/daemon/{config,http-server,runtime,server-controller}.ts`,
`src/client/daemon-client.ts`, `src/mcp/http-transport.ts`.

- **Loopback-only bind enforced** (`config.ts:isLoopbackHost`, `DEFAULT_SERVER_HOST=
  127.0.0.1`, `http-server.ts` constructor throws if not loopback). `parseListenAddress`
  refuses non-loopback. Good — no network exposure by default.
- `/mcp` and `/mcp/admin` require **all three**: `validHost` (loopback), `validOrigin`
  (loopback `http:` or absent), and `validRequestToken` (Bearer, `timingSafeEqual`).
  `/control/shutdown` requires host + token. `/healthz` GET is unauthenticated (no side
  effects). Body capped at `MAX_REQUEST_BYTES = 1 MiB`.
- **FINDING (trust gap): the bearer token is OPTIONAL.** `resolveServerToken`
  (`config.ts:74`) returns `{}` (no token) when neither `--token-file`,
  `ZVEC_GREP_SERVER_TOKEN`, nor a token file is present; `http-server.ts:311`
  `validRequestToken` returns `true` when `expected === undefined`. The CLI `args.ts`
  exposes `--token-file` but sets **no default auto-generated token**; `resolveClientToken`
  (`config.ts:96`) falls back to `daemonTokenPath` only if a file already exists. So when
  a user runs `zg server on` without a token, the daemon runs **unauthenticated** — its
  only boundary is loopback + loopback Origin. On a multi-user or shared host, any local
  process that can reach `127.0.0.1:7999` (and sends no/any Origin) could call MCP.
  `requestPrincipal` (`request-state.ts:240`) tags such clients `loopback-anonymous`.
- **A9 recommendation for W19:** do **not** copy the optional-token default. Generate a
  random ≥32-char token on first daemon start, persist it 0600, and **require** it
  (fail-closed). For mvp-browser-os (single local user, Tauri sidecar) the token should be
  injected into the sidecar at launch and never exposed to other apps.

### B3 — API-key storage at rest  ⚠️ PLAINTEXT (ADAPT — must use OS keychain)
Files: `src/engine/config.ts:118-169` (`globalConfigPath` = `~/.zvec-grep/config.json`,
`0600`), `src/engine/manifest.ts:8-48` (`manifest.json` in `<workspace>/.zvec-grep`,
`0600`), `src/engine/service/zvec-grep.ts:1506` `providerApiKeyIdentity`.

- Resolution order (`resolveEmbeddingRuntimeOptions`): explicit `--api-key` > workspace
  manifest `embeddingRuntime.apiKey` > global config `providers.qwen.apiKey` > env > `""`.
- The API key is stored **in plaintext** in both the global config (`0600`) and the
  **workspace-local** manifest (`0600`). The workspace manifest travels with the project
  directory, so a key placed there is exposed wherever the workspace lives (cloud sync,
  shared dir, repo). The global config is `0600` (single-user safe) but still plaintext on
  disk — no OS keychain/secret-service, no encryption at rest.
- Mitigating: the API key is **never persisted into indexes/caches** — only an HMAC
  identity (`providerApiKeyIdentity`, keyed by an in-process random secret) is used for
  cache fingerprinting (`zvec-grep.ts:1506`). Error contexts include `endpoint`/`model`
  but **never** the apiKey. Logs redact it (B6).
- **A9 recommendation for W19:** ADAPT — do **not** copy plaintext-at-rest. Use the OS
  secret store (Linux secret-service / `keyring`, macOS Keychain, Windows Credential
  Manager) or the Tauri/CapGo secure-storage plugin. If a plaintext fallback is
  unavoidable, scope it to the global config only (never the workspace manifest) and
  document the exposure.

### B4 — MCP toolsets & least privilege  ✅ STRONG (COPY/ADAPT)
Files: `src/mcp/toolset.ts` (default `agent`), `src/mcp/tools.ts` (registration +
annotations), `src/daemon/http-server.ts:69` (admin = `full`).

- Two profiles: `agent` (default — **only** `zvec_grep_search`, read-only) and `full`
  (adds index/index_drop/rg/index_status/server_status). The stdio bridge connects the
  agent host to `/mcp` (agent profile); the CLI connects to `/mcp/admin` (full profile).
  So the agent never receives mutation tools unless `ZVEC_GREP_MCP_TOOLSET=full` is set.
- Tools carry `readOnlyHint`/`destructiveHint`/`openWorldHint` annotations for agent
  awareness. `zvec_grep_server_status` explicitly omits repository paths.
- Search results are **compact**: `agent-formatted text` + bounded `preview:"short"`
  snippets; the verbose structured content is dropped unless
  `includeSearchStructuredContent` (admin only). This is data minimization for agent
  context.

### B5 — Local/remote embedding egress & bearer to provider  ✅ CORRECT wiring, DEFER enablement
Files: `src/engine/models/backends/qwen.ts` (POST `endpoint`, `Authorization: Bearer
<apiKey>`), `src/observability/trace-context.ts` (`traceHeaders`).

- Remote egress is a single `fetch(endpoint, {Authorization:"Bearer <apiKey>", body:
  {model,input,dimensions}})`; 60s timeout via `AbortSignal.timeout`, cancellable. Images
  (VL model) are base64-encoded and sent as content — i.e. image bytes leave the machine
  (disclosed in the consent `workspaceContent`).
- **No machine/path metadata leaks to the provider.** `traceHeaders()` returns only W3C
  `traceparent`/`tracestate`/`baggage` derived from the *caller-injected* MCP `_meta`; in
  direct CLI mode it returns `{}`. So zg itself does not transmit hostname or workspace
  path to the remote. (Caveat: if the agent host injects `baggage` containing sensitive
  data, zg forwards it — bounded to 8 KiB and validated. Minor; flag for W19 UI.)

### B6 — Logs & query/content redaction  ✅ STRONG (COPY/ADAPT)
Files: `src/daemon/logger.ts` (`sanitizeFields`), `src/cli/errors.ts` (`redactErrorText`),
`test/job-scheduler.test.mjs` (credential-redaction tests), `test/daemon-logger.test.mjs`.

- `sanitizeFields` drops any field whose key matches `/token|api.?key|authorization|query/i`
  and truncates string values >512 chars. Log file is `0600`. Request IDs + trace IDs are
  logged (not secrets).
- Error contexts from failed remote calls are scrubbed: `job-scheduler.test.mjs` asserts
  `apiKey=`, `Bearer`, `Basic`, `token:` are replaced with `[redacted]` before surfacing
  (covers provider error strings that echo credentials).
- **query/content redaction** in the A9 scope is satisfied two ways: (1) logs never record
  query text or apiKey; (2) remote content is only sent after explicit consent (B1), and
  returned results are compact (B4).

### B7 — Model download (supply chain)  ⚠️ NO INTEGRITY VERIFICATION (REJECT as-is)
Files: `src/engine/models/backends/model2vec.ts:323` (`resolveCachedFile` →
`https://huggingface.co/${repo}/resolve/${revision}/${file}`), `src/engine/models/catalog.ts`
(pinned `revision` commit hashes), `src/cli/help.ts:381` ("Local models are downloaded to
the model cache on first use").

- Local `model2vec` weights are fetched with `fetch(url,{redirect:"follow"})` then
  validated **only** by `isUsableModelFile` (exists & `size>0`). There is **no hash,
  signature, or checksum verification** of the downloaded `model.safetensors` or
  `tokenizer.json`. `revision` is a pinned commit hash (good for reproducibility), but a
  tampered artifact served by HF (or a MITM, or a malicious redirect) would be loaded and
  executed. For GGUF (`llama-cpp`) and ONNX (`transformers-js`) backends the risk is
  higher (arbitrary model execution). `redirect:"follow"` is unconstrained.
- Model cache dir default `<home>/.zvec-grep/models`; partial files use `.part-` then
  atomic `rename`. No explicit chmod on weights (umask default) — permissions are not the
  issue; **integrity** is.
- **A9 recommendation for W19:** REJECT copying the download path as-is. If local models
  are adopted, W19 must add: pinned `revision` (already present) **+ SHA-256/target
  checksum verification against a vetted expectation**, TLS pinning or a trusted mirror,
  and `redirect:"manual"` (no cross-host redirect). Until then, keep local-model download
  **disabled** by default.

### B8 — Server/daemon lifecycle & lease  ✅ SAFE (no priv escalation)
Files: `src/daemon/root-lease.ts`, `src/daemon/server-controller.ts`
(`DaemonInstanceLock`), `src/engine/utils/daemon-lease.ts`.

- "root-lease" is **index-write mutual exclusion**, not OS privilege. It uses a `0600`
  lease file (pid/hostname/instanceToken) with a 5 s heartbeat and abandoned-lease
  recovery (dead-pid detection). No `setuid`/sudo/elevation anywhere.
- Single-instance lock (`instance.lock`, `0600`) prevents two daemons owning the same
  home; shutdown verifies the same instance token before signalling (`SIGTERM`→`SIGKILL`
  with 2 s grace). Lifecycle is clean.

## 3. Threat model (summary)

| # | Threat | Boundary | Likelihood | Impact | Upstream control | W19 verdict |
|---|---|---|---|---|---|---|
| T1 | Remote embedding sends workspace content/query without consent | B1/B5 | Low (gated) | High (data egress) | Explicit HMAC grant + consent + anti-replay | **COPY** the gate |
| T2 | Other local process reaches unauthenticated daemon | B2 | Med (multi-user) | High (local index read/mutation) | Loopback bind only; token optional | **ADAPT** → require token (fail-closed) |
| T3 | API key read from disk / workspace manifest | B3 | Med | High (paid remote abuse) | Plaintext 0600 global; plaintext 0600 workspace | **ADAPT** → OS keychain; never workspace |
| T4 | Tampered model weights executed | B7 | Low–Med | Med–High | Pinned revision only; **no checksum** | **REJECT** as-is; add verification or disable |
| T5 | Secret leaked into logs / error text | B6 | Low | Med | Field-key redaction + 512 trunc + tests | **COPY** the redactor |
| T6 | Replayed/forged remote-embedding consent | B1 | Low | Med (unwanted egress) | Signed requestState + replay guard + TTL | **COPY** |
| T7 | Agent given mutation tools by default | B4 | Low | Med | `agent`=search-only default | **COPY** the toolset split |
| T8 | Machine/path metadata leaked to provider | B5 | Low | Low | traceHeaders = caller-injected only | **COPY**; strip `baggage` in W19 UI |
| T9 | OS privilege escalation via daemon | B8 | Low | High | None present (no elevation) | **COPY** (no action) |

## 4. W19 classification (COPY / ADAPT / REIMPLEMENT_FROM_BEHAVIOR / DEFER / REJECT)

| Item | Class | Rationale / W19 action |
|---|---|---|
| Remote-embedding authorization gate (permit + HMAC grant + consent + anti-replay) | **COPY** | Self-contained, Apache-2.0, directly reusable as egress-consent primitive |
| Daemon loopback HTTP auth model (bearer + validOrigin + timingSafeEqual + 1 MiB cap) | **ADAPT** | Keep structure; make token **mandatory** (generate+require), not optional |
| Log/error redaction (`sanitizeFields` + `[redacted]`) | **COPY** | Port to mvp-browser-os observability; keep key-regex + 512 trunc |
| MCP toolset least-privilege (agent vs full) + result compaction | **COPY** | Reuse profile split + bounded-snippet result shape |
| `traceHeaders` (caller-injected only, no host/path) | **COPY** | Reuse; additionally strip `baggage` at W19 edge |
| API-key storage (plaintext 0600 global + workspace manifest) | **ADAPT→REJECT plaintext** | Use OS keychain; never store secret in workspace manifest |
| Local model auto-download (no integrity check) | **REJECT** (keep disabled) | Add checksum + pinned mirror + no cross-host redirect before enabling |
| Optional unauthenticated daemon mode | **REJECT** (keep disabled) | Require token by default (fail-closed) |
| Remote embedding egress transport itself (qwen.ts fetch) | **DEFER** | Do not enable until A0 approval + A3 adoption verdict |
| Index-write lease / single-instance lock | **REIMPLEMENT_FROM_BEHAVIOR** | Re-implement equivalent mutex for mvp-browser-os index store if needed |

**Counts:** COPY = 5, ADAPT = 2 (+1 plaintext→keychain), REIMPLEMENT_FROM_BEHAVIOR = 1,
DEFER = 1, REJECT = 2 (disabled-by-default items).

## 5. What W19 may copy, must adapt, must keep disabled

- **May COPY:** the remote-embedding authorization subsystem (B1), the daemon auth
  *shape* (B2 structure), the log/error redactor (B6), the MCP toolset split + compact
  results (B4), and the trace-header discipline (B5/B8).
- **Must ADAPT:** API-key storage → OS keychain (B3); daemon token → mandatory (B2).
- **Must KEEP DISABLED until hardened:** (a) remote-embedding egress (enabled only per
  explicit per-workspace grant + consent, never by default); (b) local model auto-download
  (until checksum verification + trusted mirror added, B7); (c) unauthenticated daemon mode
  (B2).

## 6. Unresolved questions / dependencies
- **A3 adoption verdict** gates whether *any* zvec-grep dependency/module enters product
  code (board hard rule). A9 assumes "if adopted, copy the above".
- **A10** must record provenance/license/NOTICE for each COPY item and the destination
  architecture (Rust/Tauri vs the upstream TS). The authorization gate is TS — porting to
  Rust requires re-implementation, not a literal copy.
- **mvp-browser-os secret store** choice (keyring vs Tauri secure-storage) is a W19
  decision that blocks the B3 ADAPT.
- Dynamic verification (running upstream `npm test`) was **out of scope** under the
  no-dependency-install research boundary; test-coverage evidence below is from reading
  the suite, not execution.

## 7. Test-reuse evidence (read from upstream suite, not executed)
- `test/server-http.test.mjs` — 401 (no token), 401 (invalid token), 403 (invalid origin),
  200 (valid Bearer). Covers B2.
- `test/mcp-request-state.test.mjs` — replay-guard (in-memory + persistent) + signature
  tamper. Covers B1 anti-replay.
- `test/authorization.test.mjs` — grant path + HMAC signature + verification. Covers B1.
- `test/job-scheduler.test.mjs` / `test/daemon-logger.test.mjs` — credential redaction
  (`[redacted]`). Covers B6.
- `test/trace-context.test.mjs` — trace header parsing/bounds. Covers B5.
- `test/mcp.test.mjs` / `test/mcp-modern-http.test.mjs` / `test/mcp-legacy-http.test.mjs`
  — toolset exposure + origin. Covers B4/B2.
- `test/config.test.mjs` / `test/config-cli.test.mjs` — apiKey resolution. Covers B3.
- `test/stdio-bridge.test.mjs` — bridge auth forwarding. Covers B2 client side.
