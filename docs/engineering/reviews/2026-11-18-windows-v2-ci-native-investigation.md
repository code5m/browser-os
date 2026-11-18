# BrowserOS Windows/Linux V2: 2026-11-18 CI native evidence and ACL trial

Target: feature/browseros-windows-linux-v2-onepass-20261114.
No master merge; real GitHub Actions push runs are required for acceptance.

## Verified results

- Push of no-source-change commit 470be906 triggered six GitHub Actions workflows, including Windows/Linux V2 run 44270327758.
- The first run passed static-contract, linux-native and windows-bfc-loopback.
- Windows native `windows-cargo` **FAILED**: 27/29 filters passed; two failures were
  `private_snapshot_requires_own_package_sid_and_rejects_other_profiles` and
  `real_rust_analyzer_runs_against_private_snapshot`.
- AppContainer positive control did not produce the expected own-source contents (zero matches), but the combined `cmd /C type ... & type ...` fixture also lacked stderr in its assertion. That evidence *alone* cannot distinguish ACL failure from shell-command quoting.
- LSP test logged `LSP_SNAPSHOT_HOST_STATE root_dir=true project_file=true source_file=true` and a package-SID RX ACE on the project manifest. The lowbox still did not return publishDiagnostics within 300 seconds; stderr showed failed `rustc --print cfg` and a file-watcher warning.
- The first Supply Chain audit failed on `libz-sys 1.1.31` RUSTSEC-2026-0544, patched in 1.1.33. The checked registry index checksum was added to Cargo.lock in 7f15295.
- The second run 44271673395 expands `windows_process::` and the full Windows native suite; these results remain evidence pending until jobs finish.

## Targeted trial

- Give Windows lowboxes explicit RX traversal/list permissions on *directories*, including private package ancestors, but use **noninherited** `S-1-15-2-1` ACEs. Do not add the generic SID on source files. Continue granting each file only to its owner/package SID, current user and SYSTEM.
- Preserve the separate package-SID RX on directories and files; no network access, no host-source write rights and no build execution privileges are added.
- The generic AppContainer group may enumerate the directory names when it knows the private path. That residual metadata exposure is not equivalent to read access on the source files and must be evaluated against the risk model; the native cross-SID negative test remains a mandatory gate.
- Fix test-only ACL diagnostics to capture the first `icacls` ACE and to include lowbox stderr rather than misclassifying empty stdout.
- Add an independent, simple `cmd.exe /C type own-source` positive control before testing combined commands or cross-profile denial, then retain overwrite and create-negative checks.

## Acceptance criteria on the new exact HEAD

1. Windows `cargo check --locked` and native harness compilation PASS.
2. Standalone own-package source read PASS; foreign-package read denied; source overwrite and new-file creation denied.
3. Real rust-analyzer diagnostics, symbols, hover and definition PASS without snapshot URI leaks.
4. `windows_process::` and full native suite classify the developer-host 21 failures; do not claim host limitation without runner evidence.
5. Supply Chain `cargo audit` confirms no vulnerable `libz-sys 1.1.31` remains.

A passing static contract does not satisfy these dynamic acceptance gates. If an ACL trial gives any unrelated lowbox access to private file **content**, it must be reverted rather than bypassing the failing security test.
