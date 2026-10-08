# Engineering Health

This page is the small-reader entry for BrowserOS engineering status. It is intentionally static: the latest truth is always GitHub Actions, not a cached green badge in the repository.

## Current stable baseline

- Product/platform release: `capability-platform-v4-stable`
- Frozen commit: `a6ff92674db98ffad964b784167fdb8c9a98f3cc`
- Current engineering branch work must not move that tag.

## What must be green before merge

- BrowserOS Engineering Governance
- BrowserOS UI Safety
- BrowserOS Hot-Plug Acceptance
- BrowserOS Supply Chain Assurance
- BrowserOS Full Validation
- Mirror GitHub to Gitee

Full Validation includes architecture/capability/governance, runtime startup, production build, Rust fmt/check/tests, packaged GUI cold start, Full Tauri GUI regression, GUI evidence artifact upload, diff hygiene and full pre-merge.

## Local quick checks

```bash
npm run check:engineering-governance
npm run check:native-semantics
npm run check:evidence-lifecycle
npm run check
npm run build
```

## Machine facts

- Governance registry: `docs/engineering/governance.json`
- Native semantic authority: `docs/architecture/native-boundary/native-commands.yaml`
- Evidence lifecycle policy: `docs/engineering/evidence-policy.json`
- Supply-chain workflow: `.github/workflows/supply-chain-assurance.yml`

## Known blocked item

Release artifact signing/provenance is blocked until a trusted signing identity is deliberately configured. SBOM checksums are integrity evidence, not a signing substitute.
