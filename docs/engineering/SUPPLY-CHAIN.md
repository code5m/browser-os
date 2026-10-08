# Supply-Chain Assurance

BrowserOS uses lockfiles as the dependency baseline and generates deterministic CycloneDX 1.6 SBOMs directly from `package-lock.json` and `src-tauri/Cargo.lock`.

The supply-chain workflow performs:
- `npm ci` lockfile validation;
- production `npm audit --audit-level=high`;
- Rust advisory scan with `cargo audit`;
- deterministic Node/Rust CycloneDX SBOM generation;
- SHA-256 checksum generation;
- 14-day GitHub Actions artifact retention;
- GitHub Dependency Review on pull requests where available.

No release-signing identity/key is configured by this project. Governance records signing as `BLOCKED_NO_RELEASE_SIGNING_IDENTITY`; BrowserOS must not claim signed releases until a trusted identity/provenance path is deliberately configured. SBOM checksums are integrity evidence, not a substitute for signing.
