# BrowserOS Gate Matrix

Machine registry: `docs/engineering/governance.json`.

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
| Build metrics | `scripts/pre-merge.sh` | merge | frozen metrics baseline |
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
