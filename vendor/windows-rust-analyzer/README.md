# Windows rust-analyzer runtime

BrowserOS builds and ships its Windows analyzer from the official Rust
1.100.0 source at commit
`0ef1bc386059596dada8fb65b50425aab2ea637a`.
The executable is part of the application resource bundle. Windows LSP does
not choose an analyzer from PATH or execute a rustup shim.

## Reason for the patch

The Windows AppContainer acceptance run found a real failure in the upstream
metadata path: `rustc.exe --print cfg -O` could not start because
`Command::output()` selected the global NUL device for child stdin and
the lowbox could not open that device. The fixed-source patch sets Windows
metadata subprocess stdin to an anonymous pipe. Rust's `Command::output`
implementation closes the parent writer before waiting, so the subprocess
still receives EOF.

The patch does not change device permissions, enable network access, allow
Cargo, enable build scripts or proc macros, or bypass failed metadata probes.
The actual AppContainer test must still resolve a cfg-gated helper through
hover and definition and return real syntax diagnostics. A successful build
alone does not establish that acceptance result.

Primary source locations:

- [Pinned metadata helper](https://github.com/rust-lang/rust/blob/0ef1bc386059596dada8fb65b50425aab2ea637a/src/tools/rust-analyzer/crates/project-model/src/lib.rs)
- [Pinned Command output implementation](https://github.com/rust-lang/rust/blob/0ef1bc386059596dada8fb65b50425aab2ea637a/library/std/src/sys/process/mod.rs)
- [Pinned Windows pipe implementation](https://github.com/rust-lang/rust/blob/0ef1bc386059596dada8fb65b50425aab2ea637a/library/std/src/sys/process/windows/child_pipe.rs)

## Reviewed inputs

`source.json` records the exact upstream repository, commit, subtree,
metadata-file hash before and after the patch, patch hash, original lock
hash, reviewed lock hash and both upstream license hashes.

The independent `Cargo.lock` is the upstream analyzer workspace lock with
three compatible security updates: url 2.5.8 → 2.5.9, form_urlencoded
1.2.2 → 1.2.3, and rustix 1.1.4 → 1.1.6. Each updated checksum and dependency
constraint comes from the official crates.io index. Cargo still builds with
`--locked`; the analyzer lock is audited separately from the BrowserOS
host lock and has its own software bill of materials.

Changing any input requires updating the explicit source contract and
reviewing the patch and lock together. Do not replace the executable with a
downloaded unpatched analyzer or generate a new lock implicitly during a
normal application build.

## Preparation and reuse

On Windows, from the repository root:

```powershell
pwsh -NoProfile -File scripts/build-windows-lsp-runtime.ps1
```

PowerShell 7, Git, rustup and the native MSVC build tools are required. The
builder installs the exact `1.100.0-x86_64-pc-windows-msvc` toolchain
without changing the user's default. It checks the actual compiler release,
commit and host and records its executable SHA-256. The nested Cargo build
uses a separate working directory, Cargo cache and target directory. Parent
Cargo compiler, wrapper, flags and jobserver environment variables are
removed rather than inherited into that build.

CI can supply `-WorkDirectory` under its runner temporary directory.
The default is a dedicated temporary directory. The builder validates the
upstream commit and input hashes before applying the patch, verifies that
only the metadata source and Cargo.lock changed, and builds only the
rust-analyzer binary with `--locked --release`.

The generated, ignored source bundle contains exactly these declared
resource files:

- `src-tauri/resources/windows-lsp/rust-analyzer.exe`
- `src-tauri/resources/windows-lsp/runtime.json`
- `src-tauri/resources/windows-lsp/LICENSE-MIT`
- `src-tauri/resources/windows-lsp/LICENSE-APACHE`
- `src-tauri/resources/windows-lsp/Cargo.lock`

A cached bundle is reused only after validating its source contract,
executable size and SHA-256, lock and license hashes. Unknown files are not
deleted. The builder rejects reparse points in its output directory chain
or any of the five output files.

`runtime.json` uses UTF-8 without a BOM and LF line endings. It
contains no build timestamp or local filesystem paths. Its top-level fields
are `schema_version`, `source`, `artifact` and `build`.
The artifact records the actual analyzer filename, size and SHA-256. The
build section records the pinned build compiler's release, source commit,
executable hash and target. That compiler is distinct from the user's
installed compiler selected later for sandboxed cfg probes.

## Cargo, development and installed execution

Windows `build.rs` invokes the same idempotent builder before the host
includes the generated manifest. Tauri development therefore does not race
a missing resource, and it retains the existing frontend development server
owner.

`build.rs` validates Cargo's `<profile>/build/<package>/out` output
layout and passes the profile directory as `-ExecutableDirectory`.
The builder publishes the same verified five files into that directory's
`windows-lsp` subdirectory, with the manifest last, and verifies them
again. The cached-bundle path performs this publication too. A direct
`cargo run --release` therefore uses the same adjacent runtime as an
installed release. An unexpected Cargo output layout fails explicitly.

Tests and debug builds use only the fixed repository resource directory.
Release builds use only `windows-lsp` beside the running BrowserOS
executable. NSIS maps the same five source-bundle files to that installed
directory. There is no release fallback to a source checkout, environment
override or external analyzer on PATH.

The host embeds the runtime manifest at compile time. At each selection it
requires byte-for-byte equality with the disk manifest, matches its source
contract against the reviewed source.json, checks the executable length and
SHA-256 under the existing operation deadline, and retains native read
handles to the manifest and executable until sandbox staging finishes.
Those handles also pin ancestor paths and deny concurrent file writes or
replacement. The analyzer is staged without enumerating sibling DLLs.

The user's actual rustc is selected separately through the existing rustup
metadata or a non-shim installed toolchain on PATH. The native compiler file
stays pinned while its normal runtime peers are staged into the sandbox.
Invalid configured toolchains, unsafe paths, missing resources and any
integrity mismatch fail closed. Installing a different external
rust-analyzer cannot bypass the bundled-runtime check.

## Acceptance evidence

Three native bundle tests exercise the actual Windows file handles:
successful verification with write/replacement denial, altered source /
manifest / executable / size rejection, and missing-resource / cancellation
rejection. They never execute synthetic bytes.

The existing real analyzer test remains necessary: its cfg-gated helper
requires Windows target metadata before hover and definition can pass, and
the syntax diagnostic, private snapshot, own-package read and foreign-package
denial assertions remain active. Cargo and code execution stay disabled,
and language analysis continues to report
`compilation_verified=false`.

Record CI, installer payload verification and installed GUI results for the
same commit. These source changes alone do not mark native or GUI acceptance
as passed.
