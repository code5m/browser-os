# GitHub Actions Workflow Catalog

Workflow registration is machine-enforced by `docs/engineering/governance.json`.

| File | Role | Trigger policy |
| --- | --- | --- |
| `capability-v2-ui-safety.yml` | fast UI policy feedback | all branch pushes / PR to master; risk-aware jobs; manual full |
| `capability-v2-validation.yml` | mandatory merge and release gate | master pushes / non-draft PRs targeting master (including ready-for-review); manual full |
| `capability-v3-hotplug.yml` | focused Hot-Plug acceptance | all branch pushes / PR to master; risk-aware jobs; manual full |
| `browseros-developer-feedback.yml` | fast feedback and non-blocking installable preview | Quick Gate on every push; automatic .deb on perf/feature/fix pushes; optional manual preview |
| `engineering-governance.yml` | governance drift | all branch pushes / PR to master; risk-aware jobs; manual full |
| `supply-chain-assurance.yml` | supply-chain gate and SBOM evidence | all branch pushes / PR to master; risk-aware jobs; manual full |
| `mirror-to-gitee.yml` | retired legacy mirror placeholder | manual only; no branch/tag push trigger |

The v2/v3 filenames remain for historical compatibility. Every developer branch push gets the Quick Gate, and existing focused safety/hot-plug/governance workflows remain active. Full Validation no longer starts on developer pushes or draft PRs; it runs on master pushes, non-draft PRs targeting master (including the ready-for-review transition), and explicit manual dispatch. The fail-closed `scripts/ci-impact.py` still selects `docs`, `ui`, or `full` for runs that actually invoke Full Validation. Unknown, native, bridge, dependency, workflow and mixed-sensitive changes remain full. A skipped check is never evidence of passing that check.

The independent Preview job builds a real release `.deb` from the exact Git SHA and uploads it with checksum and source-commit metadata. It runs automatically on `perf/**`, `feature/**` and `fix/**` branch pushes, or by manual dispatch (once the workflow is available on the default branch). It has no installed desktop interaction and is NOT `GUI_PASS`. Developers can download/install the preview for manual verification before Full Validation finishes; an earlier preview must not be confused with a later commit.

## Evidence

Full validation uploads GUI summary, driver report and application log as a short-retention Actions artifact so CI evidence does not vanish with the runner and does not permanently inflate the Git repository. Independent `native-regression` owns the Rust format/check/test gate, avoiding the duplicate test sequence inside `validate`. Preview uploads `.deb`, SHA256 and source SHA without pretending that a package build is GUI acceptance. Supply-chain assurance uploads deterministic SBOM/checksum evidence.

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
- Before producing a new Release, manually trigger Full Validation + Supply Chain Assurance at the exact release SHA. Stable tags cannot be moved to shortcut verification.
- Draft PR: fast feedback and preview only. Mark ready for review: Full Validation runs; all required checks must succeed before merge.
- Download the preview from the Developer Feedback workflow's `browseros-preview-<sha>` artifact. Check `SOURCE_COMMIT` and `SHA256SUMS` and install it on the authorized test desktop. Preview is `PACKAGE_PASS` at most; only user-confirmed real desktop acceptance may be `GUI_PASS`.
- GitHub required-check settings must be reconciled with conditionally skipped jobs; branch protection should require the always-running impact/governance jobs instead of skipped child jobs, without weakening full/release evidence.
