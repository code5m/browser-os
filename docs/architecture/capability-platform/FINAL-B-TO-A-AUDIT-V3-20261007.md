# Capability Platform v3 — Low-Risk B -> A Audit — 2026-10-07

## Verdict

**V3_LOW_RISK_A_PASS**

This pass evaluated all 19 v2 B-level capabilities and promoted only the lowest-risk candidate that could meet the existing A-grade HP2 evidence bar without changing product semantics.

## Maturity

Before:

`A=2, B=19, C=0, D=1`

After:

`A=3, B=18, C=0, D=1`

A capabilities:

- bookmark
- vault
- home

New promotion:

- `home`: B -> A

## Why Home qualifies

Home was already structurally HP2 before this pass:

- non-resident;
- LIGHT resource class;
- suspendable and destroyable;
- no native handle, child process, WebView, PTY, watcher, timer, socket or background task;
- one generic Shell contribution;
- no runtime resource created during activation.

This pass adds Home to the same unified Hot-Plug Acceptance Harness used by Bookmark and Vault.

The harness uses the real Home definition and registers/activates its real runtime dependencies (Browser, Workspace, Apps) before testing Home.

Verified sequence:

`ACTIVE -> SUSPENDED -> ACTIVE -> SUSPENDED -> DISABLED -> ACTIVE`

Also verified:

- contribution disappears on disable;
- contribution returns exactly once on re-enable;
- disabled persistence survives a read;
- a fresh Runtime restarts Home disabled without stale contribution;
- a fresh Runtime restarts Home enabled with exactly one contribution;
- three repeated disable/enable cycles create no duplicate contribution;
- activation duration is recorded;
- lastError remains null;
- no unsupported dynamic resource kind is owned.

Fast deterministic evidence:

- workflow: `Capability v3 hot-plug acceptance`
- run: `37557509676`
- result: PASS
- capabilities observed: bookmark, vault, home
- `HOT_PLUG_ACCEPTANCE_RESULT=PASS`

## Why the other 18 remain B

The detailed matrix is in:

`docs/architecture/capability-platform/V3-B-TO-A-EVALUATION-20261007.md`

Key blockers are intentionally not bypassed:

- Graph: debounced/in-flight query cancellation still needs explicit lifecycle cleanup evidence.
- Skill/Agent: execution backends remain intentionally incomplete.
- Workspace: current contract says suspendable=false/destroyable=false and owns many Shell views.
- Clipboard: resident capability with focus/Tauri focus listeners and no symmetric teardown.
- Apps: detached child processes are not reversible lifecycle resources.
- Database: network/secret-sensitive operations need explicit absent/no-in-flight proof.
- Git: write subprocesses + credential/workspace dependencies + two-phase write gate.
- Tools: owned WebView windows lack reversible teardown proof.
- Plugin: HEAVY/NATIVE and destroyable=false.
- Script: child-process hot unplug is explicitly unproven.
- Task: scheduler is a process-wide singleton background thread.
- Browser/Grid: native WebView/child-process teardown and suspend contracts are not HP2-proven.
- Terminal: PTY/child process and active-session graceful handling.
- Credential: resident OS-keyring security service.
- Session: resident shutdown/persistence ordering.
- Workbench: resident Shell frame.

## Guardrail

No capability is promoted by changing a label alone. Home becomes A only because the same unified runtime acceptance harness used for existing A capabilities passes with Home included.

## Final condition

The branch is mergeable only after the normal full repository validation remains green:

- architecture/capability checks
- Runtime startup
- production build
- Rust fmt/check/test
- packaged GUI cold-start
- full Tauri GUI regression
- diff check
- full pre-merge
