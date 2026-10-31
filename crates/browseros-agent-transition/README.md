# BrowserOS Agent Transition (isolated migration experiment)

Status: **NOT CONNECTED / DEFAULT OFF / NOT A PRODUCTION FIX**.

This standalone Rust crate is an additive companion to
`../browseros-agent-channel`. It does **not** change the original Agent
Channel, `perf`, `master`, Tauri, Vue, capability infrastructure, npm
workspace, `Cargo.lock`, workflows, existing commands or runtime behavior.
It imports the previously-created Agent Channel as a path dependency.
It starts no threads, network requests, filesystem reads/writes, browsers,
MCP processes or model calls.

## Source audit / motivation

Current behavior at `perf@96ccd638`:

1. Legacy `agent_chat` / `agent_run` / `agent_install` are not registered,
   and `AGENT_SKILL_COMMANDS_AVAILABLE=false` protects the UI.
   `coding_service` is functional, with independent model/tool/session logic.
2. `coding.rs` `apply` validates and backs up files, then writes them one
   by one and persists per-file session state. Failed later writes can leave
   earlier writes applied; the old flow does not automatically compensate.
3. `coding.rs` read-only actions bypass a workspace mutex, but long chats
   still hold the workspace mutation lock, and the Provider has a bounded
   per-profile concurrency limit.
4. `src/capability/platform/manager.ts` rolls back orchestration metadata
   after an async lifecycle failure; restoring physical resources is delegated
   to the capability lifecycle hook.
5. Skill/Plugin management and executable host tools are separate contracts.
   A definition or install preview must not imply the ability to execute.

See [DESIGN-AND-ROLLOUT.md](./DESIGN-AND-ROLLOUT.md) for traceability, hard
limits and the future cutover plan.

## Added, isolated contracts

- `runtime.rs`: one `AgentRuntimePort` for Coding, Browser, Database,
  Agent Manager and Skill workloads. Exact legacy command mapping and
  fail-closed opt-in cutover; the default route always retains existing
  behavior (including intentionally disabled legacy execution).
- `apply.rs`: host-neutral compensating multi-file apply with preflight
  version checks, durable-journal port, write-ahead stages, per-file
  operation IDs, failure compensation, explicit recovery and replay protection.
  It **does not** guarantee cross-file atomicity. An ambiguous operation or
  concurrent external edit requires recovery, never blind overwrite.
- `concurrency.rs`: in-memory session revision CAS and per-file leases
  intended for short commit windows, never around slow model calls.
  A production host must provide durable multi-process equivalents.
- `lifecycle.rs`: capability state machine with FAILED/RECOVERING, lease
  evidence, stale-generation rejection, and explicit host verification.
  Failed physical cleanup never silently returns a healthy UI state.
- `tests/transition_contracts.rs`: contract and failure-injection cases,
  including mid-batch failure, post-write error, interrupted recovery,
  conflicting external changes, idempotency and failed hotplug cleanup.

## Verify this crate independently

```sh
cargo fmt --manifest-path crates/browseros-agent-transition/Cargo.toml -- --check
cargo test --manifest-path crates/browseros-agent-transition/Cargo.toml
```

These commands do not test the already-running BrowserOS application. They
require an authorized Rust environment with dependency access. There is no
claim of a passing cargo test without an actual run.

## Explicit non-goals

- Do not wire `coding_service` or `agent_chat` to this crate yet.
- Do not enable legacy Agent install/execute commands.
- Do not replace existing Coding Agent, Skill/Plugin stores or hotplug logic.
- Do not introduce a second model loop, credential store, tool executor,
  event transport or permission authority.
- Do not duplicate side-effecting calls in shadow mode.
- Do not let the model provide a trusted rollout/approval decision.
- Do not claim full transactional safety or crash durability until host
  adapters provide fsync-backed journals, per-file compare-and-swap, exact
  operation provenance, security checks and verified recovery.

The downstream adapter must use the existing Coding Agent implementation
when possible, and must be reviewed/tested separately before activation.

License: Mulan PSL v2. The crate uses only existing project dependencies.
