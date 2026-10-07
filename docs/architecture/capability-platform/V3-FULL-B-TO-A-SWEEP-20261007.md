# Capability Platform v3 — Full B -> A Maturity Sweep — 2026-10-07

## Scope

Starting point after the low-risk Home promotion:

`A=3, B=18, C=0, D=1`

This sweep processes every remaining B capability to a final engineering decision. A capability is promoted only when its real runtime lifecycle is reversible and the same unified HP2 acceptance gate passes. A retained B is not unfinished work: it has a documented architectural blocker that would require changing product/runtime semantics rather than merely finishing a safe lifecycle seam.

## Promoted in this sweep

### Graph — B -> A

Why it qualifies:

- non-resident, suspendable, destroyable;
- no owned native process/WebView/PTY;
- lifecycle gate starts inactive and is activated only by Runtime;
- suspend/deactivate increments a generation and cancels debounce/in-flight Graph work;
- late responses from a previous lifecycle generation are ignored;
- contribution removal/restore, repeated cycles and fresh Runtime persistence pass the unified Harness;
- cleanup callback execution is explicitly asserted by the Harness.

### Workspace — B -> A

Why it qualifies:

- v2 already moved Script/Snippet ownership out; stale Workspace declarations were removed;
- remaining owned runtime surface is reversible UI contribution metadata, not a native heavy resource;
- all six actual Workspace contributions are registered/unregistered through the generic Contribution Registry;
- Shell now has a capability-agnostic availability convergence rule: if a hot-pluggable workbench view disappears, it falls back to Home or the first available surface;
- Browser/Grid/Terminal native main views are excluded from that convergence and retain their own lifecycle owners;
- a deterministic Node gate validates fallback behavior;
- unified Hot-Plug Harness passes repeated disable/enable and fresh Runtime restoration.

### Clipboard — B -> A

Why it qualifies after this sweep:

- changed from resident static lifecycle to non-resident ACTIVE/SUSPENDED lifecycle;
- browser focus listener now has symmetric removeEventListener;
- Tauri focus listener saves and executes its returned unlisten callback;
- async Tauri listener registration races are fenced by a sequence token, so a late unlisten is executed immediately after suspend;
- App may still request `startClipWatch()`, but the binding starts only while the capability lifecycle is active;
- Host contribution registration preserves the package lifecycle hook and remains synchronous for bootstrap;
- unified Harness explicitly observes lifecycle binding start/stop and repeated contribution cycles.

### Database — B -> A

Why it qualifies after this sweep:

- backend DB connection objects are request-scoped/transient rather than a resident global pool;
- all known query ids are cancelled on suspend/deactivate;
- schema generation is invalidated on lifecycle transition;
- connection/query/schema/list refresh paths capture lifecycle generation and discard late responses after suspend;
- sensitive credentials remain OS-keyring references; no secret is moved into the hot-plug framework;
- Database remains dependent on resident Credential, which is registered/active during acceptance;
- async cleanup is awaited by the Manager's suspend/deactivate path;
- unified Harness explicitly observes Database cleanup execution and repeated restart/cycle behavior.

## Final A list

1. bookmark
2. vault
3. home
4. graph
5. workspace
6. clipboard
7. database

Expected final maturity after all repository gates pass:

`A=7, B=14, C=0, D=1`

## Remaining B — final decisions

| Capability | Final decision | Architectural blocker |
| --- | --- | --- |
| skill | KEEP B | Execution/install backend is intentionally incomplete; current capability is a read-only parse/validate/permission-preview shell (C1). Promoting lifecycle maturity before the actual execution contract exists would be misleading. |
| agent | KEEP B | agent_chat/run/install execution backend is intentionally incomplete (C1). No real streaming/network execution lifecycle exists to tear down and prove. |
| apps | KEEP B | launch_app spawns detached external processes. Once launched, those processes are not owned/recalled by the capability lifecycle; disable cannot truthfully mean resource teardown. |
| git | KEEP B | Git write paths spawn subprocesses behind a two-phase write gate. There is no governed cancellation/kill contract for an in-flight write subprocess, so hot disable cannot be guaranteed safe. |
| tools | KEEP B | open_tool owns independent WebView windows. No capability-level close-all/reversible WebView teardown contract exists yet. |
| plugin | KEEP B | HEAVY/NATIVE declaration with suspendable=false and destroyable=false; there is no unloadable executing plugin runtime to prove HP2 against. |
| script | KEEP B | Owns child processes and currently declares suspendable=false. Active script cancellation/termination is not yet a capability-level hot-unplug contract. |
| task | KEEP B | Scheduler is a process-wide singleton background thread. Frontend capability disable does not own/stop the scheduler and scheduled execution semantics must survive UI absence. |
| browser | KEEP B | Owns native WebViews/child processes; suspendable=false/destroyable=false. A graceful zero-residue shutdown/recreate contract is not proven. |
| grid | KEEP B | Owns multiple native WebViews/child processes and currently does not claim a SUSPENDED lifecycle. Real HP2 needs explicit multi-WebView teardown/recreate semantics. |
| terminal | KEEP B | Owns PTY + child processes and is suspendable=false. Active user shell sessions require an explicit graceful handoff/termination policy before hot disable. |
| credential | KEEP B | Resident OS-keyring security infrastructure. It is intentionally non-disableable because other capabilities rely on the security service contract. |
| session | KEEP B | Resident persistence/shutdown-order infrastructure. Hot disabling it would change application durability semantics. |
| workbench | KEEP B | Resident Shell frame and command/navigation host. Disabling it would remove the platform surface that hosts other capabilities. |

## Why these 14 are complete decisions, not deferred analysis

Each retained B has been classified by the resource or product invariant that blocks HP2. Moving any of them to A requires a separate product/runtime contract such as process ownership, WebView teardown, PTY handoff, scheduler ownership, security-service availability, or completing an execution backend. This sweep does not hide those design changes behind a maturity label.

## New deterministic gates

- `scripts/check-hot-plug-acceptance.mjs`
  - candidates: bookmark, vault, home, graph, workspace, clipboard, database
  - repeated ACTIVE/SUSPENDED/DISABLED cycles
  - contribution withdrawal/restore
  - fresh Runtime disabled/enabled persistence
  - duplicate detection
  - lifecycle cleanup probes for Graph/Clipboard/Database

- `scripts/check-v3-a-promotions.mjs`
  - exact A set is evidence-locked
  - requires C3
  - requires HP2 enable/disable/register/unregister
  - requires non-resident and SUSPENDED support
  - requires suspendable resource policy
  - requires unified Hot-Plug evidence

- `scripts/check-hot-plug-view-fallback.mjs`
  - validates generic Shell convergence when a contributed workbench view disappears
  - protects Workspace and all future hot-pluggable workbench surfaces from dead-view regressions

## Acceptance

The sweep is complete only when the latest branch HEAD passes all normal repository gates:

- UI safety
- architecture/capability checks
- Runtime startup
- production build
- Rust format/check/tests
- packaged .deb GUI cold start
- full Tauri GUI regression
- git diff --check
- full pre-merge

No threshold, baseline or test is relaxed by this sweep.
