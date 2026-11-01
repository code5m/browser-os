# BrowserOS Agent Channel — standalone Rust foundation (prototype)

> **EXPERIMENTAL / NOT INTEGRATED / NOT BUILD-VERIFIED**. This folder is an
> independent Rust crate added on a new branch from \`master\`.
> It is **not** a replacement for existing Agent, CodingWorkbench or MCP code.
> No existing application source, Tauri command, app manifest, workflows or
> other branch is modified.

## What has been implemented

- \`model.rs\`: vendor-neutral async streaming \`ModelProvider\` trait and typed model events. Inspiration: [Rig](https://github.com/0xPlaygrounds/rig). **Rig is not yet a dependency or functional provider adapter**.
- \`protocol.rs\`: versioned in-process command envelope; no model-to-approval action. This is **NOT an ACP or A2A wire implementation**.
- \`context.rs\`: explicit, bounded references to browser/editor/terminal/workspace resources; no automatic harvesting and no filesystem reads.
- \`task.rs\`: task lifecycle, session references, proposal/approval/denial, one-time non-serializable execution permit, optimistic revision checks, cancellation signal, JSON snapshot import/export. Recovery drops pending approvals and pauses interrupted tasks.
- \`tool.rs\`: registration of explicit tools, effect classification, trusted-host approval permit requirement, execution timeout and cooperative cancellation. **No actual MCP, browser, filesystem or process tool registered**.
- \`event.rs\`: typed metadata-only event journal, bounded in-memory replay, live broadcast and CloudEvents-compatible JSON projection. **No official CloudEvents SDK or durable outbox**.

## Try locally (only if explicitly authorized to use a local Rust environment)

\`\`\`sh
cargo fmt --manifest-path agent-foundation/Cargo.toml -- --check
cargo check --manifest-path agent-foundation/Cargo.toml
cargo test --manifest-path agent-foundation/Cargo.toml
cargo run --manifest-path agent-foundation/Cargo.toml --example dry_run
cargo clippy --manifest-path agent-foundation/Cargo.toml --all-targets -- -D warnings
\`\`\`

The GitHub connector used to create this branch cannot run Cargo. All checks
above are **NOT RUN** as of this commit. Never label them PASS.
In particular, no BrowserOS GUI, app integration, package or real provider
acceptance has been conducted.

## Important caveats

1. **Task state and event journal are not transactional.** The implementation
   is in-memory, not durable. Restore JSON is a prototype only: production
   snapshots need an access-controlled and encrypted store, bounded retention,
   schema migrations, atomic writes, and a durable event outbox.
2. **Approval is in-process, not an authenticated protocol.** A future host
   must authenticate the user and scope a grant to \`task_id\`, \`call_id\`,
   \`workspace\`, \`principal\`, \`resource revision\` and effect. Tool
   implementations must revalidate each resource on execution.
3. **Dropping a future does not stop non-cooperative side effects.** A remote
   service or subprocess may finish despite cancellation/timeouts. External
   tools need idempotency keys, process supervision, and explicit outcome
   reconciliation. Never retry unknown outcomes automatically.
4. **No tool output/prompt is recorded in the event journal**, but task
   snapshots currently contain goal and context identifiers, and pending
   operations can contain sensitive arguments. Do not persist without redaction,
   encryption and explicit user consent.
5. **Concurrent tool execution is not serialized per task.** Production work
   needs an explicit task lease, per-resource locks and CAS-protected final
   commits. This scaffold is not safe for arbitrary untrusted tool executors.
6. **The model runtime performs only one bounded turn with explicit approval gates**, not autonomous tool planning/execution. Real provider transport remains a trait.
   Actual provider auth, cost accounting, retries, model capability
   negotiation, bounded transport, streaming backpressure, and prompt-injection
   boundaries remain unimplemented.

## Why this layout

\`\`\`text
BrowserOS (existing; unchanged)
  ├─ Browser / Grid / WebView
  ├─ Editor / CodingWorkbench / LSP
  └─ Legacy Agent / MCP
             |
      [NO CONNECTION]
             |
agent-foundation (new isolated prototype)
  ├─ engine.rs         single-turn runtime, never auto-executes tools
  ├─ model.rs          model stream contract
  ├─ protocol.rs       versioned agent command envelope
  ├─ context.rs        explicit scoped resource references
  ├─ task.rs           task lifecycle + approval
  ├─ tool.rs           tool registry + guarded execution
  └─ event.rs          event journal + replay
\`\`\`

Recommended integration order **after explicit separate approval**:
(1) align shared task/event vocabulary, (2) build durable task/outbox storage,
(3) implement authenticated approval broker, (4) adapt existing Coding Agent,
(5) adapt browser/terminal via existing Public APIs, (6) add ACP/MCP/Rig
connectors and GUI regression tests. Never turn on old Agent commands simply
because the scaffold exists.

## External projects to evaluate, not copied or linked

- [Rig](https://github.com/0xPlaygrounds/rig): model abstraction.
- [Agent Client Protocol Rust SDK](https://github.com/agentclientprotocol/rust-sdk): editor ↔ agent wire protocol.
- [A2A](https://github.com/a2aproject/A2A): inter-agent task collaboration (optional, different from ACP).
- [official MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk): tool transport and resource discovery.
- [CloudEvents Rust](https://github.com/cloudevents/sdk-rust): event interoperability.

Before adding dependencies, inspect latest versions, MSRV, transitive licenses,
security advisories, native dependency overhead and cargo-deny policy.
The code in this folder is original scaffolding; it does not vendor these
projects. The repository root license is MulanPSL-2.0.

## Delivery tracking

See [IMPLEMENTATION-PLAN.md](IMPLEMENTATION-PLAN.md). Source and test cases are committed, but actual Cargo compilation has not been run from the GitHub-only session. Treat compilation, clippy and examples as **UNVERIFIED**.
