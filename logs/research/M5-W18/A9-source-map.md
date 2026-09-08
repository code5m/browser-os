# A9 — zvec-grep Trust-Boundary Source Map (M5-W18-R)

Exact upstream files/symbols examined, with line anchors, for A10's transplant ledger
and A0's implementation cards. Reference snapshot:
`/home/ainfinit/Documents/极智简单/V3/research/zvec-grep-src` @
`52653951b24617762f4ab0c71c34d594e5001617` (Apache-2.0).

## B1 — Remote-embedding authorization gate
| File | Symbols | Lines | Notes |
|---|---|---|---|
| `src/authorization/types.ts` | `REMOTE_EMBEDDING_CAPABILITY`, `RemoteEmbeddingWorkspaceGrant`, `RemoteEmbeddingAuthorizationDocument`, `RemoteEmbeddingTarget`, `RemoteEmbeddingDataDisclosure` | 1–74 | Grant schema; `targetFingerprint` per provider/model/endpoint |
| `src/authorization/store.ts` | `RemoteEmbeddingAuthorizationStore.{grant,hasGrant,revoke,revokeAll,status}`, `sign`, `verifyGrant`, `getOrCreateSigningKey` | 27–272 | HMAC-SHA256 signing key 32-byte at `~/.zvec-grep/authorization-signing.key` (0600); grant file `<ws>/.zvec-grep/authorization.json` (0600); `timingSafeEqual` |
| `src/authorization/manager.ts` | `RemoteEmbeddingAuthorizationManager.{existingWorkspacePermit,grant}` | 9–30 | Permit issuance |
| `src/authorization/operation.ts` | `remoteEmbeddingAuthorizationGuard`, `withRemoteEmbeddingOperationPermit`, `createRemoteEmbeddingOperationPermit` | 13–79 | The enforcement point; AsyncLocalStorage permit; throws `REMOTE_EMBEDDING_REQUIRED` |
| `src/authorization/planner.ts` | `planRemoteIndexAuthorization`, `planRemoteSearchAuthorization`, `searchUsesVector`, `indexStatusIsFresh` | 8–133 | Disclosure computation (queryText / workspaceContent none-selected-changed-full) |
| `src/authorization/prompt.ts` | `formatRemoteEmbeddingAuthorizationPrompt`, `REMOTE_EMBEDDING_ELICITATION_UNSUPPORTED_MESSAGE` | 13–79 | Consent text; host elicitation fallback |
| `src/authorization/target.ts` | `canonicalizeWorkspaceRoots`, `workspaceFingerprint`, `remoteEmbeddingTargetFingerprint`, `createRemoteEmbeddingTarget` | 6–72 | Fingerprint derivation |
| `src/engine/service/zvec-grep.ts` | `authorizedModel` wrapping `remoteEmbeddingAuthorizationGuard` | 1448–1470 | Wire guard before `model.embed` |
| `src/mcp/tools.ts` | `resolveRemoteEmbeddingAuthorization` | 724–838 | MCP elicitation + signed requestState + replay guard; default `cancel` |
| `src/mcp/request-state.ts` | `RemoteEmbeddingRequestState`, `InMemory/PersistentRemoteEmbeddingRequestStateReplayGuard`, `createRemoteEmbeddingRequestStateCodec`, `loadOrCreateMcpRequestStateKey`, `requestPrincipal` | 19–286 | Signed challenge (32-byte key 0600), tombstones 0600, TTL 10 min, max 4096, `loopback-anonymous` when no token |

## B2 — Daemon HTTP auth & loopback
| File | Symbols | Lines | Notes |
|---|---|---|---|
| `src/daemon/config.ts` | `DEFAULT_SERVER_HOST=127.0.0.1`, `DEFAULT_SERVER_PORT=7999`, `isLoopbackHost`, `resolveServerToken`, `resolveClientToken`, `validateToken` (≥32 chars) | 7–127 | Token optional (returns `{}` when absent) |
| `src/daemon/http-server.ts` | `DaemonHttpServer`, `validHost`, `validOrigin`, `validBearerToken` (`timingSafeEqual`), `validRequestToken`, `MAX_REQUEST_BYTES=1MiB`, `/healthz`/`/control/shutdown`/`/mcp`/`/mcp/admin` | 25–375 | Loopback bind enforced; bearer+origin+token required; admin=`full` toolset |
| `src/daemon/runtime.ts` | `runDaemonForeground` | 27–125 | Resolves token (may be undefined) |
| `src/daemon/server-controller.ts` | `DaemonInstanceLock`, `startServer`, `stopServer`, `forceStopRecordedProcess` | 43–446 | Single-instance lock (0600); SIGTERM→SIGKILL grace 2 s |
| `src/client/daemon-client.ts` | `DaemonClient.invokeTool`, `daemonAdminServerUrl` (`/mcp/admin`) | 34–250 | CLI uses admin endpoint w/ Bearer; agent via stdio→`/mcp` |
| `src/mcp/http-transport.ts` | `McpHttpEndpoint` | 37–263 | Legacy/modern sessions; idle TTL 30 min; max 256 legacy |

## B3 — API-key storage
| File | Symbols | Lines | Notes |
|---|---|---|---|
| `src/engine/config.ts` | `globalConfigPath` (`~/.zvec-grep/config.json`, 0600/700), `resolveEmbeddingRuntimeOptions` (apiKey order), `parseProviders` (plaintext apiKey), `environmentApiKey` (`ZVEC_GREP_API_KEY`/`DASHSCOPE_API_KEY`/`QWEN_API_KEY`) | 12–127, 64–112, 345–476 | **Plaintext** apiKey at rest |
| `src/engine/manifest.ts` | `WORKSPACE_MANIFEST_MODE=0o600`, `writeWorkspaceManifest`, `isEmbeddingRuntime` (apiKey string) | 8–52, 127–142 | **Workspace-local plaintext** apiKey |
| `src/engine/service/zvec-grep.ts` | `providerApiKeyIdentity` (HMAC, in-process secret) | 1506–1510 | Only HMAC identity persisted in cache, not the key |
| `src/cli/auth.ts` | `authorizationStore`, `authorizeCliPlan` | 118–193 | CLI consent flow; throws if non-TTY without `--allow-remote` |

## B4 — MCP toolsets
| File | Symbols | Lines | Notes |
|---|---|---|---|
| `src/mcp/toolset.ts` | `DEFAULT_MCP_TOOLSET="agent"`, `McpToolset="agent"\|"full"`, `resolveMcpToolset` | 1–21 | Default = search-only |
| `src/mcp/tools.ts` | `registerZvecGrepTools` (toolset-gated registration), tool annotations, `formatAgentContextResult` compact `short` | 334–646 | agent vs full; bounded snippets; `server_status` omits paths |

## B5 — Egress & bearer to provider
| File | Symbols | Lines | Notes |
|---|---|---|---|
| `src/engine/models/backends/qwen.ts` | `QwenTextEmbeddingModel.doEmbed`, `Qwen3VlEmbeddingModel.doEmbed`, `remoteEmbeddingSignal` (60 s) | 56–113, 115–223, 262–441, 586–589 | `POST endpoint` + `Bearer <apiKey>`; images base64; 60 s timeout |
| `src/observability/trace-context.ts` | `traceHeaders`, `traceContextFromMcpBody/_meta`, `validTracestate/validBaggage` | 25–73, 75–182 | Only W3C trace ctx (caller-injected); `{}` in CLI; bounded/validated |
| `src/engine/models/catalog.ts` | `DEFAULT_QWEN_*_ENDPOINT` (`https://dashscope.aliyuncs.com/...`) | 1–4 | Remote endpoints |

## B6 — Logs & redaction
| File | Symbols | Lines | Notes |
|---|---|---|---|
| `src/daemon/logger.ts` | `createDaemonLogger`, `sanitizeFields` (`/token|api.?key|authorization|query/i`, 512 trunc), log file 0600 | 14–64 | Redaction at source |
| `src/cli/errors.ts` | `redactErrorText`, `modelDownloadFailureMessage` | 19–77 | `[redacted]` in error contexts |

## B7 — Model download (supply chain)
| File | Symbols | Lines | Notes |
|---|---|---|---|
| `src/engine/models/backends/model2vec.ts` | `resolveCachedFile` (`https://huggingface.co/${repo}/resolve/${revision}/${file}`, `redirect:"follow"`), `download`, `isUsableModelFile` (size>0 only), `DEFAULT_MODEL_CACHE_DIR=~/.zvec-grep/models` | 58–96, 281–357, 385–387 | **No checksum/signature verification** |
| `src/engine/models/catalog.ts` | `local/potion-*` entries (`repo`, pinned `revision` commit hash) | 106–158+ | Reproducibility pin present; integrity check absent |
| `src/cli/help.ts` | "Local models are downloaded to the model cache on first use" | 381 | Documents auto-download |

## B8 — Lifecycle / lease
| File | Symbols | Lines | Notes |
|---|---|---|---|
| `src/daemon/root-lease.ts` | `RootLeaseManager` (pid/hostname/instanceToken, 5 s heartbeat, abandoned recovery) | 27–242 | Index-write mutex; no OS elevation |
| `src/engine/utils/daemon-lease.ts` | `acquireDaemonLeaseGuard` | — | Cross-process lock primitive |

## Upstream test files (coverage evidence)
`test/server-http.test.mjs`, `test/mcp-request-state.test.mjs`, `test/authorization.test.mjs`,
`test/job-scheduler.test.mjs`, `test/daemon-logger.test.mjs`, `test/trace-context.test.mjs`,
`test/mcp.test.mjs`, `test/mcp-modern-http.test.mjs`, `test/mcp-legacy-http.test.mjs`,
`test/config.test.mjs`, `test/config-cli.test.mjs`, `test/stdio-bridge.test.mjs`,
`test/root-lease.test.mjs`. (Suite is Node `node:test` `.mjs`; not executed under W18-R
no-dependency-install boundary.)
