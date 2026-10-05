# Capability Platform v2 — B → A Evaluation Matrix

This matrix evaluates audit-grade promotion only. Manifest maturity (C1/C2/C3) and audit grade (A/B/C/D)
are related evidence, not interchangeable labels.

A promotion requires the V2 hard criteria: runtime enable, suspend, resume, disable, contribution cleanup,
command/listener/task cleanup, no stale route/menu/action/store references, no duplicate registration,
refresh persistence, process-restart persistence, independent composition, Hot-Plug Acceptance Harness,
and capability-boundary gates.

| Capability | Manifest | Lifecycle / HP | V2 A decision | Evidence / blocker |
| --- | --- | --- | --- | --- |
| bookmark | C3 | SUSPENDED, HP2 | already A reference | Existing reference implementation; unified harness is bound to its maturity evidence. |
| vault | C3 | SUSPENDED, HP2 | A candidate | Non-resident, light/cache-only, explicit Host ports, register/unregister; must pass the unified restart-aware Hot-Plug Harness before audit promotion. |
| home | C2 | SUSPENDED, HP2 | keep B | Structurally close, but required browser/workspace/apps dependency assembly and restart/hot-plug evidence are not yet in the V2 harness. |
| clipboard | C2 | BACKGROUND, HP0, resident | keep B | Resident static capability; no runtime disable/unregister contract. |
| git | C2 | SUSPENDED, HP0 | keep B | Process/credential/workspace dependencies; no HP1/HP2 contract or process cleanup acceptance. |
| database | C2 | SUSPENDED, HP0 | keep B | Database handle/credential lifecycle has no runtime hot-plug acceptance. |
| terminal | C3 | ACTIVE only, HP0 | keep B | PTY/child-process owner is intentionally not suspendable; cannot satisfy A criteria. |
| plugin | C2 | ACTIVE only, HP0 | keep B | No executable plugin runtime/loader; dynamic lifecycle intentionally unavailable. |
| skill | C1 | SUSPENDED, HP0 | keep B | Execution backend incomplete and no runtime enable/disable registration path. |
| agent | C1 | SUSPENDED, HP0 | keep B | Agent/Graph dependency cycle is removed, but hot-plug lifecycle remains unimplemented. |
| graph | C2 | SUSPENDED, HP0 | keep B | Independently assemblable after V2 decoupling, but no HP lifecycle contract. |
| browser | C3 | declared SUSPENDED but resource suspendable=false, HP0 | keep B | Native WebView/Grid resource lifecycle prevents safe runtime disable. |
| workspace | C3 | declared SUSPENDED but resource suspendable=false, HP0 | keep B | Static composition; no safe complete teardown contract. |
| apps | C2 | SUSPENDED, HP0 | keep B | Detached child applications are not owned through a reversible runtime lifecycle. |
| tools | C2 | SUSPENDED, HP0 | keep B | No HP enable/disable/register/unregister evidence. |
| task | C2 | BACKGROUND, HP0 | keep B | Scheduler thread is process-level singleton and cannot be hot-unloaded independently. |

## V2 promotion policy

- No capability is promoted merely because its manifest says `SUSPENDED`.
- HP0 capabilities remain B even when their UI is contribution-driven.
- Native/process capabilities remain B until owner-level resource shutdown and restart behavior is measured.
- A candidate becomes audit-grade A only after `scripts/check-hot-plug-acceptance.mjs` executes its real
  definition and contribution lifecycle and the full capability boundary suite remains green.
- Infrastructure/runtime/shell/settings are not forced into A.
