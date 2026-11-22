# BrowserOS GLib 0.18.5 backport

This is the single GLib crate from gtk-rs/gtk-rs-core tag `0.18.5`, immutable
commit `42b9caf98e03ded086362d9653ca58fe94dc8658`. The checked-in provenance
records every upstream Git blob. This import was verified against Git objects;
it does **not** claim that the crates.io archive was downloaded or verified.
The registry checksum is retained only as the upstream advisory/SBOM identity.

## Local changes

- Backport exactly upstream commit
  [b5a4071e439bef2b5eea76c3aa25e5ae84839e34](https://github.com/gtk-rs/gtk-rs-core/commit/b5a4071e439bef2b5eea76c3aa25e5ae84839e34):
  `VariantStrIter::impl_get` uses `let mut p` and passes `&mut p` to the
  variadic C out-parameter. This removes the undefined behavior described by
  [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html).
- Remove only four sibling `path` attributes from the upstream Cargo manifest.
  The existing GLib/GObject/GIO sys and macro version constraints, features,
  public API and package version remain unchanged.
- Materialize the upstream `LICENSE` and `COPYRIGHT` symlinks as their exact
  root-file contents. Their link and target Git identities are both recorded.
  Remove only LICENSE's final extra blank line to satisfy the existing whitespace
  gate; the original bytes are reconstructed and checked by the verifier.
- Add this note and the provenance manifest. The repository attributes preserve
  these files as exact bytes on every platform.

No GTK window positioning, WebView lifecycle or platform permissions change.
The original MIT license and copyright notice remain in this directory.
System GNOME libraries retain their own licensing terms.

## Evidence and advisory visibility

`node scripts/check-vendored-glib.mjs` checks the complete source inventory,
reverses only the documented edits, and compares the resulting Git blob IDs
with the immutable upstream source. It also verifies the main and standalone
browser-tabs Cargo patch configuration. A string match alone is insufficient.

The behavioral regression is
`tauri-browser-tabs/crates/tauri-plugin-browser-tabs/tests/glib_variant_iter.rs`.
CI must run it with Cargo `--locked --release`, retain its real output, and
verify all five named tests, the optimized-build markers and zero ignored tests.
The same CI gate checks actual Cargo metadata for one resolved GLib 0.18.5 from
this path, with no registry copy and no accidental workspace membership.
Source integrity does not establish a runtime PASS.

Cargo-audit 0.23 skips path and non-registry dependencies. The Supply Chain
workflow therefore retains its ordinary host scan **and** audits a clearly
labelled registry-origin projection of the current lock. The projection puts
this one package back at its original crates.io identity only for upstream
advisory discovery. It is never the application build lock. The upstream
RUSTSEC warning remains visible even when the local two-line fix is verified;
there is no advisory ignore, fake package version or weakened audit threshold.
The SBOM identifies the local backport and its upstream origin separately.

Keep the source inventory, provenance, optimized regression and registry-origin
scan when updating this fork. Remove the backport only when the whole consumer
graph resolves to an upstream fixed version and the same regression succeeds.
