# BrowserOS Supply Chain — strict production advisory remediation

The temporary baseline permit list has been removed from `.github/workflows/supply-chain-assurance.yml`.
`npm audit --include=dev --audit-level=moderate` is now blocking. Real GitHub Actions diagnostic showed all five previously baselined production issues had compatible fixes. The generated lockfile upgrades Vue/@vue packages to 3.5.42, nanoid to 3.3.20, PostCSS to 8.5.29 and source-map-js to 1.2.2, retaining the existing app-level semver constraints. Vue 3.5.42 was selected after a CI probe proved both strict production audit PASS and build growth 25.12%, below the frozen 25.2% budget.

The original advisory diagnoses covered:
- GHSA-g2v6-rqmx-r4w6 — Vue server renderer
- GHSA-2v37-7h3g-55p8 — nanoid
- GHSA-fxqj-rqcc-2cmp — PostCSS
- GHSA-68fv-2mgg-jv7q — source-map-js

Neither npm dependency audit PASS nor CycloneDX SBOM + SHA-256 means release signatures exist. Rust cargo audit remains blocking in the supply-chain workflow. Dependency Review remains separately limited by repository graph permissions and must not be represented as an unconditional blocking proof.

Do not restore the historical package allowlist to greenwash future advisories. All security and build/test results must come from the checked commit SHA.

## Pinned native source backports

The host keeps the GTK3-compatible GLib 0.18.5 API and uses the checked-in
single-crate source at `src-tauri/vendor/glib`. Its source manifest names the
immutable upstream Git commit and every upstream blob. Only the upstream
VariantStrIter soundness fix is backported; four sibling Cargo paths are
normalized for standalone registry resolution, the license symlinks are
materialized, and the license's extra final blank line is removed for the
existing whitespace gate. The package version is not falsified. The import
does not claim that a crates.io archive was downloaded or verified.

`check-vendored-glib.mjs` checks the full inventory against the upstream
identities after reversing the explicitly documented changes. Actual locked
Cargo metadata must resolve one local GLib 0.18.5 and keep it outside the host
workspace membership. Five public-API iterator regressions run with
`--locked --release`, including forward/reverse/skip/last operations and UTF-8
and empty values. Their exact execution results and optimized-build markers
are required; source verification alone is not runtime evidence.

Cargo-audit 0.23 does not inspect path/non-registry crates. Consequently the
ordinary host audit is retained, and a separately named registry-origin
projection restores GLib's original registry identity for advisory discovery.
That projection is never the application build lock. The upstream
RUSTSEC-2024-0429 warning remains visible while the local backport has its own
source and runtime evidence. No advisory ignore or relaxed threshold is added.

The packaged Windows rust-analyzer has its own immutable Rust source, patch
and Cargo lock at `vendor/windows-rust-analyzer`. The Supply workflow separately
audits that lock rather than assuming the host lock covers the sidecar.
`source.json` pins the source revision, patch, lock and license hashes; the
runtime build/package gate checks the actual bundled executable and licenses.

Supply artifacts include three CycloneDX inventories (`node.cdx.json`,
`rust.cdx.json`, `rust-analyzer.cdx.json`). GLib is explicitly labelled as a
modified local backport with its registry origin; its original archive
checksum is not presented as the hash of modified source. The sidecar inventory
records its own fixed source/target and lock. These are complete lockfile
inventories across targets, not proofs of compiled reachability or signatures.

Both vendor trees and their provenance checkers route to full/native and
Node/Rust supply validation, including changes only to a vendor README,
license, patch or source manifest. The original formatting, whitespace,
locked build, audit and package gates remain in force.
