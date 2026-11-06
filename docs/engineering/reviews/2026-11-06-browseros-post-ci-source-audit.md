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

Acceptance: `cargo fmt --check`, `cargo check --locked`, `cargo test --locked`, `npm run check`, exact-SHA frontend production budget, Full Validation, and the *new exact-SHA* installed GUI regression must pass before merging. Current CI size reporting no longer emits the obsolete M0 percentage breach; approved absolute byte caps, same-run master increment, frozen remote ACL and local user-owned untracked directories remain unchanged.

## Follow-up source changes after `b7279472` (same integration PR)

- **Coding bootstrap identity:** `public_api::prepare_coding_session` now asks the Coding Owner to atomically persist a new real Session UUID before the Foundation TaskRegistry operation starts. The exact persisted UUID is reused for task scope, and the session ID is streamed to the UI for cancellation/recovery on interrupted model responses. No session is created by read-only Coding/tree, LSP or context actions. The new Rust regression checks disk persistence before task creation, registry UUID equality, and invalid/session-reuse cases.
- **WebKitGTK initial viewport:** main-tab creation, hibernation restore and recovery no longer initialize a WebView with a `1x1` viewport. The Browser Owner constructs it offscreen with a stable 800x600 viewport and `visible=true` to avoid the plugin's initial `webview.hide()` path on a rendering view; the frontend still controls final positioning. Grid child starts with its host 400x300 viewport. This source change does not remove the plugin's API-level `set_visible(false)` capability or the host *window* `win.hide()` used for Grid, and does not claim a GUI hang fix without installed-client reproduction.
- **Retired M0 cumulative verdicts:** active CI and `measure-build-metrics.py` no longer emit or enforce the obsolete cumulative historical threshold, `BREACH` or exception-PASS status. Exact production SHA256/absolute editor, Core+Shared, total budgets and the fresh same-run master increment gate remain fail-closed. `check-frontend-size-dual-budget.py` compares the AST of `collect_dist` and `gzip_bytes` to master to prohibit unreviewed measurement drift while retiring the diagnostic legacy comparator. Original archival measurement snapshots are not used as a gate.
- **Unresolved external validation:** 58 Native effects still lack separately demonstrated installed-GUI side-effect evidence; Bilibili and WeChat require real authorized account actions, and the large `bridge.rs` still needs bounded Owner extraction. These are not replaced with fixture PASS assertions.

Do not merge until the **new exact commit** has all selected mandatory Full Validation and packaged-GUI checks PASS. Any source test results reported for this follow-up must be tied to that commit, not `d323bbb4` or prior run IDs.

## Frontend size governance update (2026-11-06)

User approved converting the three absolute size envelopes and same-run master +1% threshold from merge-blocking failures into warning-only audit signals. We retain exact SHA256 production verification, complete asset/owner accounting, pinned dependency and eager editor checks, provenance checks, build and all other native/security/GUI quality gates as strict failures. The `check-frontend-size-dual-budget.py` JSON now distinguishes `errors` (blocking integrity issues) from `warnings` (nonblocking size observations), and base-build inability is visible as `UNAVAILABLE` rather than a false successful comparison. The historical M0 45% breach remains retired.
