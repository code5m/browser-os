# Capability Platform v2 — Final Modularity Audit — 2026-10-05

## Final verdict

**V2_PASS_WITH_DEBT**

Capability Platform v2 materially improves the frozen v1 architecture without rewriting v1 history.

### Maturity

Before:

`A=1, B=15, C=5, D=1`

After:

`A=2, B=18, C=1, D=1`

Changes are evidence-based:

- C -> B: `script`, `credential`, `session`, `workbench`.
- B -> A: `vault`.
- Remains C: `grid`.
- Remains D: `settings`.

No capability is promoted solely by changing a label.

## P0 — C -> B

### script — C -> B

- independent `src/capabilities/script/` physical owner
- independent Manifest / public contract / Runtime definition
- `useScriptStore` and `useSnippetStore` owned by Script
- Script UI owned and contribution-registered by Script
- snippet native ownership aligned to Script
- legacy Workspace implementations removed
- child-process lifecycle remains HP0; not promoted to A

Deterministic guard: `scripts/check-v2-maturity-boundaries.mjs`.

### credential — C -> B

- independent `src/capabilities/credential/` runtime capability
- no longer treated as an external Runtime dependency exemption
- OS keyring remains resident/security-sensitive and not runtime-disableable
- Credential UI moved out of Browser and registered by Credential
- Browser presentation is optional and reached through governed seams
- no fake hot-plug claim

Deterministic guard: `scripts/check-v2-maturity-boundaries.mjs`.

### session — C -> B

- independent `src/capabilities/session/` owner/UI/Manifest/public/runtime definition
- legacy Browser Session owner/UI removed
- Browser-owned preview/eval operations are reached through the `browser-context` Host Service
- native persistence remains owned by Session
- resident shutdown ordering remains HP0; not promoted to A

Deterministic guards include Session persistence/logic checks plus `check-v2-maturity-boundaries.mjs`.

### workbench — C -> B

- `useWorkbenchStore` true owner moved to `src/capabilities/workbench/state/`
- old `src/stores/useWorkbenchStore.ts` reduced to compatibility re-export only
- Workbench Rail / Commands moved into capability UI
- Shell consumes generic contribution slots rather than owning Workbench UI
- resident Shell-frame semantics remain non-hot-pluggable

Deterministic guard: `scripts/check-v2-maturity-boundaries.mjs`.

### grid — remains C

Grid is intentionally not falsely promoted.

Current Grid state/process/UI behavior shares Browser's `useBrowserStore`, native WebView/process lifecycle, close/reset behavior, AI multi-grid flow and Shell navigation assumptions. Extracting an independent owner safely is a dedicated Browser/Grid native-lifecycle project, not a routine directory move.

## P1 — Agent / Graph cycle

Result: **PASS**.

- Agent and Graph no longer depend on each other's internal implementation.
- Graph treats agent identifiers as graph payload data.
- Cross-capability interaction is through public/host contracts.
- deterministic anti-cycle gate is wired into the v2 validation suite.
- no registry/runtime dependency cycle is accepted.

## P2 — Hot-Plug Acceptance Harness

`scripts/check-hot-plug-acceptance.mjs` is a hard `npm run check` gate.

Lifecycle exercised:

`ACTIVE -> SUSPENDED -> ACTIVE -> SUSPENDED -> DISABLED -> ACTIVE`

It additionally exercises repeated cycles and fresh-Runtime restart behavior.

Acceptance evidence covers:

- Runtime state
- contribution withdrawal/restoration
- duplicate registration
- refresh persistence
- process-restart persistence
- stale contribution absence
- activation duration / last error
- tracked dynamic resource proxy checks

Real candidates exercised by the unified harness:

- `bookmark`
- `vault`

## P3 — B -> A

### vault — B -> A

Vault is the only new A promotion in v2.

Evidence:

- C3 manifest
- HP2 enable/disable/register/unregister contract
- non-resident capability
- explicit Host ports
- real contribution lifecycle
- unified Hot-Plug Acceptance Harness
- repeated pause/resume/disable/enable cycles
- refresh persistence
- fresh-Runtime restart persistence
- no duplicate/stale contribution
- capability boundary suite remains green

Other B capabilities remain B where teardown/resource/restart evidence is incomplete. v2 does not promote capabilities merely to improve the count.

## P4 — Capability Manager diagnostics

Capability Manager now exposes runtime-derived diagnostics rather than a second handwritten capability table:

- capability id / display name
- category / maturity
- runtime state
- semantic owner
- dependencies / dependents / optional dependencies
- suspendable / disableable
- blocked reason
- registered contributions
- activation duration
- persistence state
- last error
- package/source ownership

Primary data comes from Manifest / Runtime / Registry.

## P5 — Manifest single source, stage 1

Canonical direction:

`manifest.ts -> generated registry -> catalog -> dependency/runtime/manager/gates`

Completed:

- Manifest is the canonical metadata source.
- generated registry remains generated and drift-checked.
- `CANONICAL_CAPABILITY_METADATA` is a zero-copy Catalog view.
- duplicate generated metadata projection was removed.
- integrated capability defaults were centralized in `defineIntegratedCapability`.
- contribution wiring was centralized via `withLazyContributions`.
- existing YAML projections are retained for compatibility; they are not silently deleted.
- drift checks remain blocking.

This is stage 1, not a Registry rewrite.

## Build / dependency discipline

No threshold or baseline was relaxed.

During v2 the frontend size gate exposed a real regression. The response was structural deduplication:

- remove stale Workbench Shell imports
- keep Script / Session Runtime entrypoints store-free
- share lazy contribution wiring
- centralize repeated integrated Manifest defaults
- remove duplicate canonical metadata projection
- deduplicate additional compatible capability Manifest metadata

The build-size limit remains unchanged.

## Deterministic gate set

The v2 GitHub workflow executes:

- `npm run check`
- `npm run check:runtime`
- `npm run build`
- Rust fmt
- Rust check
- Rust tests
- branch `git diff --check`
- `bash scripts/pre-merge.sh`

The architecture check bundle includes the v2 maturity boundary guard, Agent/Graph anti-cycle guard, Hot-Plug Acceptance Harness, Registry/drift checks, UI/native boundaries and Capability Manager/runtime checks.

## GUI acceptance debt

The repository contains the frozen v1 Capability Manager release GUI evidence under:

`logs/acceptance/capability-manager-20261004/`

The v2 GitHub environment validates runtime and headless lifecycle behavior but does not provide a trusted desktop GUI session for a fresh v2 screenshot/restart acceptance pass.

Therefore a **v2 real GUI re-verification remains debt**:

- open Capability Manager
- validate diagnostics rendering
- ACTIVE -> SUSPENDED -> ACTIVE
- SUSPENDED -> DISABLED -> ACTIVE
- repeated cycles
- refresh
- process restart
- contribution disappearance/restoration/no duplication
- no "当前视图不可用"
- no panic/error

This debt does not invalidate the deterministic source/build/runtime gates, but prevents a `V2_PASS` claim.

## Final debt

1. **Grid remains C**: Browser/Grid native lifecycle and shared owner require a dedicated extraction project.
2. **V2 real GUI re-verification**: not reproducible in the GitHub headless validation runner.
3. Native-heavy B capabilities remain B until owner-level teardown/restart evidence satisfies the same A standard as Bookmark/Vault.
4. Existing YAML/document projections remain compatibility projections until later single-source stages migrate their remaining fields.

## Final classification

### A — 2

- bookmark
- vault

### B — 18

- agent
- apps
- browser
- clipboard
- credential
- database
- git
- graph
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

### C — 1

- grid

### D — 1

- settings

## Result

**V2_PASS_WITH_DEBT**

The v2 branch is acceptable when the final unchanged-threshold workflow is green. The remaining debt is explicit and must not be represented as completed hot-plug support.
