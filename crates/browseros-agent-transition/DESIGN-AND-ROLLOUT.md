# Agent Channel: zero-touch investigation and migration plan

Scope: `code5m/browser-os`, read baseline
`perf/p0-p2-20261031@96ccd6384314cf747d73a31409f858a083939803`.

The goal is to **prepare** a unified AgentRuntime without changing any
previously-existing feature or infrastructure. These experimental contracts
are not claims that production defects are already fixed.

## Findings and precise source points

| Finding | Evidence | Candidate boundary |
| --- | --- | --- |
| Two execution surfaces | `src/capabilities/agent/state/useAgentStore.ts` guards legacy backend; `src/bridge.ts` sets `AGENT_SKILL_COMMANDS_AVAILABLE=false`; `src-tauri/src/framework/ai.rs` exposes `coding_service` | `runtime::AgentRuntimePort`, `LegacyEntry`, `CutoverRouter` |
| Multi-file partial apply | `src-tauri/src/capabilities/agent/coding.rs` lines 910-981: backups, then sequential atomic writes and intermediate session saves | `apply::ApplyCoordinator`, conditional compensation, host journal |
| Concurrency | `coding.rs` lines 832-850: reads outside workspace lock; chats use the lock; `framework/ai.rs` per-provider permit | `concurrency::SessionRevisions` + `FileLeases` |
| Hotplug physical drift | `src/capability/platform/manager.ts` disables, deactivates async, then compensates mostly UI metadata on errors | `lifecycle::CapabilityMachine`, evidence-bearing resource leases |
| Skills/plugins are not inherently executable | legacy agent and skill README and existing permission gates | same approved `ToolCatalog`/ToolPort from Agent Channel, with no second executor |

Source links:
- https://github.com/code5m/browser-os/blob/96ccd6384314cf747d73a31409f858a083939803/src/capabilities/agent/state/useAgentStore.ts
- https://github.com/code5m/browser-os/blob/96ccd6384314cf747d73a31409f858a083939803/src-tauri/src/capabilities/agent/coding.rs#L910-L981
- https://github.com/code5m/browser-os/blob/96ccd6384314cf747d73a31409f858a083939803/src-tauri/src/framework/ai.rs
- https://github.com/code5m/browser-os/blob/96ccd6384314cf747d73a31409f858a083939803/src/capability/platform/manager.ts

## Proposed single-runtime architecture

```text
EXISTING UI / IPC (unchanged during this branch)
  agent_*(currently disabled) | coding_service (working)
                    |
           [future opt-in adapter]
                    |
          AgentRuntimePort (one)
           /       |        \
        Coding   Browser   Database / Skills
           \       |        /
          original approved tool owner
                    |
    journal / session CAS / lease / lifecycle
```

The future adapter must reuse existing provider credentials, audit paths,
security checks, file scope validation, approval semantics and Tauri command
names. It may translate state/events; it must **not** implement a parallel
Coding Agent execution engine.

## Compatibility rules

1. Default `ExistingOnly`: all old APIs and disabled statuses remain
   byte-for-byte behaviorally unchanged. This crate has no feature hook.
2. `ShadowReads`: only deterministic, user-authorized, **read-only**
   operations may be compared. No extra provider requests, file edits,
   terminal commands, browser submissions or external API calls.
3. `OptIn`: explicit host-verified decision per entry point and action;
   user consent and ToolCatalog authorization remain separate checks.
4. Once the new port starts a side-effecting operation, **never** fall back
   by replaying it through the old runtime; return a recoverable status.
5. Preserve old `coding_service` result JSON, event order/types,
   cancellation, progress, draft/apply semantics, backup locations and
   errors through a separately tested compatibility adapter.
6. `agent_chat` / `agent_run` / `agent_install` must stay unavailable
   until a complete backend, permissions and UI gate exist; no fake success.
7. Skill/Plugin registration alone is not a grant. Only the same host
   permission authority can mint a scoped, one-use tool approval.

## Multi-file safety design (compensating saga)

```text
authorize -> canonicalize/validate all -> inspect versions
  -> write/fsync journal PREPARED
  -> journal MAY_HAVE_WRITTEN -> CAS write (op ID)
  -> journal APPLIED_STEP -> next file
  -> journal APPLIED (committed)
  fail/crash: inspect provenance -> reverse CAS compensation
  mismatch/uncertain: RECOVERY_REQUIRED, preserve user edits
```

An individual atomic rename is not a cross-file transaction. Host adapters
must implement atomic+fsync journal persistence, content snapshots/backups,
cryptographic revision/identity checks, TOCTOU/symlink-safe operations,
cross-process coordination, user privacy, disk-capacity checks, exact
operation provenance and a recovery user interface. An in-memory fake test
is **not** evidence these native properties have been delivered.

A crash between write and journal update is resolvable automatically only
when trusted provenance proves that file belongs to the same `apply_id`.
Without this proof, stop and surface a conflict. Never overwrite external
edits merely because their content matches a proposed replacement.

## Concurrency

Model requests do not hold workspace/session write locks. Run independent
model tasks concurrently with a bounded Provider limiter. Save session
state through revision CAS; reject stale commits. Acquire leases only for
specific canonical files during short commit windows in a consistent order.
A real adapter must implement the durable cross-process versions of these
contracts, not rely on in-memory Mutex after crashes/restarts.

## Hotplug lifecycle

Use `DISABLED -> ACTIVATING -> ACTIVE -> SUSPENDING -> SUSPENDED
-> DEACTIVATING -> DISABLED` with `FAILED -> RECOVERING -> verified
state` when resource release/activation is uncertain. Failures never
relabel a partially released capability as ACTIVE solely by restoring a
boolean. The host must verify process, IPC listener, WebView, UI contribution
and any other resource-specific owner before approving recovery. These
contracts cannot alter the existing TypeScript manager until a later PR.

## Release gates (future, separate PRs)

| Phase | Change | Must pass before proceeding |
| --- | --- | --- |
| 0 (this branch) | Add unregistered Rust migration crate + injected failures | Original files untouched; crate compiles and tests separately |
| 1 | Host-owned adapter behind OFF-by-default switch | Exact old IPC/API snapshots, no startup/permission change |
| 2 | Read-only shadow and differential tests | Same data/scope; zero writes and no duplicate provider calls |
| 3 | Opt-in Coding Agent session routing | Long streaming/cancel/error/approval parity, session CAS tests |
| 4 | Opt-in multi-file apply with durable journal/backup | Kill -9 recovery, 3rd-file failure, symlinks, disk full, external edit |
| 5 | Capability lifecycle opt-in with real leases | Failed cleanup injection, crash/restart, resource inventory validation |
| 6 | Legacy Agent + Skill/Plugin execution using same runtime | Explicit permission, C/HP gates, old API compatibility, clean rollback |

Rollback: disable the new routing policy for **new tasks**. In-flight writes
must finish/recover under the owning engine/journal before switching; do not
replay them in the legacy runtime. Existing sessions keep their owner until
drained or explicitly migrated with validated state snapshots.

## No existing code was modified here

This companion crate and its tests/documentation are new paths under
`crates/browseros-agent-transition/` only. There is no Tauri registration,
Cargo workspace edit, lockfile change, CI configuration change, npm package,
frontend toggle or automatic migration. Integration and full production
problem resolution explicitly remain future work.
