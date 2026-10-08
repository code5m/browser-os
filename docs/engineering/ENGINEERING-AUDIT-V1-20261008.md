# BrowserOS Engineering-System Audit v1 — 2026-10-08

## Verdict

**PASS_WITH_ENGINEERING_DEBT**

BrowserOS already has deep gate, evidence, recovery and architecture machinery. The primary defect is fragmentation: historical phase naming, scattered truths, duplicated execution surfaces and weak system-level discoverability.

## Audited inventory

At the pre-change master `58336477d24354254c4c2364c225d72485f24569`:

- 142 top-level entries under `scripts/`.
- 4 GitHub Actions workflows.
- 2 versioned Git hooks.
- extensive acceptance/baseline/build/security/research/checkpoint evidence.
- Capability, Semantic and Native architecture registries.
- stable Capability Platform v4 release baseline.
- Gitee mirror run for that master commit completed successfully.

## P0 findings closed here

1. No machine-readable engineering-system registry -> added `governance.json`.
2. Core CI push triggers tied to old v2/v3 branches and not direct master pushes -> generalized to every branch push, including `master`, `feature/**`, `fix/**`, `chore/**` and future branch conventions.
3. No dedicated governance CI -> added `engineering-governance.yml`.
4. No single front door for gates/workflows/assets/standards/peripheral engineering -> added `docs/engineering/`.
5. Full GUI structured evidence vanished with CI runner -> upload short-retention Actions artifact.
6. First post-merge master run exposed scenario-5 asynchronous evidence loss even though the driver passed -> keep the assertion unchanged and make the harness wait for the page-side evidence POST.

## P1 retained debt

1. Evidence volume/retention: large historical evidence; no destructive cleanup without explicit retention policy.
2. Workflow duplication: full validation repeats focused checks. Retained for defense-in-depth; optimize only with equivalent coverage proof.
3. Historical naming: many M0/M1/v2/v3 path names. Renaming now risks broken references.
4. No generated visual engineering dashboard yet.
5. Supply-chain automation (dependency review, SBOM, provenance/signing) requires a dedicated phase before becoming blocking.
6. Direct-push policy tradeoff: master allows normal direct push, so GitHub CI on master is post-push detection rather than a pre-push server gate. Versioned local pre-push + server non-force/non-delete protection remain the intended safety model.

## Target architecture

```text
BrowserOS Product
  -> Capability / Native / Semantic architecture
  -> Engineering Governance Registry
       -> Gates
       -> GitHub workflows
       -> Git hooks
       -> Standards
       -> Assets / Evidence
       -> Release / Recovery
       -> Backup mirror
  -> Machine governance checker
  -> Local + CI + packaged-GUI + release evidence
```

## Roadmap

- v1 (this change): registry, catalogs, drift checker, version-neutral CI coverage, CI GUI evidence.
- v1.1: evidence retention classes + repository-size budgets.
- v1.2: supply-chain review (dependency review, SBOM, checksums/signing/provenance).
- v1.3: generated health report for gate duration/flakiness/build size/GUI evidence.
- v2: extension-platform governance for public API compatibility, plugin package signing, permissions and isolation.

## Non-goals

No product behavior change, no maturity relabeling, no v4 stable tag movement, no mass evidence deletion, no repository split.
