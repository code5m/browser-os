# GitHub Actions Workflow Catalog

Workflow registration is machine-enforced by `docs/engineering/governance.json`.

| File | Role | Trigger policy |
| --- | --- | --- |
| `capability-v2-ui-safety.yml` | fast UI policy feedback | master + feature/** push, PR to master, manual |
| `capability-v2-validation.yml` | full BrowserOS validation | master + feature/** push, PR to master, manual |
| `capability-v3-hotplug.yml` | focused Hot-Plug acceptance | master + feature/** push, PR to master, manual |
| `engineering-governance.yml` | governance drift | master + feature/** push, PR to master, manual |
| `mirror-to-gitee.yml` | backup distribution only | branch/tag push |

The v2/v3 filenames are retained to avoid unnecessary historical path churn. Their display names and triggers are version-neutral; they protect current BrowserOS.

## Evidence

Full validation uploads GUI summary, driver report and application log as a short-retention Actions artifact so CI evidence does not vanish with the runner and does not permanently inflate the Git repository.

## Blocking policy

The first four workflows are engineering quality gates. Gitee mirror is backup distribution, not product-quality authority.

Any workflow added/removed must update `governance.json` in the same change.
