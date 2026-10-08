# Capability Platform v4 — Worthwhile Hot-Plug Final Audit — 2026-10-08

## Verdict

**V4_WORTHWHILE_HOTPLUG_PASS**

Capability Platform v4 completes the worthwhile B -> A hot-plug sweep. The implementation HEAD passed the complete repository validation chain without relaxing build thresholds, baselines, Hot-Plug acceptance, GUI acceptance, or pre-merge gates.

## Maturity result

Starting point inherited from the frozen v3 audit:

`A=7, B=14, C=0, D=1`

Final v4 result:

`A=15, B=6, C=0, D=1`

Final A set:

1. bookmark
2. vault
3. home
4. graph
5. workspace
6. clipboard
7. database
8. apps
9. plugin
10. git
11. tools
12. browser
13. grid
14. terminal
15. script

The exact A set is enforced by `scripts/check-v4-worthwhile-promotions.mjs`.

## v4 promotions

### Apps — B -> A

- non-resident C3 / HP2 capability;
- contribution withdrawal and restore are reversible;
- lifecycle can suspend and reactivate the capability-owned surface;
- launched external applications remain OS-owned after spawn and are therefore outside capability teardown ownership;
- unified Hot-Plug Acceptance passes repeated lifecycle cycles and restart persistence.

### Plugin — B -> A

- the current capability is a plugin registry/management surface, not an executing unloadable plugin runtime;
- management operations expose a busy guard and reject suspension while unsafe;
- idle lifecycle cleanup, contribution removal/restore, repeated cycles and restart persistence pass the unified Harness.

### Git — B -> A

- write operations are guarded as non-suspendable while in flight;
- idle suspend invalidates capability-owned transient read/preview state rather than killing an active write;
- explicit Git UI lifecycle wiring remains statically auditable;
- unified Hot-Plug Acceptance verifies busy rejection, cleanup, contribution restoration and restart behavior.

### Tools — B -> A

- tool-* WebViews have a capability-level close-all cleanup path;
- suspend/deactivate cleanup is reversible;
- contribution withdrawal/restore and restart persistence pass the unified Harness.

### Browser — B -> A

- Browser owns ordinary tab WebViews only; Grid remains a separate owner;
- suspend freezes/hides Browser-owned WebViews;
- deactivate closes Browser-owned tab resources and clears owner state;
- resume restores owner visibility behavior;
- cleanup-failure rollback keeps lifecycle internal state aligned with Runtime ACTIVE state;
- packaged GUI and Full Tauri GUI regressions pass.

### Grid — B -> A

- Grid owns grid-* WebViews and their child-process lifecycle;
- suspend freezes/moves Grid resources out of view without corrupting Browser ownership;
- deactivate uses the real close-grid teardown path;
- crash recovery, view switching, normal-tab coexistence, context menu, and login-state restart scenarios pass Full GUI regression;
- scenario evidence collection is deterministic and does not weaken assertions.

### Terminal — B -> A

- active PTY sessions reject suspend/deactivate rather than being silently killed;
- idle lifecycle cleanup is reversible;
- owner lifecycle remains aligned with Runtime after cleanup failure;
- unified Harness verifies busy rejection, contribution cycling and restart behavior.

### Script — B -> A

- manually tracked active runs reject suspension while still running;
- completed runs can be removed and the capability can suspend/restart safely;
- the shared Task scheduler/runner remains outside Script lifecycle ownership and is not terminated by Script disable.

## Remaining B — final decisions

| Capability | Final decision | Reason |
| --- | --- | --- |
| skill | KEEP B | Execution/install backend is intentionally incomplete. A read-only parse/validate/permission-preview shell should not claim a stronger execution lifecycle than exists. |
| agent | KEEP B | The real agent execution/streaming backend is incomplete, so there is no truthful end-to-end execution resource lifecycle to hot-unplug yet. |
| task | KEEP B | Scheduler is process-wide background infrastructure whose scheduled semantics must survive UI absence; frontend capability disable does not own the scheduler. |
| credential | KEEP B | Resident OS-keyring security infrastructure is intentionally non-disableable because other capabilities depend on the security service contract. |
| session | KEEP B | Resident persistence/shutdown-order infrastructure; hot disabling it would change durability semantics. |
| workbench | KEEP B | Resident Shell/navigation host for other capabilities; disabling it would remove the platform surface itself. |

These six are intentional product/runtime boundaries, not unfinished v4 implementation work.

## Lifecycle reliability hardening

v4 additionally closes lifecycle reliability debt discovered during the sweep:

- common managed Hot-Plug lifecycle helper for reversible capability owners;
- internal `active` state is committed to inactive only after cleanup succeeds;
- cleanup failure therefore remains consistent with Runtime, which keeps the capability ACTIVE when a lifecycle hook rejects;
- Browser, Grid, Plugin and Terminal cleanup-failure behavior is covered by deterministic negative acceptance probes;
- repeated capability lifecycle/contribution assembly was consolidated without changing public contribution IDs or static UI safety contracts.

## Build-footprint work

The first complete v4 implementation exceeded the existing build-metrics ceiling. The sweep did **not** modify the threshold or baseline.

The implementation was reduced by:

- deduplicating managed lifecycle state machines;
- centralizing lifecycle + contribution assembly where existing static safety gates allow it;
- preserving explicit Browser/Terminal/Git wiring where static architecture checks require source-visible ownership;
- trimming duplicate runtime diagnostic text;
- moving common HP2 evidence to `defineIntegratedCapability` while retaining source-visible C3 evidence markers for static auditing.

Final pre-merge build-metrics comparison passes against:

`logs/m0-build-metrics/build-metrics-052b18a.json`

with the existing limit unchanged.

## Final implementation evidence

Validated on GitHub Actions run `37732612292` for implementation HEAD:

`aa3c6714f7fd8ab425dc6ddf350cad80d74b228c`

Observed evidence:

- UI Safety PASS
- Unified Hot-Plug Acceptance PASS (`HOT_PLUG_ACCEPTANCE_RESULT=PASS`)
- exact v4 A-promotion gate PASS:
  `V4_A_PROMOTION_RESULT=PASS A=15 ids=bookmark,vault,home,graph,workspace,clipboard,database,apps,plugin,git,tools,browser,grid,terminal,script`
- Capability Runtime PASS (`CAPABILITY_RUNTIME_RESULT=PASS (16/16)`)
- Pluggable Runtime PASS
- Runtime resource absence PASS (`12/12`)
- architecture/capability checks PASS
- Runtime startup PASS
- Production Build PASS
- Rust format/check PASS
- Rust tests PASS: `462 passed; 0 failed; 2 ignored`
- packaged GUI cold-start PASS: `0 failure(s), 2 warning(s)`
- Full Tauri GUI Regression PASS: `M0_6C_GUI_REGRESSION_RESULT=PASS`
- branch whitespace diff check PASS
- Git object integrity PASS: no corrupt/missing objects
- build metrics gate PASS against the frozen baseline
- `PRE_MERGE_RESULT=ALL_PASS`

The two packaged-GUI warnings are environment portal/service availability warnings and are not acceptance failures.

## Acceptance rule

v4 is considered complete only if this audit-only commit also passes the same normal repository CI gates. No v4 gate is waived by this document.
