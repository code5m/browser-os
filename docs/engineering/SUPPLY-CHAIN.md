# Supply-Chain Assurance

BrowserOS uses lockfiles as the dependency baseline and generates deterministic CycloneDX 1.6 SBOMs directly from `package-lock.json` and `src-tauri/Cargo.lock`.

The supply-chain workflow performs:
- `npm ci` lockfile validation;
- production `npm audit --omit=dev` evidence capture to `artifacts/sbom/npm-audit-prod.json`;
- Node advisory baseline enforcement: critical advisories always fail, and any non-baselined moderate/high/critical advisory fails;
- Rust advisory scan with `cargo audit`;
- deterministic Node/Rust CycloneDX SBOM generation;
- SHA-256 checksum generation;
- 14-day GitHub Actions artifact retention;
- GitHub Dependency Review on pull requests where available.

## Current Node advisory baseline debt

The Node audit baseline records historical production-audit debt already present when Supply-Chain Assurance was introduced. It is not a license to add new risk: the workflow fails on any new non-baselined moderate/high/critical package and on every critical advisory.

Baselined packages as of 2026-10-08:

| Package | Severity class observed | Advisory source |
| --- | --- | --- |
| `@vue/server-renderer` / `vue` | high | GHSA-g2v6-rqmx-r4w6 |
| `nanoid` | high | GHSA-2v37-7h3g-55p8 |
| `source-map-js` | high | GHSA-68fv-2mgg-jv7q |
| `postcss` | moderate | GHSA-fxqj-rqcc-2cmp |

A dedicated dependency-upgrade task should retire this baseline by upgrading Vue and affected transitive packages, then removing the package allowlist from `.github/workflows/supply-chain-assurance.yml`.

No release-signing identity/key is configured by this project. Governance records signing as `BLOCKED_NO_RELEASE_SIGNING_IDENTITY`; BrowserOS must not claim signed releases until a trusted identity/provenance path is deliberately configured. SBOM checksums are integrity evidence, not a substitute for signing.
