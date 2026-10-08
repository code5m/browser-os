# GitHub Actions Workflow Catalog

Workflow registration is machine-enforced by `docs/engineering/governance.json`.

| File | Role | Trigger policy |
| --- | --- | --- |
| `capability-v2-ui-safety.yml` | fast UI policy feedback | all branch pushes, PR to master, manual |
| `capability-v2-validation.yml` | full BrowserOS validation | all branch pushes, PR to master, manual |
| `capability-v3-hotplug.yml` | focused Hot-Plug acceptance | all branch pushes, PR to master, manual |
| `engineering-governance.yml` | governance drift | all branch pushes, PR to master, manual |
| `supply-chain-assurance.yml` | supply-chain gate and SBOM evidence | all branch pushes, PR to master, manual |
| `mirror-to-gitee.yml` | retired legacy mirror placeholder | manual only; no branch/tag push trigger |

The v2/v3 filenames are retained to avoid unnecessary historical path churn. Their display names and triggers are version-neutral; they protect current BrowserOS. All branch pushes are covered by the active quality gates so branch naming (`feature/**`, `fix/**`, `chore/**`, or another convention) cannot bypass validation.

## Evidence

Full validation uploads GUI summary, driver report and application log as a short-retention Actions artifact so CI evidence does not vanish with the runner and does not permanently inflate the Git repository. Supply-chain assurance uploads deterministic SBOM/checksum evidence.

## Blocking policy

UI Safety, Full Validation, Hot-Plug, Engineering Governance and Supply Chain are engineering quality gates. The legacy Gitee mirror is retired and must not gate GitHub `master`.

Any workflow added/removed must update `governance.json` in the same change.
