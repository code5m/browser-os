# BrowserOS Engineering Governance

This directory is the engineering front door for BrowserOS: gates, CI workflows, Git safety, evidence, standards, releases, recovery and peripheral engineering.

## Source-of-truth rule

- Machine truth: `docs/engineering/governance.json`.
- Human documents explain the registry and must not silently become a second registry.
- GitHub `code5m/browser-os` / `master` is the only writable source of truth.
- Gitee automatic mirroring is retired; the legacy workflow is manual-only and must not gate GitHub `master`.
- Frozen Capability Platform v4 baseline: `capability-platform-v4-stable` -> `a6ff92674db98ffad964b784167fdb8c9a98f3cc`.

## Read by task

| Task | Start here |
| --- | --- |
| Change checker/gate | [GATE-MATRIX.md](./GATE-MATRIX.md) |
| Change GitHub Actions | [WORKFLOW-CATALOG.md](./WORKFLOW-CATALOG.md) |
| Locate evidence/registries/baselines | [ASSET-CATALOG.md](./ASSET-CATALOG.md) |
| Change project standards | [STANDARDS-CATALOG.md](./STANDARDS-CATALOG.md) |
| Change Git/release/recovery/diagnostics | [PERIPHERAL-ENGINEERING.md](./PERIPHERAL-ENGINEERING.md) |
| Freeze or roll back release | [RELEASE-AND-RECOVERY.md](./RELEASE-AND-RECOVERY.md) |
| Understand audit and roadmap | [ENGINEERING-AUDIT-V1-20261008.md](./ENGINEERING-AUDIT-V1-20261008.md) |
| Check Rust native semantics | [NATIVE-SEMANTICS.md](./NATIVE-SEMANTICS.md) |
| Check evidence lifecycle | [EVIDENCE-LIFECYCLE.md](./EVIDENCE-LIFECYCLE.md) |
| Check supply chain | [SUPPLY-CHAIN.md](./SUPPLY-CHAIN.md) |
| See small-reader health view | [ENGINEERING-HEALTH.md](./ENGINEERING-HEALTH.md) |

## Mandatory machine gate

```bash
npm run check:engineering-governance
```

The checker rejects unregistered workflows/hooks, missing governed paths, source-of-truth drift, stable-baseline drift, or removal of governance from the main check chain.

## Repository topology

BrowserOS remains one repository for core product and platform work. Internal modularity is enforced by Capability/Native/Semantic boundaries. Extraction to another repository requires an independent public API, release/version lifecycle and proven reuse/ownership need.
