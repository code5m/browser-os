# Native 58-command caller-scope audit — 2026-11-02

This is a **static source and Tauri capability gate** at branch `fix/capability-runtime-agent-chat-20261102`, **not** user-confirmation or installed-GUI proof.

All 58 previously unverified permission declarations were located in the installed main-window `default-commands` allowlist; 56 have no remote grant, while `report_resources` and `report_title` are intentionally exposed through the restricted `remote-collect` telemetry capability. `default-commands` is granted only to the `main` window; this does **not** establish permission granularity for every dangerous effect.

Command-body source guard directly observed on **27/58**; the remaining commands have at least Tauri ACL containment but still require manual per-Owner side-effect/approval review. The permission debt baseline remains open deliberately, rather than silently relabeling the 58 as cleared.

| Command | Actual source | Tauri caller scope | Native command-body guard | Effect category (heuristic, not approval) |
|---|---|---|---|---|
| `add_bookmark` | `src-tauri/src/bridge.rs:2037` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `clear_tab_resources` | `src-tauri/src/capabilities/browser/commands.rs:217` | main only (Tauri ACL) | local invocation check observed | state or OS side effect; inspect Owner approval |
| `clipboard_read` | `src-tauri/src/shared/clipboard.rs:10` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `clipboard_write` | `src-tauri/src/shared/clipboard.rs:17` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `close_browser` | `src-tauri/src/bridge.rs:419` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `close_tools` | `src-tauri/src/capabilities/tools/tools.rs:102` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `debug_log` | `src-tauri/src/framework/commands.rs:14` | main only (Tauri ACL) | Tauri ACL only in this function | read-only/telemetry |
| `eval_in_tab` | `src-tauri/src/bridge.rs:3752` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `flush_sessions` | `src-tauri/src/capabilities/session/commands.rs:197` | main only (Tauri ACL) | local invocation check observed | state or OS side effect; inspect Owner approval |
| `get_default_browser` | `src-tauri/src/bridge.rs:4090` | main only (Tauri ACL) | Tauri ACL only in this function | read-only/telemetry |
| `get_resource_capture_settings` | `src-tauri/src/capabilities/browser/commands.rs:240` | main only (Tauri ACL) | local invocation check observed | read-only/telemetry |
| `get_session_policy` | `src-tauri/src/capabilities/session/commands.rs:248` | main only (Tauri ACL) | local invocation check observed | read-only/telemetry |
| `graph_node_get` | `src-tauri/src/capabilities/graph/commands.rs:308` | main only (Tauri ACL) | local invocation check observed | read-only/telemetry |
| `graph_query` | `src-tauri/src/capabilities/graph/commands.rs:289` | main only (Tauri ACL) | local invocation check observed | read-only/telemetry |
| `graph_stats` | `src-tauri/src/capabilities/graph/commands.rs:325` | main only (Tauri ACL) | local invocation check observed | read-only/telemetry |
| `hide_all_webviews` | `src-tauri/src/bridge.rs:556` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `hide_webview` | `src-tauri/src/bridge.rs:529` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `issue_intent` | `src-tauri/src/bridge.rs:1224` | main only (Tauri ACL) | local invocation check observed | state or OS side effect; inspect Owner approval |
| `list_bookmarks` | `src-tauri/src/bridge.rs:2050` | main only (Tauri ACL) | Tauri ACL only in this function | read-only/telemetry |
| `list_tab_resources` | `src-tauri/src/capabilities/browser/commands.rs:198` | main only (Tauri ACL) | local invocation check observed | read-only/telemetry |
| `list_tools` | `src-tauri/src/capabilities/tools/tools.rs:96` | main only (Tauri ACL) | Tauri ACL only in this function | read-only/telemetry |
| `m0_ready` | `src-tauri/src/bridge.rs:4133` | main only (Tauri ACL) | Tauri ACL only in this function | read-only/telemetry |
| `open_browser` | `src-tauri/src/bridge.rs:412` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `open_tool` | `src-tauri/src/capabilities/tools/tools.rs:127` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `position_browser` | `src-tauri/src/bridge.rs:429` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `remove_bookmark` | `src-tauri/src/bridge.rs:2058` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `report_resources` | `src-tauri/src/capabilities/browser/commands.rs:284` | main + restricted remote telemetry | local invocation check observed | read-only/telemetry |
| `report_title` | `src-tauri/src/bridge.rs:1100` | main + restricted remote telemetry | local invocation check observed | read-only/telemetry |
| `resource_stats` | `src-tauri/src/bridge.rs:3315` | main only (Tauri ACL) | Tauri ACL only in this function | read-only/telemetry |
| `session_delete` | `src-tauri/src/capabilities/session/commands.rs:141` | main only (Tauri ACL) | local invocation check observed | state or OS side effect; inspect Owner approval |
| `session_discard` | `src-tauri/src/capabilities/session/commands.rs:93` | main only (Tauri ACL) | local invocation check observed | state or OS side effect; inspect Owner approval |
| `session_export` | `src-tauri/src/capabilities/session/commands.rs:152` | main only (Tauri ACL) | local invocation check observed | state or OS side effect; inspect Owner approval |
| `session_get` | `src-tauri/src/capabilities/session/commands.rs:129` | main only (Tauri ACL) | local invocation check observed | state or OS side effect; inspect Owner approval |
| `session_list` | `src-tauri/src/capabilities/session/commands.rs:117` | main only (Tauri ACL) | local invocation check observed | state or OS side effect; inspect Owner approval |
| `session_restore` | `src-tauri/src/capabilities/session/commands.rs:173` | main only (Tauri ACL) | local invocation check observed | state or OS side effect; inspect Owner approval |
| `session_save` | `src-tauri/src/capabilities/session/commands.rs:65` | main only (Tauri ACL) | local invocation check observed | state or OS side effect; inspect Owner approval |
| `set_default_browser` | `src-tauri/src/bridge.rs:4108` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `set_resource_capture_settings` | `src-tauri/src/capabilities/browser/commands.rs:256` | main only (Tauri ACL) | local invocation check observed | state or OS side effect; inspect Owner approval |
| `set_session_policy` | `src-tauri/src/capabilities/session/commands.rs:262` | main only (Tauri ACL) | local invocation check observed | state or OS side effect; inspect Owner approval |
| `set_tab_hibernation` | `src-tauri/src/bridge.rs:3469` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `skill_parse` | `src-tauri/src/capabilities/skill/commands.rs:44` | main only (Tauri ACL) | local invocation check observed | read-only/telemetry |
| `skill_permission_preview` | `src-tauri/src/capabilities/skill/commands.rs:60` | main only (Tauri ACL) | local invocation check observed | read-only/telemetry |
| `skill_validate` | `src-tauri/src/capabilities/skill/commands.rs:50` | main only (Tauri ACL) | local invocation check observed | read-only/telemetry |
| `snippet_add` | `src-tauri/src/bridge.rs:1680` | main only (Tauri ACL) | local invocation check observed | state or OS side effect; inspect Owner approval |
| `snippet_list` | `src-tauri/src/bridge.rs:1668` | main only (Tauri ACL) | local invocation check observed | state or OS side effect; inspect Owner approval |
| `snippet_remove` | `src-tauri/src/bridge.rs:1801` | main only (Tauri ACL) | local invocation check observed | state or OS side effect; inspect Owner approval |
| `snippet_update` | `src-tauri/src/bridge.rs:1740` | main only (Tauri ACL) | local invocation check observed | state or OS side effect; inspect Owner approval |
| `tab_activate` | `src-tauri/src/bridge.rs:3766` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `tab_close` | `src-tauri/src/bridge.rs:3061` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `tab_go_back` | `src-tauri/src/bridge.rs:3733` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `tab_go_forward` | `src-tauri/src/bridge.rs:3739` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `tab_list` | `src-tauri/src/bridge.rs:3715` | main only (Tauri ACL) | Tauri ACL only in this function | read-only/telemetry |
| `tab_new` | `src-tauri/src/bridge.rs:3054` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `tab_open` | `src-tauri/src/bridge.rs:3067` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `tab_position` | `src-tauri/src/bridge.rs:3085` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `tab_reload` | `src-tauri/src/bridge.rs:3745` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `tab_set_title` | `src-tauri/src/bridge.rs:3723` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |
| `take_pending_open_urls` | `src-tauri/src/bridge.rs:4035` | main only (Tauri ACL) | Tauri ACL only in this function | state or OS side effect; inspect Owner approval |

**Unresolved:** determine allowed capability principal, user confirmation (only when needed), sensitive data visibility, exact side effects and error rollback per command. Test remote denial using an actual tab/Grid WebView before removing a legacy debt entry. `check-native-owner-effect-audit.mjs` now fails if these capabilities change and expose any of the other 56 commands remotely.
