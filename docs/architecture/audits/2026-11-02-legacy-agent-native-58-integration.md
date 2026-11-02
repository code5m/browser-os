# BrowserOS Agent Owner and 58 Native commands: implementation and residual evidence

- Parent PR: #51, branch `fix/capability-runtime-agent-chat-20261102`; **do not merge into master without explicit approval**.
- This file captures the implementation added after `a677db23` and its actual security boundaries. It does **not** claim installed GUI or 58 separate user-confirmation E2E passes.

## Agent task Owner

The single process-wide `capabilities::agent::execution_owner::ProductionExecution` now covers ordinary chat, project-scoped Coding, saved legacy Agent-definition chats (read-only), and the Agent Channel's user-invoked read-tool and routed-fixture entrypoints. They share the same Foundation `TaskRegistry` for task-state creation, running, success/failure, cancellation-on-drop and terminal retirement. Model credentials remain behind Framework Keyring, file editing remains behind Coding Owner, and Browser/Git/Database tools stay behind their respective Public APIs.

`legacy_chat` is a new safe action of the existing `ai_service` command (not a new permission-bearing native command): it accepts a validated AgentDef and only the existing configured chat model, rejects A2A delegation and ALL declared capabilities, bounds its system prompt and message history, and passes an empty tool list. Saved Agent blueprints in Blueprint Studio can invoke it and cancel by UUID. Legacy `agent_install`, historical `agent_run`, `agent_chat` and Skill execution entrypoints are still disabled, rather than being silently exposed as privileged processes. No unsafe write tool is implicitly authorized by an Agent JSON definition.

Agent Channel's approved read tool now shares the production task registry; its authorization remains a signed, one-use ToolRegistry permit bound to command/resource/arguments. The synthetic benchmark implementation retains a nested fixture-local TaskRegistry by design; it creates mock tasks only, with no external model or native effects. This does not imply the entire experimental Rig AgentRun graph has been migrated into production.

## Native command runtime enforcement

**58/58** historical commands have Tauri ACL declarations; **56/58** additionally have per-handler main-WebView source guard, bounded string/coordinate payloads, and/or existing domain validations. **2/58** (`report_resources`, `report_title`) intentionally allow narrowly gated remote telemetry, and use their existing origin-specific source and payload controls. The remaining three page commands (`tab_new`, `tab_close`, `tab_list`) now have distinct `#[tauri::command]` main-WebView wrappers with bounded inputs, and separate crate-private `_native` implementations called by the trusted Rust bootstrap/GUI-regression and Browser Owner paths. All **58/58 native IPC handlers** therefore carry either the explicit main-window guard (56) or the original narrowly scoped remote telemetry policy (2).

`shared/native_legacy_policy.rs` rejects untrusted tab/grid WebViews for 56 commands and fixes bounded text and numerical parameters; security regression scans all 58 actual Rust commands, ensuring their guard cannot be accidentally removed. No Tauri remote WebView is given new permissions. The actual native code path for `eval_in_tab`, `hide_all_webviews`, `tab_activate`, and `tab_position` also has a main-WebView binding when called by the trusted Rust GUI regression path.

For irreversible `session_delete` and `snippet_remove`, ordinary local main-window ACL is no longer accepted as sufficient: the UI requires a real user-confirm button and obtains a **single-use random UUIDv4 intent** bound to exactly that action and resource ID. The Rust handler validates, consumes, and audits the intent; an omitted, mismatched, expired or replayed intent fails before deletion. `IntentRegistry` is bounded to 256 entries and prunes expired ones on new issue. No model, tool or remote webpage can mint one of these main-window-only delete scopes.

Other Native actions are classified by **their existing UI gesture, Owner, and side-effect semantics**, not by forcing a modal for ordinary navigation, closing a tab, clipboard use, background resource reports or read-only list operations. A blanket 58-dialog policy would break BrowserOS; it is not silently adopted. Actual installed GUI user-gesture attribution and special-risk effects such as script snippets, JavaScript injection and OS default-browser modification still require scenario-specific end-to-end evidence.

## Verification and known limits

- `node tests/check-native-legacy-security.mjs` covers the exact 58 native command paths with 53 main guards / 2 remote telemetry guards / 3 native-internal exceptions.
- `node scripts/check-native-capability-boundaries.mjs`, `node scripts/check-native-owner-effect-audit.mjs --check`, and `npm run check:rust-contracts` detect registry/physical route drift. The policy helper and shared Foundation task owner have Rust unit tests.
- `node scripts/check-native.mjs --allow-diff` was used for the explicitly requested Native security modification while the danger-zone `bridge.rs` diff was uncommitted. Its historical GUI WebKit warnings were **not suppressed**. The normal checker requires the worktree to be clean of such edits (no bypass in CI).
- This branch changes Tauri command Rust-injected `Webview` arguments in some legacy commands, uses wrappers for tab_new/close/list, and adds `intent` to two deletes. These are review-sensitive native interface changes; independently verify installed GUI and generated cross-layer contract after Rust CI.
- Do NOT label user-confirmation validation `GUI_PASS` or full 58 effect review `ALL_PASS` until installed package, remote-deny, payload-limit and real human-confirm end-to-end tests are actually executed. Specific Native command physical ownership and untrusted page isolation remain separate release criteria.
