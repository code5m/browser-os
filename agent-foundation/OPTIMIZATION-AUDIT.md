# Read-only BrowserOS impact and reuse audit

Baseline: \`code5m/browser-os\` \`master\` at
\`9607e260f86b15a730219c619ab5a20cc01efe65\`.
No pre-existing files were edited. This is **analysis**, not an assertion that
the suggested refactoring is already done.

## Existing facts confirmed from source/README

| Existing path (master) | What exists now | Potential future optimization (not executed) |
|---|---|---|
| \`src/capabilities/agent/README.md\` and \`state/useAgentStore.ts\` | Legacy Agent definition/validation/readonly UI; \`agent_chat\`, \`agent_run\`, \`agent_install\` execution disabled | Keep definition registry, extract a single Agent Session/Task Public API; avoid duplicating a second runtime in the legacy store. |
| \`src-tauri/src/capabilities/agent/agent.rs\` and \`agent_memory.rs\` | Native agent parsing/policy, memory KV | Reuse validation and sensitive-data protections; prevent parallel AgentDef and memory policy schemas. |
| \`src-tauri/src/capabilities/agent/coding.rs\` | Working coding session, scoped proposals, reviews, cancellation, and workspace locking | Prefer adapters to current review-before-write protocol; move session/tool/event vocabulary to common module later instead of a rewrite. Measure lock hold time and avoid lock across async work; distinguish cancellation intent from externally executed effects. |
| \`src-tauri/src/capabilities/agent/local_provider.rs\` | Fixed localhost Ollama adapter using \`std::net::TcpStream\` | Consider asynchronous provider transport **only with benchmarks**: blocking socket work can delay threads; preserve bound/timeout/localhost restrictions. |
| \`src-tauri/src/capabilities/agent/coding_lsp.rs\` | Rust LSP isolation and per-project consent/diagnostics | Preserve sandbox and per-project approval; audit process lifecycle and persistent-session proposal against isolation assumptions; no blanket LSP process reuse. |
| \`src/capabilities/agent/ui/CodingWorkbench.vue\` | Coding editor UI tracks buffers, session, tools, approvals, LSP and project epoch | Separate composables around lifecycle, file buffers and agent events; prefer single task-scope cancellation and late-response revision check. Do not break Monaco unsaved buffer semantics. |
| \`src-tauri/src/mcp.rs\` | Restricted MCP capability registry and policy, not a general running MCP server | Reuse existing allowlist/path/URL policy; do not create a second competing source of tool authority. |
| \`src/capabilities/agent/README.md\` | Legacy Agent and Coding Agent have different execution contracts | Normalize on one channel with specialized strategies, not separate executors plus UI chats. |

## Red lines / excluded changes

- No edits to \`src-tauri\`, \`src\`, \`tauri-browser-tabs\`, CI workflows or existing manifests.
- No new Tauri commands, IPC capability permissions, WebView sizing/visibility,
  terminal commands, local project execution or GUI layout changes.
- No removal of existing Coding Agent, MCP, or LSP safety checks.
- No automatic editing of other branches or merge into \`master\`.
- No unapproved activation of old \`agent_chat/agent_run/agent_install\`.

## Prioritized future work (requires its own separate approval)

P0 — Validate new crate with real Cargo, add canonical model/tool/task schema,
permission broker and per-task execution lease, secure state+event persistence.

P1 — Reconcile old Agent definitions and active Coding Agent without creating
duplicate owners; build adapter toward current CodingWorkbench source of truth,
and approval flow before exposing any tools.

P2 — Benchmark Grid IPC, WebView lifecycle and GUI stalls alongside async
provider/LSP workers; establish baseline; add regression acceptance for
multi-tab, editor sessions and package cold-start.

P3 — Add official MCP and ACP support as adapters, A2A only if external agent
collaboration becomes a product requirement. Keep vendor/model SDK swappable.

## Known gaps in this prototype

In-memory state, replay, no real provider, no MCP transport, no ACP/A2A wire
compatibility, no authentication, no real FS/shell/browser tools, no external
side-effect guarantees, no task actor/lease, no real model loop, no installed GUI
validation. Describing it as production-ready would be incorrect.
