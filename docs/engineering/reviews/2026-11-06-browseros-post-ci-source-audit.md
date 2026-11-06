# BrowserOS source audit and reliability follow-up (2026-11-06)

Scope: `code5m/browser-os`, integration PR #59. This review reads the latest source and distinguishes proven changes from unresolved observations. It does not reuse old master snapshots as current truth.

## Fresh evidence before this follow-up

- `d323bbb4` had `BrowserOS Full Validation` PASS, including the separately requested `packaged-gui` native-regression run 42158414209. CI was on the exact committed source.
- `src-tauri/src/main.rs` still synchronously called `grid_manager.reposition_visible()` in the GTK `Moved/Resized` callback. `GridProcessManager::send()` holds a UDS writer mutex and writes to a connection with an up-to-2s write timeout. A stalled Grid child could freeze the main loop; this is a **source-confirmed blocking path**, not proof of the user's earlier hang's unique root cause.
- The `CloseRequested` and `ExitRequested` handlers synchronously ran `ShutdownCoordinator::shutdown()`; callbacks include tab close, terminal/process-group cleanup and Grid shutdown, which should not run on the GTK event loop.
- `framework/ai.rs::coding_service` created every `ProductionExecution` via `begin("coding")`, which assigned a random session UUID and nil workspace UUID, even when the incoming Coding request supplied a valid durable `session_id`.
- `src-tauri/src/bridge.rs` remains 6,577 lines and historical source audit still reports **58 native effects requiring GUI proof**, a serious maintainability/audit debt. Moving 6,000 lines mechanically without business Owner tests would not resolve it.
- `check-native.mjs --allow-diff` also reports historical warnings around 1x1 WebView initialization/hibernation and `webview.hide()`. These need context-specific packaged-GUI reproduction; warning text alone does not prove a new regression.

## Implemented in this follow-up

1. **Grid geometry**: `request_reposition_async()` is the only main-window move/resize path. An atomic latest-generation counter and one worker coalesce updates; a new event during a slow UDS write causes a replay after completion. Startup without an AppHandle does not touch sockets. Background slow-send diagnostics report only timing and coalesced generations. Focus-restoration UDS work is also moved off GTK.
2. **Graceful exit**: `CloseRequested` prevents closing while one background cleanup worker runs. `ExitRequested` prevents premature termination until a complete report exists. Cleanup reports failures separately; the final exit is requested on the GTK event loop after cleanup. A duplicate close/exit event cannot start a second cleanup transaction.
3. **Coding task scope**: requests with a real persisted `session_id` use that exact UUID and a stable, domain-separated SHA-256 (UUIDv8-format) workspace identity derived from the canonical root; no project path/user text is recorded in the TaskRegistry. First-time bootstrap requests without an existing session remain ephemeral and must not be represented as durable tasks.
4. Regression contracts cover the source wiring, with Rust unit tests for update coalescing, duplicate-shutdown claiming, scoped identity and invalid session input.

## Remaining evidence / real limitations

- **Packaged GUI:** the latest follow-up commit requires its *own* installed-client CI run. Previous PASS on `d323bbb4` is not evidence for newly changed GTK shutdown/position code. Test move/resize with a deliberately slow Grid UDS peer; final geometry; close and exit while Grid/WebKit/Terminal are active; confirm no orphan PIDs and no app hang.
- **P2 Native Owner split:** `bridge.rs` is still a broad orchestration module. Move vertical business logic to bounded Owners incrementally with unchanged ACL semantics, and prove execution and side effects. The 58 pending native GUI effect reviews cannot be called complete on static checks.
- **Bilibili:** a real authenticated tab and local BFC service are needed to confirm live favorites after login; HTTP 412/challenge must remain `limited`, never `logged-out` or `synced`. The DOM/local readback implementation has contracts, but a mock is not login evidence.
- **WeChat:** end-to-end `draft/add` delivery requires a real authorized account; never auto publish. Local keyring/persistence contracts alone do not prove upstream API permissions.
- **Coding:** an initial bootstrap without a saved `session_id` intentionally uses a temporary TaskRegistry identity because the durable ID is created by `coding.rs`. TaskRegistry is not the persisted Coding session store.
- **WebKitGTK tab visibility:** the historical 1x1/hide warnings need reproducible GUI evidence before modifying rendering-sensitive initialization.
- **Performance:** cold-start RSS/PSS, navigation event latency, memory pressure and long-run regression should be measured on an installed package; successful `cargo test`/headless Vite builds do not establish them.

Acceptance: `cargo fmt --check`, `cargo check --locked`, `cargo test --locked`, `npm run check`, exact-SHA frontend production budget, Full Validation, and the *new exact-SHA* installed GUI regression must pass before merging. Immutable M0 history, approved budgets, frozen remote ACL and local user-owned untracked directories are unchanged.
