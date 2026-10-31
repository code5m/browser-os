# BrowserOS Agent Channel (isolated Rust contracts)

## Status

**Experimental / not connected to production.** This is a small standalone Rust
crate, based on `perf/p0-p2-20261031`. It creates an internal protocol boundary
for a future AI Agent Channel without altering or registering any existing code.

It is deliberately located under `crates/`, **not `packages/*`** (the npm
workspace glob), and is **not** part of `src-tauri/Cargo.toml`. No main
application Cargo manifest, lockfile, feature, Tauri command, frontend store,
permission, MCP server, workflow, or engineering governance file is modified.

## What has actually been implemented

| Internal module | Working contract / logic | Future upstream adapter |
| --- | --- | --- |
| `model` | model-neutral request/events, text-only OpenAI-style request JSON, async adapter trait | Rig |
| `agent` + `interop` | guarded task state machine and typed external-agent adapter seam | ACP (editor), A2A (agent-to-agent); Goose for architecture ideas |
| `tools` | tool catalog, default-deny exact scope / one-use approval gate, execution port | official Rust MCP SDK (RMCP) |
| `events` | ordered per-task in-memory event log, resume cursor, CloudEvents 1.0 JSON envelope | CloudEvents SDK / Tokio transport |
| `context` | explicitly selected context reference contract | future Browser/Editor/Terminal adapters |

All implementation in this crate is original. **No upstream source is copied,
vendored, or forked** and no upstream binary/SDK is linked into the running app.
Names of ACP/A2A/Rig/RMCP identify planned adapter boundaries, not claims that
their official SDKs or full wire specifications are already supported.

Upstream reference repositories:
- Rig: https://github.com/0xPlaygrounds/rig
- ACP Rust SDK: https://github.com/agentclientprotocol/rust-sdk
- A2A Rust SDK: https://github.com/a2aproject/a2a-rs
- MCP Rust SDK: https://github.com/modelcontextprotocol/rust-sdk
- CloudEvents Rust SDK: https://github.com/cloudevents/sdk-rust
- Goose (architecture reference): https://github.com/aaif-goose/goose

## Run isolated tests

```sh
cargo test --manifest-path crates/browseros-agent-channel/Cargo.toml
cargo fmt --manifest-path crates/browseros-agent-channel/Cargo.toml -- --check
```

No repository-wide build is required to *use* these isolated checks. However,
they do not replace the full BrowserOS regression suite when this crate is
eventually wired into the application.

## Trust and capability boundaries

- No I/O, model API calls, network clients, subprocesses, or environment reads.
- Context references never trigger automatic page/file/project collection.
- A model cannot turn a proposed tool call into `AuthorizedToolCall` without
  a trusted host providing an explicit, exact, single-use `Approval`.
- The approval gate operates in memory only. A future host must enforce identity,
  approval provenance, resource ownership, expiry, durable replay protection,
  and permissions again immediately before a real side effect.
- Task state `Approve` only changes task status; it does **not** grant tool rights.
- Events are in memory; replay cursors do not guarantee recovery across restart.
- The OpenAI-style JSON helper supports only a basic text subset. Provider-
  specific authentication, tool-call payload mapping, cancellation, retries,
  multimodal data, and SSE remain future adapter responsibilities.

## Integration boundary (not part of this change)

Future separate PRs may implement adapters behind explicit opt-in features and
host-controlled permission checks. Integration must require its own design and
regression review; do **not** automatically register this crate with BrowserOS.

The implementation scope intentionally excludes actual Rig/RMCP/ACP/A2A SDK
bindings, background task engines, cloud agents, browser automation, editor
writes, persistence, UI, and any changes to existing infrastructure.

License: the host repository's Mulan PSL v2. Third-party dependencies are
limited to serde / serde_json in this isolated crate; their licenses must be
reviewed before any distribution or future activation.
