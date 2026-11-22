# Windows V2 native LSP regression: follow-up source repair and verification contract

Date: 2026-11-18
Branch: feature/browseros-windows-linux-v2-onepass-20261114
Base HEAD: 39386d50fca3e3e7b897ce40c4a0cdf5a1e3cd3c
Scope: no new branch, no master merge, no CI or workflow dispatch.

## 1. Native results from Windows operator (not rerun in this change)

- `cargo test --locked --no-run`: completed, 1m49s, unused warnings only.
- `vendor/portable-pty`: 4 + 1 passing, including the input contract.
- `security_policy::`: 87 pass / 0 fail.
- `capabilities::git`: 65 pass / 0 fail.
- `lsp`: 10 pass / 2 fail. The real rust-analyzer acceptance reported `LSP_DIAGNOSTICS_TIMEOUT`, no process exit and warnings about `rustc --print cfg` missing plus a file-watch path.
- `windows_process::`: 3 pass / 2 fail on that host; the failures still need a distinct-host comparison. `ERROR_INVALID_HANDLE` is not proof of a missing host console.
- The connected Desktop Commander Windows device is OFFLINE; this change **cannot** claim a new Windows run or actual `icacls` inspection.

## 2. Root cause statements and evidence level

| Observation | Evidence class | Interpretation |
|---|---|---|
| The no-Cargo branch removed Cargo/rustc staging | Source confirmed | The rustc cfg probe can print `program not found`; this is a regression in environment parity, but not by itself proof that the analyzer cannot parse Rust |
| `rust-analyzer` prints `Input watch path is neither a file nor a directory` | Native log | Possible missing/inaccessible path, but this warning is also tracked as upstream issue rust-lang/rust-analyzer#19560; cannot infer that source files are absent |
| Old `fd1957b` added `ALL APPLICATION PACKAGES` ACEs; later code removed them | Source confirmed | Restoring a global-app-package ACE on sensitive source files could make them readable by unrelated app containers; **do not** use as an undocumented workaround |
| `CreatePseudoConsole` returned `0x80070006`, input/output handles reported live | Native log | Does not prove invalid console-host hypothesis. Valid handle flags do not establish valid types/pipe direction or host policy; compare to original Windows runner |
| New inline project graph fixes watcher/manifest cause | Design hypothesis | Requires actual LSP publishDiagnostics and symbol/hover/definition success to mark fixed |

Official background:
- https://rust-analyzer.github.io/book/non_cargo_based_projects.html (inline `linkedProjects` object graphs are supported).
- https://rust-analyzer.github.io/book/configuration.html (initialize configuration; disabling sysroot discovery).
- https://github.com/rust-lang/rust-analyzer/issues/19560 (the watch-path warning exists upstream).
- https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer (dual-principal DACL).
- https://fuchsia.googlesource.com/third_party/rust/+/e9e936cfa8dfdd1b7320a219316b6bd6dbccf300/src/tools/rust-analyzer/crates/project-model/src/rustc_cfg.rs (cfg probe errors use fallback flags).

## 3. Repair implemented

1. `write_reviewed_rust_project` still creates a bounded local source graph in the private snapshot; `reviewed_project_for_initialize` reopens it through `safe_paths::read_regular`, validates every root against the pinned snapshot and rejects malformed, missing, symlink/reparse or external paths. It sends the same parsed graph **inline** in initialize `linkedProjects`, avoiding an additional lowbox watcher/open of `rust-project.json` itself and avoiding fallback to Cargo auto-discovery.
2. The LSP initialize request also supplies the explicit snapshot workspace folder while keeping the private AppContainer, `cargo.sysroot=null`, no proc-macro, no build scripts and no compilation claim. The rustc cfg warning is not suppressed by editing logs; actual native LSP semantics must pass.
3. Native package-SID test now probes both the copied source file **and** `rust-project.json` from within the same lowbox, and verifies a foreign lowbox cannot read the private source. Two independent lowbox write-negative tests cover overwriting an existing source and creating a new one in an RX directory.
4. On test-only LSP diagnostics failure, output includes host file/dir existence and a bounded `icacls` ACE excerpt for snapshot root and project manifest, excluding full user paths. This distinguishes a missing source from a failed lowbox access check when Windows test execution becomes available. It does not log actual source contents or secrets.
5. The static source contract now asserts the inline graph, new access checks and no-Cargo staging boundary. The test matrix already includes the native LSP and package-SID acceptance tests; no workflow was run.

## 4. Native verification: same SHA, interactive-capable Windows

From the clean feature branch:
```powershell
git rev-parse HEAD
git status --short
cargo test --manifest-path src-tauri/Cargo.toml --locked --no-run
cargo test --manifest-path src-tauri/Cargo.toml --locked lsp -- --nocapture --test-threads=1
cargo test --manifest-path src-tauri/Cargo.toml --locked windows_process:: -- --nocapture --test-threads=1
```

Minimum acceptance:
- LSP startup; `LSP_RA_EVENT textDocument/publishDiagnostics file:///...` for a real reviewed Rust source.
- 4 protocol paths: diagnostics, documentSymbol, hover, definition, with the private snapshot file URIs remapped to the authorized workspace.
- A deliberately broken Rust source gets nonempty parser diagnostics; `compilation_verified=false` remains explicit.
- Owner package SID reads its source plus project graph; unrelated package SID cannot; both overwrite and create operations in snapshot are refused.
- Any LSP failure includes sanitized test-only file existence and per-object ACE evidence. Do not treat `rustc cfg` warnings or watcher warnings in isolation as a root cause.
- Full 21 local failures require test names + per-failure stderr; do not reclassify all of them as environment noise or all as product bugs.
- Compare ConPTY tests with a genuine Windows runner/desktop, retain exact HRESULT, OS build, pipe handle type and session token, and isolate any product defect before changing core PTY primitives.

## 5. Release gate

State: SOURCE_PATCHED / WINDOWS_NATIVE_UNVERIFIED. No Cargo invocation, CI, Windows GUI, NSIS or Docker execution occurred in this source-editing session. No new native PASS may be claimed from historical SHA. If the in-process rust-analyzer no-Cargo graph is still inaccessible, the next step is to use actual lowbox `icacls` and package SID positive/negative traces to correct the ACL — **never** grant `ALL APPLICATION PACKAGES` on private source files or escape the sandbox merely to turn tests green.
