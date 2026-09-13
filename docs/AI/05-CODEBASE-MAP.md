# 05 Codebase Map

Status: Phase 02 expanded (verified against source on 2026-09-10).

This document maps important project areas for future agents. Every claim below has been verified against the source tree on 2026-09-10. Labels follow `PROJECT-RULES.md`: `CURRENT`, `APPROVED_TARGET`, `PENDING`, `DEPRECATED`.

---

## 1. Top-Level Areas

| Path | Description | Status |
|---|---|---|
| `src/` | Vue 3 + TypeScript application shell | `CURRENT` |
| `src/components/` | UI components (61 `.vue` files across 8 domains) | `CURRENT` |
| `src/stores/` | Pinia state stores (19) | `CURRENT` |
| `src/composables/` | Composables (3): `useBrowserHost`, `useModalFocus`, `useTerminalResize` | `CURRENT` |
| `src/utils/` | Utility modules (16) | `CURRENT` |
| `src/styles/` | `global.css` | `CURRENT` |
| `src/bridge.ts` | Unified IPC bridge (~747 lines, wraps all `invoke`) | `CURRENT` (BrowserRuntime precursor) |
| `src/types.ts` | Centralized TypeScript type definitions | `CURRENT` |
| `src/domain/` | Domain layer | `APPROVED_TARGET` (directory does not exist) |
| `src/application/` | Application layer | `APPROVED_TARGET` (directory does not exist) |
| `src-tauri/` | Tauri 2 + Rust native backend | `CURRENT` |
| `src-tauri/src/main.rs` | App entry (~1565 lines), plugin registration, `generate_handler!` | `CURRENT` |
| `src-tauri/src/lib.rs` | `mvp_core` library boundary | `CURRENT` |
| `src-tauri/src/bridge.rs` | Rust command surface (~7170 lines, ~140 commands) | `CURRENT` |
| `src-tauri/src/core/` | Core seam + keyring store | `CURRENT` |
| `src-tauri/permissions/` | `default-commands.toml`, `remote-collect.toml` | `CURRENT` |
| `src-tauri/capabilities/` | `default.json`, `browser-remote.json` | `CURRENT` |
| `src-tauri/dev-capabilities/` | `main.json` (debug-only injection) | `CURRENT` |
| `tauri-browser-tabs/` | Custom native browser-tabs plugin | `CURRENT` |
| `scripts/` | Validation / diagnostic scripts (17+ `check-*.mjs`, `pre-merge.sh`) | `CURRENT` |
| `docs/AI/` | AI architecture + design docs (`00`, `01`, `05`) | `CURRENT` |
| `.ai/` | Agent roles, registry, shared context entry | `CURRENT` |

---

## 2. Component Map (`src/components/`)

`CURRENT` — 8 domain subdirectories:

| Subdir | Count | Key files |
|---|---|---|
| `browser/` | 9 | `BrowserHost.vue`, `SessionCloseDialog.vue`, `AINavPanel.vue`, `BookmarkPanel.vue`, `ResourcePanel.vue` |
| `workspace/` | 25 | `FilePanel.vue`, `GitPanel.vue`, `AgentChatPanel.vue`, `TaskPanel.vue`, `ScriptPanel.vue`, `DatabasePanel.vue` |
| `layout/` | 9 | `UnifiedTabBar.vue`, `TopBar.vue`, `StatusBar.vue`, `Sidebar.vue`, `MainArea.vue`, `ActivityBar.vue` |
| `graph/` | 5 | `GraphPanel.vue`, `GraphViewer.vue`, `NodeDetail.vue`, `EdgeDetail.vue` |
| `home/` | 5 | `HomePanel.vue`, `HomeLaunchers.vue`, `HomeShortcuts.vue` |
| `system/` | 4 | `SettingsPanel.vue`, `TerminalPane.vue`, `ClipboardPanel.vue`, `AppPanel.vue` |
| `shared/` | 3 | `ConfirmModal.vue`, `ImageGallery.vue`, `ImageLightbox.vue` |
| `plugin/` | 1 | `PluginManager.vue` |

---

## 3. Store Map (`src/stores/`)

`CURRENT` — 19 Pinia stores:

`useAgentStore`, `useBookmarkStore`, `useBrowserStore`, `useDatabaseStore`, `useGitStore`, `useGraphStore`, `useGridArchiveStore`, `useHomeStore`, `useImagePreviewStore`, `useLayoutStore`, `usePluginStore`, `useResourceStore`, `useSessionStore`, `useSettingsStore`, `useSystemStore`, `useTaskStore`, `useVaultStore`, `useWorkbenchStore`, `useWorkspaceStore`.

---

## 4. Rust Backend Map (`src-tauri/src/`)

`CURRENT` — core files and modules:

| File / module | Role |
|---|---|
| `main.rs` | Entry, plugin registration, grid child process, `generate_handler!` (~140 commands) |
| `lib.rs` | `mvp_core` library boundary |
| `bridge.rs` | All `#[tauri::command]` (~140): browser, grid, session, git, script, file, db, terminal, graph, plugin, agent/skill, mcp, artifact, bookmark, vault |
| `core/mod.rs`, `core/seam.rs`, `core/keyring_store.rs` | Core seam + secrets |
| `agent.rs`, `agent_memory.rs` | Agent parsing / validation / memory |
| `session.rs` | Session save / restore / policy |
| `graph.rs` | Knowledge graph queries |
| `grid_ipc.rs`, `grid_process.rs` | Grid child process IPC |
| `database.rs`, `terminal.rs`, `git2` via `git_*` | DB / terminal / git |
| `security_policy.rs` | Permission policy |
| `shutdown.rs`, `crashlog.rs` | Shutdown + crash logging |
| `tools/*.html` | Built-in tools (base64, cron, json, regex, timestamp) |

---

## 5. Native Plugin Map (`tauri-browser-tabs/`)

`CURRENT`

```
tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/
├── lib.rs              plugin init
├── models.rs           LogicalRect, BrowserTabError, Result
├── commands.rs         TabManagerState (create_tab, update_rect, set_visible,
│                       set_zoom, close_tab, navigate, get_tab_ids, eval, eval_result)
│                       + 7 #[tauri::command] (create_tab, update_rect, set_visible,
│                       close_tab, navigate, list_tabs, set_zoom)
└── platform/
    ├── mod.rs
    ├── linux.rs        ensure_size_allocated (gtk_fixed_move + size_allocate +
    │                   queue_draw, never queue_resize), ensure_physical_size,
    │                   forget_size_allocation
    ├── macos.rs
    └── windows.rs
```

Permissions: `permissions/default.toml` + `permissions/autogenerated/commands/*.toml`.

JS bindings: `packages/core/src/` (geometry, manager, types), `packages/vue/src/` (`BrowserHost.vue`, `useBrowserTabs.ts`).

---

## 6. Native Danger Zones

Per `AGENTS.md` §4 — require explicit task authorization to modify:

- `src-tauri/src/bridge.rs`
- `src-tauri/src/**/linux.rs`
- `src-tauri/capabilities/**`
- `tauri-browser-tabs/**`

These cover GTK/WebKitGTK behavior, native window layering, logical/physical pixels, webview show/hide/destroy ordering, Wayland/X11, and Tauri permissions.

---

## 7. Validation Entry Points

### 7.1 `CURRENT` (exist in source)

| Command | Scope |
|---|---|
| `npm run build` | Vite frontend build |
| `bash scripts/pre-merge.sh` | Aggregated pre-merge gate |
| `git diff --check` | Whitespace / conflict markers |
| `cargo check --manifest-path src-tauri/Cargo.toml` | Rust compilation |
| `scripts/check-core-boundary.py` | `mvp_core` library boundary R-B1..R-B5 |
| `scripts/check-native-webview-overlay.mjs` | No HTML overlay over native webview |
| `scripts/check-window-drag.mjs` | Window drag event + permission contract |
| `scripts/check-*-logic.mjs` / `check-*-ui-logic.mjs` (17+) | Domain-specific logic checkers |

### 7.2 `APPROVED_TARGET` (planned in `.ai/registry.md`, not yet in source)

- `scripts/check-architecture.mjs`
- `scripts/check-ui.mjs`
- `scripts/check-native.mjs`
- `scripts/check-browser-runtime.mjs`
- `scripts/doctor.mjs`
- `scripts/check-task-boundary.mjs`

---

## 8. Key Source Facts (verified 2026-09-10)

| Claim | Verification |
|---|---|
| Components do not directly call `invoke` | grep `invoke` in `src/components/` → 4 hits, all comments; 0 actual calls |
| `BrowserRuntime` / `MockRuntime` / `BrowserScene` not in source | grep in `src/` → 0 hits each |
| `syncScene` not in source | grep in `src/` → 0 hits |
| `WebViewSafeShell` / `BrowserViewportAnchor` not in source | grep in `src/` → 0 hits |
| `src/domain/` and `src/application/` do not exist | directory listing confirms absence |
| `bridge.ts` is the unified invoke wrapper | 747 lines, imports `invoke` from `@tauri-apps/api/core`, exports `bridge` object |
| Child webview fix uses `gtk_fixed_move` + `size_allocate` | `platform/linux.rs:77-83` |
| Hidden tabs moved to x=-30000, size unchanged | `PROJECT-RULES.md` rule 3.5; `bridge.rs` `hide_bounds` |
| `mvp_core` boundary enforced by script | `scripts/check-core-boundary.py` exists |

---

## 9. Map Gaps

- `BrowserRuntime` / `MockRuntime` / `BrowserScene` / `syncScene` / `WebViewSafeShell` / `BrowserViewportAnchor` are `APPROVED_TARGET`; not yet locatable in source.
- `src/domain/` and `src/application/` directories are `APPROVED_TARGET`; absent today.
- Planned checker scripts (§7.2) are not yet created.
- Individual `bridge.rs` command-to-module ownership (which of the ~140 commands belongs to which Rust module) is not yet enumerated here.
- `src/utils/` (16 modules) contents are not yet individually mapped.
