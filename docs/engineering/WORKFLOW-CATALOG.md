# GitHub Actions Workflow Catalog

Workflow registration is machine-enforced by `docs/engineering/governance.json`.

| File | Role | Trigger policy |
| --- | --- | --- |
| `capability-v2-ui-safety.yml` | fast UI policy feedback | all branch pushes / PR to master; risk-aware jobs; manual full |
| `capability-v2-validation.yml` | fast merge gate with explicit optional native and installed GUI modes | master pushes / non-draft PRs targeting master (including ready-for-review); manual full |
| `capability-v3-hotplug.yml` | focused Hot-Plug acceptance | all branch pushes / PR to master; risk-aware jobs; manual full |
| `browseros-developer-feedback.yml` | fast feedback and manual-only optional installable preview | Quick Gate on every push; manual .deb only (default off) |
| `engineering-governance.yml` | governance drift | all branch pushes / PR to master; risk-aware jobs; manual full |
| `supply-chain-assurance.yml` | supply-chain gate and SBOM evidence | all branch pushes / PR to master; risk-aware jobs; manual full |
| `browseros-sustained-performance.yml` | opted-in installed RSS/PSS/CPU evidence | manual dispatch only; no weekly schedule |
| `mirror-to-gitee.yml` | retired legacy mirror placeholder | manual only; no branch/tag push trigger |

The v2/v3 filenames remain for historical compatibility. Every developer branch push gets the Quick Gate, and existing focused safety/hot-plug/governance workflows remain active. Full Validation no longer starts on developer pushes or draft PRs; it runs on master pushes, non-draft PRs targeting master (including the ready-for-review transition), and explicit manual dispatch. The fail-closed `scripts/ci-impact.py` still selects `docs`, `ui`, or `full` for runs that actually invoke Full Validation. Unknown, native, bridge, dependency, workflow and mixed-sensitive changes remain full. A skipped check is never evidence of passing that check.

The independent Preview job builds a release Debian package only after an explicit workflow_dispatch request with build_preview=true (default false). Branch pushes never compile a native preview. The formerly weekly installed-resource benchmark is also manual-only.

Full Validation provides off (default), rust-tests, packaged-gui, and packaged-resource modes, selected by manual input, one PR label, or the CI_NATIVE_MODE repository variable. The current source frontend is built ONCE into a SHA-bound artifact and reused in static and optional native jobs. The same-run master size audit builds a different source commit, which is an approved regression baseline, not a duplicate of the current source.

Skipping Rust or GUI is reported as SKIPPED, not PASS. For release evidence, explicitly run rust-tests and packaged-resource at the same SHA in two separately requested workflows. Details: [Optional Native CI and Build Reuse](./CI-OPTIONAL-NATIVE-20261101.md).

## Evidence

Only opted-in GUI-mode validation uploads GUI summary, driver report and app log as a short-retention Actions artifact so CI evidence does not vanish with the runner and does not permanently inflate the Git repository. Optional native-regression owns exactly one Rust-test compile OR one packaged Release/GUI path; the validate job never recompiles Rust. Preview uploads `.deb`, SHA256 and source SHA without pretending that a package build is GUI acceptance. Supply-chain assurance uploads deterministic SBOM/checksum evidence.

## Blocking policy

UI Safety, Full Validation, Hot-Plug, Engineering Governance and Supply Chain remain engineering quality gates for their applicable triggers. The new Developer Feedback/Preview workflow is non-blocking and cannot replace a full merge/release gate. Configure GitHub branch protection to require the appropriate Full Validation job results for merge to `master`; this Git repository change alone cannot configure branch protection. The legacy Gitee mirror is retired and must not gate GitHub `master`.

Any workflow added/removed must update `governance.json` in the same change.

## Local Git credential guards (2026-10-25)

The registered `.githooks/pre-commit` checks staged content for known local AI credentials and recognizable provider tokens. `.githooks/pre-push` checks all outgoing commit trees for each requested ref before the existing GitHub fast-forward checks. The guards report file names and error codes only. `npm run check:ai-config` and `pre-merge.sh` cover import regression fixtures and current-file secret checks. Credential values are read in memory from explicit local sources and OS Secret Service; they are never stored in reports or Git assets.

## Change-aware execution and evidence

- `docs`: only narrative documentation; skip compiler, GUI, Hot-Plug, and advisory scans. Engineering Governance remains active for all pushes and PRs.
- `ui`: up to six safe leaf-view/UI source files plus documentation; run Node setup, app checks and production build, and UI policy check; skip Rust compiler/packaged GUI where no native or bridge contracts changed.
- `full`: changes to Rust, Tauri configuration, root app, scripts, dependency manifests, workflow rules, native/bridge, or any unknown file. Run every original validation gate. Manual dispatch forces this mode.
- Dependency scanning runs when dependency manifests, security/SBOM scripts, workflows or manual dispatch require it; unrelated source changes do not reinstall `cargo-audit`.
- Classifier exceptions, nonexistent diff boundary, and unexpected file names must fail closed to `full`; no filename patterns may weaken a required release gate.
- Before producing a Release, manually run Full Validation rust-tests and packaged-resource separately at the exact SHA, plus Supply Chain Assurance. Stable tags cannot be moved to shortcut verification.
- Draft PR: fast feedback only; preview is manual opt-in. Mark ready for review: Full Validation runs; all required checks must succeed before merge.
- Download the preview from the Developer Feedback workflow's `browseros-preview-<sha>` artifact. Check `SOURCE_COMMIT` and `SHA256SUMS` and install it on the authorized test desktop. Preview is `PACKAGE_PASS` at most; only user-confirmed real desktop acceptance may be `GUI_PASS`.
- GitHub required-check settings must be reconciled with conditionally skipped jobs; branch protection should require the always-running impact/governance jobs instead of skipped child jobs, without weakening full/release evidence.
