# BrowserOS Supply Chain — strict production advisory remediation

The temporary baseline permit list has been removed from `.github/workflows/supply-chain-assurance.yml`.
`npm audit --omit=dev --audit-level=moderate` is now blocking. Real GitHub Actions diagnostic showed all five previously baselined production issues had compatible fixes. The generated lockfile upgrades Vue/@vue packages to 3.5.43, nanoid to 3.3.20, PostCSS to 8.5.29 and source-map-js to 1.2.2, retaining the existing app-level semver constraints.

The original advisory diagnoses covered:
- GHSA-g2v6-rqmx-r4w6 — Vue server renderer
- GHSA-2v37-7h3g-55p8 — nanoid
- GHSA-fxqj-rqcc-2cmp — PostCSS
- GHSA-68fv-2mgg-jv7q — source-map-js

Neither npm dependency audit PASS nor CycloneDX SBOM + SHA-256 means release signatures exist. Rust cargo audit remains blocking in the supply-chain workflow. Dependency Review remains separately limited by repository graph permissions and must not be represented as an unconditional blocking proof.

Do not restore the historical package allowlist to greenwash future advisories. All security and build/test results must come from the checked commit SHA.
