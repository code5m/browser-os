# BrowserOS Gate Matrix

Machine registry: `docs/engineering/governance.json`.

## Local layered entry points

- `npm run gate:quick` runs the gate entry contract test, engineering governance, AI configuration/secret checks, coding-workbench logic checks, and staged/unstaged whitespace checks. `QUICK_CHECKS=PASS` covers only these checks; it is not merge, current-build budget or GUI acceptance. It is a fixed subset, not automatic change-impact analysis.
- `npm run gate:coding` runs AI configuration/secret and coding-workbench logic checks followed by the managed coding UI suite, without a production build. It uses simulated native transport and does not verify real provider or MCP connectivity, package or installed GUI. Run the explicit synthetic provider tests separately when real connectivity needs verification.
- `npm run gate:merge` runs the existing full `npm run check`, default Rust tests, the managed coding UI suite, and the unchanged `scripts/pre-merge.sh` including the independent production-build/hash budget audit. `LOCAL_MERGE_CHECKS=PASS` is local automated evidence only. Ignored Rust integration tests, real provider/MCP calls, remote CI, package, installed GUI, clean-worktree status and approvals are not proved. The script prints their unverified statuses; an installed configured Git pre-merge hook invokes this entry point. The hook is optional and is not automatically installed.
- The managed coding UI suite starts its own Vite server on a temporary localhost port and stops that server after testing; it never installs BrowserOS. It needs Playwright resolvable by Node or `PLAYWRIGHT_MODULE` pointing to an existing installation, plus `/usr/bin/google-chrome`. Missing prerequisites or failed tests fail the gate; they are never skipped as a pass. Its native transport is simulated.
- `npm run verify:client` is a separate build/package/cold-start verification, not part of either local gate; it does not itself establish user-confirmed GUI acceptance. Installation requires explicit authorization, and the desktop must be verified independently according to `AGENTS.md`.
- Developer branch pushes now run the independent `BrowserOS Developer Feedback` Quick Gate. `perf/**`, `feature/**` and `fix/**` pushes also offer a manually requested release .deb preview; default pushes never build it; its source SHA and SHA256 travel with the artifact. A preview is not installed GUI acceptance.
- Full Validation runs for non-draft PRs targeting `master`, `master` pushes and explicit manual dispatch. Draft PRs do not queue Full Validation. `native-regression` is optional, with one Rust-test or one packaged GUI build per selected mode; by default it is SKIPPED, not PASS.
- The local wrapper is not evidence that remote CI ran or that the new coding UI suite is wired into CI. The legacy `PRE_MERGE_RESULT=ALL_PASS` remains scoped to `pre-merge.sh`, not the broader local merge entry point or release acceptance.

| Gate family | Canonical entry | Blocks | Primary evidence |
| --- | --- | --- | --- |
| Developer feedback | `npm run gate:quick` | developer push (fast feedback) | contract, coding and governance check subset |
| Installable preview | `npm run tauri -- build --bundles deb` | manual test readiness (non-blocking) | SHA-bound `.deb` and checksum artifact; GUI_PENDING |
| Engineering governance | `npm run check:engineering-governance` | CI / merge | registry + drift checker |
| Architecture & Capability | `npm run check` | CI / merge | architecture/capability/native/semantic checkers |
| Runtime startup | `npm run check:runtime` | CI / merge | startup/runtime contract |
| Hot-Plug | `check-hot-plug-acceptance.mjs` | CI / merge | lifecycle + negative blocker evidence |
| UI safety | Git/Scheduler/UI policy checkers | CI / merge | self-test + repository scan |
| Production build | `npm run build` | CI / merge | Vite production build |
| Rust quality | optional Rust tests | opt-in / release evidence | one compilation or SKIPPED |
| Packaged GUI | optional packaged GUI mode | opt-in / release evidence | one Release build or SKIPPED |
| Full GUI regression | GUI modes only | opt-in / release evidence | reuse Release, never auto-run |
| Build metrics | `scripts/pre-merge.sh` | merge | strict production SHA256, attribution and dependency checks; absolute size and master increment advisory-only |
| Git integrity | `scripts/pre-merge.sh` | merge | repository object integrity |
| Diff hygiene | `git diff --check` | push / merge | whitespace result |
| Release baseline | immutable tag + GitHub Release | release | tag + release notes + final audit |

## Layers

1. Developer-local: hooks, `npm run check`, doctor, pre-merge.
2. GitHub CI: branch Quick Gate always; preview only on request; default fast static validation on ready PRs/master; native release evidence separately opt-in.
3. Packaged runtime: real .deb and Xvfb/DBus GUI acceptance.
4. Release: named immutable baseline and rollback reference.
5. Recovery: diagnostics, snapshots and recovery procedures.

Default PR and master Full Validation is FAST: native compilation, installation, installed GUI and resources are all SKIPPED unless explicitly enabled. See [Optional Native CI](./CI-OPTIONAL-NATIVE-20261101.md).

A fast green check never substitutes for a slower gate covering another failure mode. Build success is not GUI success; source Hot-Plug proof is not packaged-client acceptance.

2026-10-26: user explicitly approved editor ≤3.5 MiB, core/shared ≤1.35 MiB and total ≤4.75 MiB. Machine limits and pinned Monaco/DOMPurify versions are in `coding-editor-budget.json`. `report-coding-editor-budget.mjs --verify-dist --audit-budget` performs a production-equivalent in-memory build, verifies every disk file/hash and rejects eager editor JS. Since 2026-11-06, the historical three size limits are advisory only. Shared/unattributed bytes remain core. Archived baseline `052b18a` remains as an audit artifact but is not an active CI size failure. Rust warning growth remains blocking. The three domain dependency freezes allow only the explicitly pinned Monaco addition; unrelated dependency changes remain failures.

2026-10-27: the user additionally approved the size budget for the new Monaco features. This implementation uses a finite envelope of editor ≤4 MiB, core/shared ≤1.5 MiB, total ≤5.25 MiB, recorded alongside the previous approval in `coding-editor-budget.json`. This is a recorded review envelope, not an unlimited exemption from verified asset/owner/hash checks; the numeric limits are advisory as of 2026-11-06. The independent total-limit negative fixture now proves a WARN (not a failure) when only the total cap is exceeded; bad accounting, missing hashes or dependency drift still fail. Pinned dependency versions and Rust warning quality checks remain enforced; the archived 45% report is no longer an active gate.
