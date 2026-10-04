# Capability modularity completion audit — 2026-10-04

## Conclusion

The project now has a real Capability architecture, but it is **not accurate to call the whole product completely modular or fully hot-pluggable**.

- **A — truly modular and runtime-pluggable:** 1 (`bookmark`).
- **B — architecture-module complete, not yet safely runtime-pluggable:** 15.
- **C — still physically embedded or not integrated as an independent runtime capability:** 5.
- **D — framework/core resident service, intentionally not a normal removable capability:** 1 (`settings`).

The strongest verified claim is:

> The Capability platform, registry, dependency assembly, contribution mechanism, persistence, and lifecycle manager are working. All integrated capabilities have a public boundary and no cross-capability internal state/UI import. Bookmark is the accepted end-to-end pluggable pilot. The remaining capabilities are not all independently suspendable, disableable, resource-releasable, and restart-restorable.

## Audit basis

This audit uses current source and deterministic checks, not directory names alone:

- product registry: `docs/architecture/capability-registry/capabilities.yaml` (22 product entries);
- generated runtime registry: `src/capability/platform/generated-registry.ts`;
- capability manifests and public entry points;
- `check-capability-registry`: PASS, fail=0, warn=0;
- `check-ui-boundaries`: PASS, fail=0, warn=0, cross-internal import=0;
- `check-capability-boundaries`: PASS, fail=0, with one explicit optional-cycle warning (`agent ↔ graph`);
- Capability Manager release GUI evidence in `logs/acceptance/capability-manager-20261004/`;
- native ownership records and the existing `bridge.rs` facade decision.

`demo` is a discovery/test fixture package and is not counted as a product capability. `bridge`, `pinia`, `security_policy`, and `shell` are shared infrastructure rather than user-removable product capabilities.

### Legend

- `R/M`: architecture registry / independent manifest.
- `Internal`: imports another capability's internal `state` or `ui`; all integrated capabilities are `0` after the final Workspace→Browser seam cleanup.
- `Store/UI`: remaining public-store or Shell coupling. Public imports are legal, but a large number still limits independent replacement.
- `Native`: direct bridge/native ownership or a Host port adapter.
- `M1`: isolated source directory; `M2`: independent workspace package; `M0`: logical/embedded only.

## Boundary and ownership matrix

| Capability | Independent owner | R/M | Declared dependencies | Internal | Store/UI coupling | Native / Bridge coupling | Physical |
|---|---|---|---|---:|---|---|---|
| browser | `useBrowserStore` | yes/yes | bridge | 0 | high: Shell consumes Browser public stores; Grid, Session and Resource state remain close to Browser | WebView/process lifecycle and central bridge facade | M1 |
| grid | shared `useBrowserStore` | yes/no | browser, bridge | embedded | physically under Browser components and shares Browser owner | multi-WebView/native grid process | M0 |
| workspace | `useWorkspaceStore` plus governed subdomain owners | yes/yes | bridge; browser optional | 0 | high: Shell consumes Workspace stores; Browser access now goes through `browserNav` narrow seam | many filesystem/script/repo bridge calls | M1 |
| terminal | `useTerminalStore` | yes/yes | bridge | 0 | Shell consumes Terminal public store | PTY and child-process lifecycle | M1 |
| bookmark | `useBookmarkStore` | yes/yes | bridge; browser optional | 0 | two legal Browser public consumers | bridge persistence; no heavy native resource | M1 |
| credential | native `KeyringStore` | yes/no | bridge | embedded | exposed through Browser credential UI | OS keyring/security policy | M0 |
| database | `useDatabaseStore` | yes/yes | credential, bridge | 0 | contribution-driven panel | network/database handle through bridge | M1 |
| git | `useGitStore` | yes/yes | credential, workspace, bridge | 0 | legal Workspace public imports and repo contribution slot | Git processes and credentials through bridge | M1 |
| agent | `useAgentStore` | yes/yes | bridge; graph optional | 0 | contribution-driven panel | bridge; execution backend is incomplete | M1 |
| skill | `useSkillStore` | yes/yes | bridge | 0 | contribution-driven panel | bridge; read/validate path | M1 |
| plugin | `usePluginStore` | yes/yes | bridge | 0 | contribution-driven panel | native manifest inspection; runtime remains locked | M1 |
| graph | `useGraphStore` | yes/yes | bridge; agent optional | 0 | contribution-driven panel | startup in-memory graph snapshot through bridge | M1 |
| vault | `useVaultStore` | yes/yes | bridge; browser optional | 0 | Host adapter supplies Workbench/layout ports | package-owned state with Host native port | M2 |
| task | `useTaskStore` | yes/yes | workspace, bridge; script optional | 0 | legal Workspace public import | Rust scheduler/background execution | M1 |
| clipboard | `useClipboardStore` | yes/yes | bridge | 0 | Host adapter also consumes layout state | system clipboard Host port; resident | M2 |
| apps | `useAppsStore` | yes/yes | bridge | 0 | Shell consumes Apps public store | launches detached OS processes | M1 |
| tools | `useToolsStore` | yes/yes | bridge | 0 | contribution-driven panel | independent tool WebViews | M1 |
| session | `useSessionStore` | yes/no | bridge | embedded | state currently lives inside Browser package/public surface | shutdown/persistence native path | M0 |
| script | `useScriptStore` | yes/no | bridge | embedded | UI and state live inside Workspace | script child processes | M0 |
| workbench | `useWorkbenchStore` | yes/no | bridge | embedded | Shell/layout owner rather than isolated product package | Host/layout and bridge facade | M0 |
| settings | `useSettingsStore` | yes/yes | none | 0 | framework preference surface | none significant; deliberately resident | M1 |
| home | `useHomeStore` | yes/yes | browser, workspace, apps, bridge | 0 | legal Browser/Workspace/Apps public contracts | launches apps/opens filesystem through declared dependencies | M1 |

## Runtime lifecycle matrix

The Manager intentionally requires `C3 + HP1 or higher + governed + non-resident`. A manifest saying that a hook exists is not counted as accepted runtime disablement.

| Capability | Maturity / hot plug | Independent enable | Pause | Safe disable | Re-enable | Contribution withdrawn | Restart restores state | Class | Remaining blocker |
|---|---|---:|---:|---:|---:|---|---|:---:|---|
| browser | C3 / HP0 | no | no | no | no | static composition only | n/a | B | native WebViews are not suspendable/destroyable; deep Shell/Grid coupling |
| grid | not integrated | no | no | no | no | no independent contribution lifecycle | n/a | C | shared Browser owner and native process lifecycle |
| workspace | C3 / HP0 | no | no | no | no | static composition only | n/a | B | resources marked non-suspendable; broad Shell and bridge surface |
| terminal | C3 / HP0 | no | no | no | no | static composition only | n/a | B | PTY suspend is unsupported; release contract not accepted |
| bookmark | C3 / HP2 | **yes** | **yes** | **yes** | **yes** | **yes, GUI verified** | **yes, GUI verified** | **A** | HP3 install/uninstall is intentionally out of scope |
| credential | not integrated | no | no | no | no | n/a | n/a | C | security-sensitive native owner; should not be casually unloadable |
| database | C2 / HP0 | no | no | no | no | static only | n/a | B | owned connection cancellation/release and absence proof incomplete |
| git | C2 / HP0 | no | no | no | no | static only | n/a | B | Workspace/credential dependency and process cleanup contract incomplete |
| agent | C1 / HP0 | no | no | no | no | static only | n/a | B | execution backend incomplete; optional graph cycle remains explicit |
| skill | C1 / HP0 | no | no | no | no | static only | n/a | B | absence and lifecycle proof missing |
| plugin | C2 / HP0 | no | no | no | no | static only | n/a | B | plugin execution is locked; no safe runtime unload contract |
| graph | C2 / HP0 | no | no | no | no | static only | n/a | B | startup snapshot lifecycle and optional agent cycle need direction |
| vault | C2 / HP2 | Manager blocks | Manager blocks | Manager blocks | Manager blocks | generic mechanism exists; product E2E not accepted | not accepted | B | must reach C3 absence proof and verify Host-port cleanup |
| task | C2 / HP0 | no | no | no | no | static only | n/a | B | Rust scheduler/background task ownership and cancellation proof |
| clipboard | C2 / HP0, resident | no | no | no | no | static resident | n/a | B | Host/layout coupling and resident policy; no runtime switching contract |
| apps | C2 / HP0 | no | no | no | no | static only | n/a | B | detached processes are not owned or reclaimable by capability lifecycle |
| tools | C2 / HP0 | no | no | no | no | static only | n/a | B | tool WebView destroy/release proof missing |
| session | not integrated, resident | no | no | no | no | n/a | application-level only | C | embedded Browser state and shutdown ordering |
| script | not integrated | no | no | no | no | n/a | n/a | C | physically hosted by Workspace; process lifecycle not independently governed |
| workbench | not integrated | no | no | no | no | n/a | application-level only | C | Shell owner, store and UI are not an independent capability package |
| settings | C2 / HP2 declaration, resident | no by design | no by design | no by design | no by design | always present | framework persistence | D | framework preference service; should remain resident |
| home | C2 / HP2 | Manager blocks | Manager blocks | Manager blocks | Manager blocks | generic mechanism exists; product E2E not accepted | not accepted | B | strong dependencies plus missing C3 absence/restore proof |

## Classification

### A. Truly modular / pluggable

- `bookmark`: independent owner, manifest, public contract, dependencies, contribution lifecycle, persistence, Manager actions, entry removal/restoration, refresh persistence, and process-restart persistence are all verified.

### B. Architecture modular, not safely pluggable

- `agent`, `apps`, `browser`, `clipboard`, `database`, `git`, `graph`, `home`, `plugin`, `skill`, `task`, `terminal`, `tools`, `vault`, `workspace`.

These are not merely arbitrary folders: each has an owner and governed boundary, and all cross-capability imports use public contracts. They still lack some combination of resource release, C3 absence proof, accepted lifecycle hooks, restart recovery, or safe dependent handling.

### C. Not yet an independent runtime module

- `grid`, `credential`, `session`, `script`, `workbench`.

These are registered domain concepts but remain physically embedded, share another owner, or have no independent manifest/runtime definition.

### D. Framework/core resident

- `settings`.
- Shared infrastructure outside the product capability count: `bridge`, `pinia`, `security_policy`, `shell`.

## Remaining coupling facts

1. Cross-capability internal state/UI imports are now **zero**.
2. The final Workspace editor direct Browser-store import was replaced with the `browserNav` narrow intent seam; UI boundary warnings fell from one to zero.
3. Registry/dependency metadata warnings fell from five to zero without weakening a check or changing a threshold.
4. One optional dependency cycle remains: `agent ↔ graph`. It is non-blocking because both directions are optional, but the long-term contract should choose one direction or introduce a shared query port.
5. Shell still holds 26 public business-store references. They are legal and baseline-guarded, but they are the largest remaining UI composition coupling.
6. The central Rust `bridge.rs` remains a compatibility facade. Native command ownership is governed, but physical per-capability native extraction is not complete.

## Prioritized follow-up plan

### P0 — finish installed-client truth

1. Install the generated deb with interactive sudo authentication.
2. Re-run `verify:client` and cold-start `/usr/bin/mvp-browser-os` from the desktop entry.
3. Repeat the short bookmark lifecycle smoke test against the installed binary, then record `GUI_PASS` only if hashes, geometry and pixels all match.

### P1 — expand the proven pluggable set without touching native-heavy cores

1. Promote `vault` to C3: absence boot, contribution removal, Host-port cleanup, persistence policy, restart restore, and multi-cycle GUI acceptance.
2. Promote `home` only after its Browser/Workspace/Apps dependency behavior is explicit when one dependency is absent.
3. Decide whether Clipboard is intentionally resident forever. If not, move view-state callbacks out of the layout Host adapter and add HP lifecycle tests.

### P2 — resource-owning capabilities

1. Define and verify cancellation/release contracts for Database, Git, Task, Tools and Terminal.
2. Only then enable Manager operations; do not infer safe disablement from `destroyable: true` metadata alone.
3. Treat Browser/Grid native WebView teardown as a dedicated high-risk native project, not a routine UI cleanup.

### P3 — physical extraction and Shell reduction

1. Extract `script` and `session` from Workspace/Browser-hosted locations into independent packages or explicitly reclassify them as sub-capabilities.
2. Decide whether `grid` is a Browser sub-capability or an independent capability; do not keep both claims.
3. Reclassify `workbench` as framework/core or give it a real package and lifecycle.
4. Replace the highest-value subset of the 26 Shell store reads with intent ports and contribution-derived view models.
5. Resolve the `agent ↔ graph` optional cycle with a single direction or shared read-only port.

## Final naming decision

Recommended external description:

> “Capability architecture and composition platform completed; one capability is fully runtime-pluggable, fifteen are boundary-modular but lifecycle-limited, five remain embedded, and one is intentionally resident.”

Avoid claiming “all capabilities are completely modular” or “the entire product is hot-pluggable” until the B and C groups have lifecycle/resource evidence comparable to Bookmark.
