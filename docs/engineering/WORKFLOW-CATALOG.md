# GitHub Actions Workflow Catalog

Workflow registration is machine-enforced by `docs/engineering/governance.json`.

| File | Role | Trigger policy |
| --- | --- | --- |
| `capability-v2-ui-safety.yml` | fast UI policy feedback | all branch pushes / PR to master; risk-aware jobs; manual full |
| `capability-v2-validation.yml` | full BrowserOS validation | all branch pushes / PR to master; risk-aware jobs; manual full |
| `capability-v3-hotplug.yml` | focused Hot-Plug acceptance | all branch pushes / PR to master; risk-aware jobs; manual full |
| `engineering-governance.yml` | governance drift | all branch pushes / PR to master; risk-aware jobs; manual full |
| `supply-chain-assurance.yml` | supply-chain gate and SBOM evidence | all branch pushes / PR to master; risk-aware jobs; manual full |
| `mirror-to-gitee.yml` | retired legacy mirror placeholder | manual only; no branch/tag push trigger |

The v2/v3 filenames are retained to avoid unnecessary historical path churn. Their display names and triggers are version-neutral; they protect current BrowserOS. All branch pushes are covered by the active quality gates so branch naming (`feature/**`, `fix/**`, `chore/**`, or another convention) cannot bypass impact classification. The fail-closed `scripts/ci-impact.py` selects `docs`, `ui`, or `full` using the complete changed-file list: unknown, empty, native, bridge, dependency, workflow, and mixed-sensitive changes become full. A manual workflow dispatch always runs full. The impact job reports an explicit skipped/validated distinction; a skipped job must never be described as passing its tests.

## Evidence

Full validation uploads GUI summary, driver report and application log as a short-retention Actions artifact so CI evidence does not vanish with the runner and does not permanently inflate the Git repository. Supply-chain assurance uploads deterministic SBOM/checksum evidence.

## Blocking policy

UI Safety, Full Validation, Hot-Plug, Engineering Governance and Supply Chain are engineering quality gates. The legacy Gitee mirror is retired and must not gate GitHub `master`.

Any workflow added/removed must update `governance.json` in the same change.

## Change-aware execution and evidence

- `docs`: only narrative documentation; skip compiler, GUI, Hot-Plug, and advisory scans. Engineering Governance remains active for all pushes and PRs.
- `ui`: up to six safe leaf-view/UI source files plus documentation; run Node setup, app checks and production build, and UI policy check; skip Rust compiler/packaged GUI where no native or bridge contracts changed.
- `full`: changes to Rust, Tauri configuration, root app, scripts, dependency manifests, workflow rules, native/bridge, or any unknown file. Run every original validation gate. Manual dispatch forces this mode.
- Dependency scanning runs when dependency manifests, security/SBOM scripts, workflows or manual dispatch require it; unrelated source changes do not reinstall `cargo-audit`.
- Classifier exceptions, nonexistent diff boundary, and unexpected file names must fail closed to `full`; no filename patterns may weaken a required release gate.
- Before producing a new Release, manually trigger Full Validation + Supply Chain Assurance at the exact release SHA. Stable tags cannot be moved to shortcut verification.
- GitHub required-check settings must be reconciled with conditionally skipped jobs; branch protection should require the always-running impact/governance jobs instead of skipped child jobs, without weakening full/release evidence.
