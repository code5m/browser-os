# BrowserOS Gate Matrix

Machine registry: `docs/engineering/governance.json`.

## Local layered entry points

- `npm run gate:quick` runs the gate entry contract test, engineering governance, AI configuration/secret checks, coding-workbench logic checks, and staged/unstaged whitespace checks. `QUICK_CHECKS=PASS` covers only these checks; it is not merge, current-build budget or GUI acceptance. It is a fixed subset, not automatic change-impact analysis.
- `npm run gate:coding` runs AI configuration/secret and coding-workbench logic checks followed by the managed coding UI suite, without a production build. It uses simulated native transport and does not verify real provider or MCP connectivity, package or installed GUI. Run the explicit synthetic provider tests separately when real connectivity needs verification.
- `npm run gate:merge` runs the existing full `npm run check`, default Rust tests, the managed coding UI suite, and the unchanged `scripts/pre-merge.sh` including the independent production-build/hash budget audit. `LOCAL_MERGE_CHECKS=PASS` is local automated evidence only. Ignored Rust integration tests, real provider/MCP calls, remote CI, package, installed GUI, clean-worktree status and approvals are not proved. The script prints their unverified statuses; an installed configured Git pre-merge hook invokes this entry point. The hook is optional and is not automatically installed.
- The managed coding UI suite starts its own Vite server on a temporary localhost port and stops that server after testing; it never installs BrowserOS. It needs Playwright resolvable by Node or `PLAYWRIGHT_MODULE` pointing to an existing installation, plus `/usr/bin/google-chrome`. Missing prerequisites or failed tests fail the gate; they are never skipped as a pass. Its native transport is simulated.
- `npm run verify:client` is a separate build/package/cold-start verification, not part of either local gate; it does not itself establish user-confirmed GUI acceptance. Installation requires explicit authorization, and the desktop must be verified independently according to `AGENTS.md`.
- Existing CI workflows continue to execute their recorded gates; the local wrapper is not evidence that remote CI ran or that the new coding UI suite is wired into CI. The legacy `PRE_MERGE_RESULT=ALL_PASS` remains scoped to `pre-merge.sh`, not the broader local merge entry point or release acceptance.

| Gate family | Canonical entry | Blocks | Primary evidence |
| --- | --- | --- | --- |
| Engineering governance | `npm run check:engineering-governance` | CI / merge | registry + drift checker |
| Architecture & Capability | `npm run check` | CI / merge | architecture/capability/native/semantic checkers |
| Runtime startup | `npm run check:runtime` | CI / merge | startup/runtime contract |
| Hot-Plug | `check-hot-plug-acceptance.mjs` | CI / merge | lifecycle + negative blocker evidence |
| UI safety | Git/Scheduler/UI policy checkers | CI / merge | self-test + repository scan |
| Production build | `npm run build` | CI / merge | Vite production build |
| Rust quality | fmt/check/test | CI / merge | compiler/tests |
| Packaged GUI | installed-client verifier | CI / release | package/cold-start evidence |
| Full GUI regression | `m0-6c-gui-regression.py` | CI / merge | summary + driver report + app log artifact |
| Build metrics | `scripts/pre-merge.sh` | merge | approved editor/core/total budgets + exact production hashes; unchanged historical 45% comparison reported separately |
| Git integrity | `scripts/pre-merge.sh` | merge | repository object integrity |
| Diff hygiene | `git diff --check` | push / merge | whitespace result |
| Release baseline | immutable tag + GitHub Release | release | tag + release notes + final audit |

## Layers

1. Developer-local: hooks, `npm run check`, doctor, pre-merge.
2. GitHub CI: feature/master push + PR validation.
3. Packaged runtime: real .deb and Xvfb/DBus GUI acceptance.
4. Release: named immutable baseline and rollback reference.
5. Recovery: diagnostics, snapshots and recovery procedures.

A fast green check never substitutes for a slower gate covering another failure mode. Build success is not GUI success; source Hot-Plug proof is not packaged-client acceptance.

2026-10-26: user explicitly approved editor ≤3.5 MiB, core/shared ≤1.35 MiB and total ≤4.75 MiB. Machine limits and pinned Monaco/DOMPurify versions are in `coding-editor-budget.json`. `report-coding-editor-budget.mjs --verify-dist --enforce-budget` performs a production-equivalent in-memory build, verifies every disk file/hash, rejects eager editor JS and enforces all three limits. Shared/unattributed bytes remain core. Historical baseline `052b18a` and its 45% comparison remain unmodified and observable; they are not falsely reported as satisfied. Rust warning growth remains blocking. The three domain dependency freezes allow only the explicitly pinned Monaco addition; unrelated dependency changes remain failures.

2026-10-27: the user additionally approved the size budget for the new Monaco features. This implementation uses a finite envelope of editor ≤4 MiB, core/shared ≤1.5 MiB, total ≤5.25 MiB, recorded alongside the previous approval in `coding-editor-budget.json`. This is not an unlimited exemption: every emitted asset/hash and all three limits remain enforced. The independent total-limit negative fixture derives its input from the policy and still proves a failure when only the total cap is exceeded. Historical baseline, 45% report, pinned dependency versions and Rust warning limits are unchanged.
