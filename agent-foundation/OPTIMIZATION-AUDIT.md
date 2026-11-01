# BrowserOS Agent foundation — source-aware optimization audit

Baseline: code5m/browser-os master @ 9607e260f86b15a730219c619ab5a20cc01efe65 (2026-11-01).
All items below are **read-only findings and proposals**; no existing application code was modified.

## Confirmed implementation facts and exact migration candidates

| Priority | Existing path / evidence | Observed behavior | Future improvement, separately authorized |
|---|---|---|---|
| P0 | src-tauri/src/capabilities/agent/coding.rs:808-851, 987-1040 | Read-only tree/read/search bypass a workspace mutation lock, but chat mutates its session while holding a per-workspace mutex; the model provider is invoked on this path. | Separate session-state mutations from long model transport; use per-session revision/CAS and one task execution lease while preserving existing file consistency and read-only bypass. Benchmark before changing locks. |
| P0 | src-tauri/src/capabilities/agent/coding.rs:823-850, 1004-1030 | Running session and cancellation flag are tracked by workspace in a global map; serialization is per-workspace. | Clarify lifecycle ownership: Agent Task owns cancellation, workspace owns file revision. Stale results must be rejected after pause/cancel. |
| P0 | src/capabilities/agent/README.md and state/useAgentStore.ts | Legacy Agent entry is readonly/guarded while CodingWorkbench has real coding service execution. | One Agent Channel owner: adapt legacy definitions and the existing Coding Agent, do not implement competing execution engines or enable absent commands. |
| P1 | src/capabilities/agent/ui/CodingWorkbench.vue (330 lines, ~52 KB) | A single component tracks buffers, project epochs, resources, LSP requests, session recovery, model streaming, MCP approval and resizing. | Split by responsibility into workspace state, agent session state, editor model service and protocol adapters; retain one state owner and add stale-response tests. |
| P1 | src/capabilities/agent/ui/CodingWorkbench.vue:229-256 | send() builds an authorized context, receives streaming chunks, checks project epoch and supports cancelRun(). | Keep review/recovery semantics; standardize bounded events with sequence/revision checks, protect against remount duplicate sends, and measure UI render pressure. |
| P1 | src-tauri/src/capabilities/agent/local_provider.rs:43-88 | Local Ollama adapter uses blocking TcpStream; connect timeout is 2s and read timeout is 25s. | Measure call-site threading and GUI impact; use a dedicated blocking worker or async transport only if benchmark justifies it while preserving loopback and limits. |
| P1 | src-tauri/src/capabilities/agent/coding_lsp.rs:25-26, 760-905 | Persistent LSP session cache already exists: max 2 and 300s idle TTL with per-session JSON-RPC serialization. | **Do not reimplement persistent sessions.** Benchmark warm/cold latency, cache hit rate, RSS, respawns; preserve scratch sandbox, source hash and project-level consent. |
| P1 | src-tauri/src/mcp.rs and mcp_server.rs:213-289 | MCP registry and allowlist exist; feature-gated stdio server binds two readonly introspection operations; other tools fail closed. | Reuse one policy owner. Official RMCP belongs in a transport adapter, not a second registry; keep unbound operations denied. |
| P2 | src/capabilities/agent/public.ts and manifest.ts | Capability Public API currently re-exports store/manifest and lists optional browser/git/database dependencies. | Add a narrow Agent Channel API only after explicit migration approval. Domain owners supply approved tools rather than exposing internals. |
| P2 | src/capabilities/agent/README.md vs state/useAgentStore.ts | README still describes co-owned Skill state while store comments document Skill migration to its own store. | Align outdated documentation with source, without changing current ownership or runtime behavior. |
| P3 | AGENTS.md and PROJECT-RULES.md | Native WebView layout and visibility follow locked-down rules and real GUI acceptance gates. | Leave native GUI untouched in this branch. Future profiling should cover Grid IPC queue, WebView memory RSS and installed-client cold launch. |

## Single ownership model for future integration

- Agent Channel: task/session lifecycle, event stream, cancellation, user approval, durable recovery.
- Model Adapter: model negotiation, bounded transport, errors and token usage; owns no terminal/file side effects.
- Tool Registry: catalog and policy metadata only; adapters call each existing capability's Public API.
- Browser/Grid: tab/navigation and WebView lifecycle; Agent only receives explicitly authorized context.
- Editor/LSP: text model, buffer hashes, review/diff; Agent only proposes changes.
- Terminal: process supervisor and command confirmation; never let Agent silently spawn.
- Git/Database: existing business owners; readonly context first, explicit approval for mutation.

## Non-negotiable acceptance matrix

1. Unique Owner and stable Public API, no internal cross-module access.
2. All native commands, tasks and processes have clear ownership and lifecycle.
3. Writes, commands, outbound effects and retained context require scoped explicit authorization.
4. Isolated tests, fault injection, cancellation/restart tests and real installed GUI verification are separate gates.
5. Performance improvement claims need before/after evidence; documentation must match code.

## Current prototype limitations and red lines

- In-memory tasks and journal; JSON snapshot not an encrypted durable transactional store.
- No actual Rig/RMCP/ACP implementation or authenticated principal-bound approval yet.
- External effects can finish after cancellation; enforce idempotency/reconciliation before release.
- GitHub-only changes are not evidence of successful Cargo compilation.
- Modify only agent-foundation/ on the independent branch; no app wiring, no master/perf/CI changes.
