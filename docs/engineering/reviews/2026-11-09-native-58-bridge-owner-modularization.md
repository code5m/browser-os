# BrowserOS Native 58 GUI evidence and bridge modularization (2026-11-09)

## Verified source decomposition

`bridge.rs` previously held approximately 6,590 lines of mixed Tauri commands, state management, grid/tab lifecycle, scripts, credentials and plugin operations. A compatibility facade retains `AppState`, startup/lifecycle integration and the original Tauri registrations; owner-oriented implementations are now in `src-tauri/src/bridge/{browser_commands,resource_commands,grid_runtime,tab_runtime,terminal_commands,system_commands,filesystem_commands,automation_commands,workspace_commands,workbench_commands,credential_commands,integration_commands}.rs`.

No command name, native permission file, WebView creation/hide order or frontend `invoke` signature is intentionally changed. The actual path of each registered command is updated in `native-commands.yaml`; after compilation, three-way command and AppState inventory, runtime semantic checks and 632 Rust regressions are mandatory. The existing one-shot intent, remote source rejection and sensitive-value redaction tests remain enabled.

## Remaining architectural limits

This is physical decomposition and command grouping, **not** a claim that all dependencies are completely independent: adapter modules still access shared `AppState` through `super` until bounded Owner APIs and state ownership are migrated in separate changes. The facade preserves compatibility rather than adding a second source of state. `bridge.rs` source line count is not a standalone quality or performance claim.

## Installed GUI validation

See `docs/architecture/native-boundary/NATIVE_58_INSTALLED_GUI_EVIDENCE_20261109.md`. The frozen 58-item ledger remains pending until the exact installed Release has produced verifiable 58 command results, external origin denial and native side-effect postconditions. No fixture/compile result is substituted for runtime evidence.
