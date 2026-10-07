# Capability Platform v2 — Final Modularity Audit — 2026-10-05

## Final verdict

**V2_PASS**

Capability Platform v2 is complete against the v2 scope. The frozen v1 history remains unchanged.

## Maturity

Before:

`A=1, B=15, C=5, D=1`

Final:

`A=2, B=19, C=0, D=1`

Evidence-based promotions:

- C -> B: `script`, `credential`, `session`, `workbench`, `grid`.
- B -> A: `vault`.
- D remains: `settings`.
- No capability is promoted solely by changing a label.

### A — 2

- bookmark
- vault

### B — 19

- agent
- apps
- browser
- clipboard
- credential
- database
- git
- graph
- grid
- home
- plugin
- script
- session
- skill
- task
- terminal
- tools
- workbench
- workspace

### C — 0

None.

### D — 1

- settings

## P0 — C -> B

### script — C -> B

Script has an independent physical owner, Manifest/public contract/runtime definition, Script/Snippet stores, capability-owned UI and native ownership. Legacy Workspace implementations were removed. It remains B because it does not claim HP2 hot-plug semantics.

### credential — C -> B

Credential is an independent runtime capability with its own Manifest/public contract/UI and OS-keyring ownership. It is no longer treated as a Browser-owned or external Runtime exemption. Resident security semantics remain conservative.

### session — C -> B

Session owns its state, UI, Manifest/public contract, runtime definition and native persistence. Browser interaction is through the governed `browser-context` Host Service seam rather than Browser internals.

### workbench — C -> B

Workbench owns its store and UI under `src/capabilities/workbench/`. Shell consumes generic contribution slots. The legacy store path is a compatibility re-export only.

### grid — C -> B

Grid is now a real independent capability rather than Browser-owned Grid state:

- independent `src/capabilities/grid/` Manifest, public contract, runtime definition and semantic owner `useGridStore`
- independent Grid state, archive state, UI, host/composable and resource guard
- Grid owns native Grid WebView/process lifecycle and close/rebuild behavior
- Browser no longer owns `gridOpen`, `gridSession`, Grid counts/URLs/layout/mode or Grid lifecycle intents
- dependency direction is `grid -> browser(public)` and `grid -> bridge`; `browser -> grid` is forbidden
- Shell consumes generic Grid contributions and does not import Grid internals
- Native ownership/registry resource declarations are aligned
- deterministic maturity gate explicitly reports Grid among the promoted capabilities

Deterministic evidence:

`V2_P0_MATURITY_BOUNDARY_RESULT=PASS promoted=script,credential,session,workbench,grid`

Grid remains B, not A: it has a truthful C2/B-level contract and does not falsely claim the HP2 hot-plug contract required for A.

## P1 — Agent / Graph decoupling

**PASS**

Agent and Graph no longer depend on each other's internal implementation. Cross-capability data is carried through public/host contracts, and the deterministic anti-cycle gate is blocking.

Evidence:

`AGENT_GRAPH_DECOUPLING_RESULT=PASS`

## P2 — Hot-Plug Acceptance Harness

**PASS**

The unified hard gate exercises:

`ACTIVE -> SUSPENDED -> ACTIVE -> SUSPENDED -> DISABLED -> ACTIVE`

It also verifies repeated cycles, contribution withdrawal/restoration, duplicate prevention, persistence, fresh-Runtime restart, stale contribution absence, activation duration/last error, and tracked resource proxies.

Real HP2 candidates covered:

- bookmark
- vault

Evidence:

`HOT_PLUG_ACCEPTANCE_RESULT=PASS`

## P3 — B -> A

### vault — B -> A

Vault is the only new A promotion in v2.

Evidence includes C3 Manifest, HP2 lifecycle, non-resident behavior, explicit Host ports, contribution register/unregister, repeated lifecycle cycles, persistence/restart and stale/duplicate contribution checks.

Other B capabilities remain B until they meet the same hard HP2 standard.

## P4 — Capability Manager diagnostics

**PASS**

Capability Manager derives diagnostics from Manifest / Runtime / Contribution Registry / persistence rather than a handwritten second capability table.

It exposes:

- id / display name / category / maturity
- runtime state
- semantic owner
- dependencies / dependents / optional dependencies
- suspendable / disableable / blocked reason
- registered contributions
- activation duration
- persistence state
- last error
- package/source ownership

Evidence:

`CAPABILITY_MANAGER_DIAGNOSTICS_RESULT=PASS`

## P5 — Manifest single source, stage 1

**PASS**

Canonical direction:

`manifest.ts -> generated registry -> catalog -> dependency/runtime/manager/gates`

Completed:

- Manifest is the canonical capability metadata source.
- generated registry is generated and drift-checked.
- Catalog metadata is a zero-copy canonical view.
- duplicate generated projections were removed.
- integrated Manifest defaults and lazy contribution wiring are centralized.
- compatibility YAML projections remain blocking drift-checked projections rather than silent second truths.

## Build / dependency discipline

No threshold or baseline was relaxed.

During v2, the existing build-size gate caught real regressions. They were reduced structurally by removing stale imports, keeping runtime entrypoints store-free, centralizing repeated Manifest/contribution metadata, deduplicating Browser/Grid WebView freeze payloads, compacting Grid's injected AI payload and deduplicating repeated Grid selector metadata.

The original build-size threshold remains unchanged and the final pre-merge gate passes.

## GUI / packaged runtime acceptance

The v2 final workflow now provides real packaged-runtime evidence instead of relying only on headless source tests.

### Packaged GUI cold start

The workflow:

- builds the real `.deb`
- extracts the packaged binary
- starts it under Xvfb/DBus
- requires Vue mount
- requires the first-paint probe to report a valid root

Result:

`PASSED: 0 failure(s), 2 warning(s)`

The warnings are Linux portal/service availability warnings in the virtual CI desktop and do not fail application startup.

### Full Tauri GUI regression

The real Tauri release binary is exercised under a virtual X11 desktop with deterministic local web content. The regression covers real Tauri windows, Grid child processes, normal tabs and PTY-backed runtime behavior, including:

- single Grid AI interaction
- concurrent Grid WebViews
- main-window move/resize
- focus restore
- view switching
- normal-tab close while Grid survives
- Grid child crash/recovery
- Grid WebView context menu
- login/session cookie preservation across Grid restart

Result:

`M0_6C_GUI_REGRESSION_RESULT=PASS`

Capability lifecycle correctness remains independently enforced by the Hot-Plug Acceptance Harness, and Capability Manager rendering/diagnostics contracts are enforced by the diagnostics/UI/runtime gates. Together these remove the prior v2 GUI evidence debt without requiring a manual user-operated acceptance pass.

## Final deterministic evidence

Final GitHub validation run:

`37549242517`

Fast UI safety run:

`37549242512`

Key results:

- Git UI safety: PASS
- Scheduler UI safety: PASS
- `AGENT_GRAPH_DECOUPLING_RESULT=PASS`
- `V2_P0_MATURITY_BOUNDARY_RESULT=PASS promoted=script,credential,session,workbench,grid`
- `HOT_PLUG_ACCEPTANCE_RESULT=PASS`
- `CAPABILITY_MANAGER_DIAGNOSTICS_RESULT=PASS`
- `NATIVE_CAPABILITY_BOUNDARY_RESULT=PASS`
- Runtime startup checks: PASS
- Production build: PASS
- Rust fmt: PASS
- Rust check: PASS
- Rust tests: `462 passed; 0 failed; 2 ignored`
- packaged GUI cold-start acceptance: PASS
- `M0_6C_GUI_REGRESSION_RESULT=PASS`
- branch `git diff --check`: PASS
- `SESSION_LOGIC_RESULT=ALL_PASS`
- `SEMANTIC_CLOSURE_LOGIC_RESULT=PASS (28/28)`
- `PRE_MERGE_RESULT=ALL_PASS`

The legacy Doctor output may still print `gui=GUI_PENDING` because that internal checker does not ingest the separate CI Xvfb evidence bundle. It is not the final GUI acceptance gate; the dedicated packaged GUI and full Tauri regression steps above are the authoritative v2 GUI evidence.

## Non-blocking roadmap

These are future maturity opportunities, not v2 acceptance debt:

1. More B -> A promotions only when each capability independently satisfies the same HP2 teardown/restart/resource-cleanup evidence as Bookmark/Vault.
2. A later Manifest single-source stage may eliminate remaining compatibility YAML/document projections.
3. Settings may be revisited separately if a governed capability model is desired; v2 does not artificially promote it.

## Result

**V2_PASS**

Capability Platform v2 has no remaining C capability, all required deterministic/build/native/runtime/package/GUI gates are green, and the branch is ready for final merge/freeze.
