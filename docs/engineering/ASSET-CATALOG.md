# Engineering Asset Catalog

## Canonical machine assets

- Capability registry: `docs/architecture/capability-registry/`
- Semantic registry: `docs/architecture/semantic-registry/`
- Native boundary: `docs/architecture/native-boundary/`
- Engineering governance: `docs/engineering/governance.json`

## Stable baselines

- Tag: `capability-platform-v4-stable`
- Commit: `a6ff92674db98ffad964b784167fdb8c9a98f3cc`
- Final audit: `docs/architecture/capability-platform/FINAL-WORTHWHILE-HOTPLUG-AUDIT-V4-20261008.md`

Historical baseline documents remain evidence; they do not automatically redefine the current stable product baseline.

## Evidence stores

- `logs/acceptance/`: named human/GUI evidence.
- `logs/m0-baseline/`: measured baseline runs and integrity manifests.
- `logs/m0-build-metrics/`: build-footprint evidence.
- `logs/assist/`, `logs/checkpoints/`, `logs/research/`: historical engineering/agent evidence.
- GitHub Actions artifacts: ephemeral CI evidence, especially GUI regression outputs.

## Retention debt

Evidence is valuable but large and historically layered. No destructive cleanup is allowed in this governance pass. A later retention phase should classify permanent baselines, release evidence, reproducible CI artifacts and disposable diagnostics before deleting anything.
