# Capability Platform v3 — B -> A Risk Evaluation — 2026-10-07

## Goal

Evaluate every v2 B-level capability against the same evidence standard already used by Bookmark/Vault. Promote only the lowest-risk capabilities that can prove real HP2 lifecycle behavior without changing product semantics.

A promotion requires all of the following:

- C3-or-better composition evidence;
- non-resident runtime lifecycle;
- safe pause/resume and disable/enable;
- deterministic contribution unregister/re-register;
- no duplicate contribution after repeated cycles;
- fresh-Runtime disabled/enabled persistence;
- no untracked dynamic resource class;
- no hidden listener/timer/process/webview/PTY/background-task residue;
- full architecture/runtime/build/Rust/GUI/pre-merge gates remain green.

## Evaluation matrix

| Capability | Risk | Key evidence / blocker | Decision |
| --- | --- | --- | --- |
| home | LOW | Already HP2; LIGHT; non-resident; suspendable+destroyable; no native/background resource; one generic contribution | **PROMOTED — unified Hot-Plug Harness PASS** |
| graph | MEDIUM-LOW | Non-resident and suspendable, but store owns debounce/AbortController/in-flight request state; needs explicit cancel-on-suspend/deactivate evidence | KEEP B |
| skill | MEDIUM-LOW | LIGHT and non-resident, but maturity C1 and execution/install backend intentionally incomplete | KEEP B |
| workspace | MEDIUM | C3 and no heavy native resource, but current resource policy says suspendable=false/destroyable=false and owns many Shell views | KEEP B |
| clipboard | MEDIUM-HIGH | LIGHT package, but resident=true; owns focus listener and Tauri focus subscription without symmetric teardown | KEEP B |
| apps | MEDIUM-HIGH | Detached child processes are outside reversible lifecycle ownership | KEEP B |
| database | MEDIUM-HIGH | Network/secret-sensitive dependency on credential; transient DB operations still need absent-state/no-in-flight proof | KEEP B |
| git | MEDIUM-HIGH | Write subprocesses, credential/workspace dependencies and two-phase write gate require graceful in-flight teardown proof | KEEP B |
| agent | MEDIUM-HIGH | NETWORK resource and execution backend remains intentionally incomplete (C1) | KEEP B |
| tools | HIGH | Owns tool WebView windows; reversible WebView teardown contract not proven | KEEP B |
| plugin | HIGH | HEAVY/NATIVE and destroyable=false; no unloadable plugin runtime exists | KEEP B |
| script | HIGH | Owns CHILD_PROCESS; child-process hot unplug explicitly unproven | KEEP B |
| task | HIGH | Backend scheduler is a process-wide singleton/background thread independent of frontend capability lifecycle | KEEP B |
| browser | HIGH | Owns native WebViews/child processes; suspendable=false/destroyable=false; graceful no-residue shutdown not proven | KEEP B |
| grid | HIGH | Owns multiple native WebViews/child processes; suspend/resume HP2 contract intentionally not claimed | KEEP B |
| terminal | HIGH | Owns PTY + child process; active sessions require graceful user-work handling; suspendable=false | KEEP B |
| credential | HIGH | Resident OS-keyring security capability; intentionally non-disableable | KEEP B |
| session | HIGH | Resident shutdown/persistence ordering capability; intentionally non-hot-pluggable | KEEP B |
| workbench | HIGH | Resident Shell frame; intentionally runtime-nondisableable | KEEP B |

## First promotion

### Home

Home is the only candidate that already has the structural preconditions for A without adding new business behavior:

- Manifest already declares HP2.
- Runtime definition is non-resident.
- Resource class is LIGHT.
- No native handle, process, WebView, PTY, watcher, timer, socket or background task is owned.
- Shell integration is a single generic `workbench-main` contribution.
- Re-enable is idempotent because contribution registry is keyed by contribution id.
- Existing persistence is user data persistence, not runtime-resource ownership.

This v3 branch therefore adds Home to the same unified `scripts/check-hot-plug-acceptance.mjs` used by Bookmark/Vault. The harness tests Home with its real Browser/Workspace/Apps runtime dependencies registered and active.

Acceptance sequence:

`ACTIVE -> SUSPENDED -> ACTIVE -> SUSPENDED -> DISABLED -> ACTIVE`

Plus:

- disabled persistence readback;
- fresh Runtime restart while disabled;
- fresh Runtime restart while enabled;
- three repeated disable/enable cycles;
- exact contribution count after every restore;
- duplicate contribution rejection by evidence;
- `lastError === null`;
- activation duration recorded;
- no unsupported dynamic resource class.

Result: **PASS**. Home completed the unified harness alongside Bookmark and Vault, including repeated lifecycle cycles and fresh-Runtime persistence.

## Deferred next candidates

The next realistic candidates are Graph and Workspace, but neither is low-risk enough for automatic promotion in this pass:

- Graph needs explicit lifecycle cleanup for debounced/in-flight queries.
- Workspace needs a truthful suspend/destroy contract and proof that removing all of its Shell views never leaves a stale current view.

They should be separate, evidence-driven follow-up promotions rather than bundled with Home.

## Final maturity

Before:

`A=2, B=19, C=0, D=1`

After:

`A=3, B=18, C=0, D=1`

No other capability is promoted in this pass. The remaining 18 B capabilities retain truthful blockers and are not relabeled.
