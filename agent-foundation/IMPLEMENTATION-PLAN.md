# Agent Channel Rust Foundation — execution plan

Status as of 2026-11-01: **isolated development only**. Production integration is explicitly out of scope.
Repository: code5m/browser-os. Branch: feat/agent-channel-rust-foundation-20261101.
Baseline: master 9607e260f86b15a730219c619ab5a20cc01efe65.

## Product decisions

- A single Agent Channel for the browser, editor, terminal, database and Git; specialized Agents contribute only strategy/context/tools.
- BrowserOS browser-style UI stays the product interface. This crate supplies no UI and owns no WebView.
- Rust implements the bounded orchestration and policy boundary; official Rust SDKs are external adapters rather than source-code forks.
- Keep exactly one owner of each tool side effect. Reuse existing BrowserOS Public APIs after separate approval, never reach into feature internals.
- Model output and MCP results are untrusted. Models may **propose**, not authorize or silently execute.

## Work breakdown and acceptance gates

| Priority | Deliverable | Acceptance requirement | Status |
|---|---|---|---|
| P0 | Isolated crate, typed model/task/context/tool/event contracts | Cargo fmt/check/test/clippy succeed independently; no pre-existing files modified | CODE_ADDED / VERIFICATION_PENDING |
| P1a | Single model turn with bounded stream, timeout, explicit proposal | Mock provider -> task -> one pending approval; no tool executes before host approval | CODE_ADDED / VERIFICATION_PENDING |
| P1b | Trusted approval boundary and cancellation | Unknown tools denied, approval one-use and task-scoped, cancellation rejects stale replies | CODE_ADDED / VERIFICATION_PENDING |
| P1c | Recovery and event consistency | Snapshot restore drops pending side effects; monotonic sequence; replay gap signal | PROTOTYPE / VERIFICATION_PENDING |
| P2a | Rig adapter | Adapter tests for provider errors, streaming, tool calling, usage accounting | NOT_STARTED |
| P2b | MCP official Rust SDK adapter | Tool discovery/allowlist, stdio process isolation, argument/output validation | NOT_STARTED |
| P2c | ACP Rust adapter | Editor-to-agent RPC compatibility, cancellation/approval boundary; no Tauri integration | NOT_STARTED |
| P2d | A2A + CloudEvents (optional) | Inter-agent tasks and external event bridge only if needed; v1 envelope tests | NOT_STARTED |
| P3a | Durable task/outbox storage | Atomic task+event commits, recovery with schema upgrades, explicit user resume | NOT_STARTED |
| P3b | Security hardening | Authenticated principal-bound permits, scoped resources, secret filtering, injection tests | NOT_STARTED |
| P3c | Resource governance | Per-task lease, bounded queues, parallel work limits, external effect reconciliation | NOT_STARTED |
| P3d | System testing | Benchmark, fault injection and real external-provider tests; baseline comparisons | NOT_STARTED |
| P4 | Integration RFC only | Reuse existing agent/coding/MCP APIs, document exact file ownership and migration | RESEARCH_ONLY |

## Definition of done

1. No edits outside agent-foundation/; compare against master to confirm all existing paths untouched.
2. Independent crate passes cargo fmt/check/test/clippy on authorized developer machine or independent CI.
3. No new Tauri command, no browser/editor/terminal filesystem access, no app dependencies or modifications.
4. No model-originated approval or process execution; destructive effects require authenticated explicit user grant.
5. Cancellation, timeout, crash recovery and unknown-outcome behavior independently tested.
6. No claim of real SDK compatibility before protocol-interoperability tests.
7. Real GUI, full validation and installed package checks remain separate gates and cannot inherit this crate's result.

## Key dependencies to evaluate, not yet vendored

- Rig: https://github.com/0xPlaygrounds/rig (MIT; check versions/MSRV and dependencies before enabling).
- Official Rust MCP SDK: https://github.com/modelcontextprotocol/rust-sdk (verify exact crate license, features and standards compatibility).
- ACP SDK: https://github.com/agentclientprotocol/rust-sdk (Apache-2.0).
- A2A Rust SDK: https://github.com/a2aproject/a2a-rs (Apache-2.0; only for external agent cooperation).
- CloudEvents Rust SDK: https://github.com/cloudevents/sdk-rust (Apache-2.0; optional transport bridge).

## Risks and mitigations

- Existing BrowserOS has both legacy Agent shell and working Coding Agent. Do not create competing owner; integration only through a later explicit migration.
- GitHub connector writes files but cannot run cargo. Checks remain NOT_RUN until real compiler evidence exists.
- A tool future may be cancelled while a remote side effect still completes: require idempotency keys and explicit reconciliation before production.
- Snapshot serializes user goal and opaque resource identifiers: do not persist without host security controls.
- GUI and Grid/WebView behavior are unrelated to this independent crate and must not be inferred from unit tests.

## Next work order

1. Run real independent crate build/test and fix any compiler or logic findings in this folder only.
2. Add principal+resource+effect-bound host authorization and a task execution lease.
3. Add durable snapshot/event outbox as an opt-in storage adapter (not automatically active).
4. Evaluate real Rig and RMCP adapters behind optional cargo features, write contract tests before adopting.
5. Re-read existing Coding Agent and MCP implementations and refine the integration RFC; do not wire them up.
