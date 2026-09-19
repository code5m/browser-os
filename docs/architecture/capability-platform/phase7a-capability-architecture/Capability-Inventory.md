# Capability Inventory（能力清单）

> Phase 7A — Capability Architecture Audit & Design
> 日期：2026-09-19　基线：`semantic-governance-v1` (`316130d`)
> **本阶段为审计与设计，不改业务代码。**
> 判定依据一律是**真实代码证据**（store / component / Rust module / bridge invoke），不预设、不臆造。

---

## 1. 分类标准

| 分类 | 含义 | 判定依据 |
|---|---|---|
| `CAPABILITY` | 有明确边界、可独立装配与编排的功能单元 | 有独立语义 Owner 或独立 Rust 模块，具备可分离的生命周期 |
| `SUB_CAPABILITY` | 依附父能力，不能独立存在 | 组件/工具/状态依附于父能力，无独立生命周期 |
| `UI_COMPONENT` | 纯展示组件，不持状态真源 | `.vue` 不写 store 状态 |
| `SERVICE` | 无状态工具 / 转换 / 同步逻辑 | `src/utils/*.ts` 纯函数或辅助模块 |
| `ADAPTER` | 前端 ↔ Native / 外部系统适配层 | `bridge.ts`、`composables/` |
| `INFRASTRUCTURE` | 应用骨架，所有能力共享，**不可卸载** | shell / 持久化 / 安全策略 / 会话 |
| `IMPLEMENTATION_DETAIL` | 内部实现细节，不是能力 | 派生值、内部缓存、临时纪元 |

> 铁律：**分类不是荣誉，是约束。** 标 `CAPABILITY` 就必须能在后续阶段给出 manifest、lifecycle 与资源策略；给不出就降级。

---

## 2. 清单（证据驱动）

| # | 候选 | 分类 | 真实证据 | 语义 Owner（来自 Semantic Registry） | 资源初判 |
|---|---|---|---|---|---|
| 1 | Browser | CAPABILITY | `useBrowserStore` / `components/browser/` / `grid_process.rs`+`grid_ipc.rs`（native webview） | `useBrowserStore` | HEAVY + WEBVIEW |
| 2 | Grid | CAPABILITY | `gridSession` 状态、`grid_process.rs`（多 webview）、`grid_ipc.rs`、`useGridArchiveStore` | `useBrowserStore` | VERY_HEAVY + MULTI_WEBVIEW |
| 3 | Workspace | CAPABILITY | `useWorkspaceStore` / `components/workspace/` | `useWorkspaceStore` | LIGHT |
| 4 | Files | SUB_CAPABILITY(Workspace) | `FilePanel.vue`/`FileEditor.vue`/`FileTreeNode.vue`、`fs_cmds.rs` | `useWorkspaceStore` | LIGHT |
| 5 | File Preview | SUB_CAPABILITY(Files) | `useImagePreviewStore`、`imagePreview.ts`、`images.rs` | `useImagePreviewStore` | MEDIUM |
| 6 | Terminal | CAPABILITY | `useSystemStore.termPanes`、`terminal.rs`（PTY）、`TerminalPane.vue` | `useSystemStore` | PROCESS + PTY |
| 7 | Bookmark | CAPABILITY | `useBookmarkStore`、`components/home/` | `useBookmarkStore` | LIGHT |
| 8 | Credential | CAPABILITY | Rust keyring、`security_policy.rs` | `KeyringStore`（Rust） | SECURITY_SENSITIVE |
| 9 | Database | CAPABILITY | `useDatabaseStore`、`database.rs`、`DatabasePanel.vue` | `useDatabaseStore` | NETWORK + SECRET |
| 10 | Git | CAPABILITY | `useGitStore`、`GitPanel/DiffViewer/History.vue` | `useGitStore` | MEDIUM |
| 11 | Agent | CAPABILITY | `useAgentStore`、`agent.rs`、`agent_memory.rs`、`AgentChatPanel.vue` | `useAgentStore` | MEDIUM + NETWORK |
| 12 | Skill | CAPABILITY | `skills.rs`、`SkillManagerPanel.vue` | （未登记 Owner） | LIGHT |
| 13 | Plugin | CAPABILITY | `usePluginStore`、`plugin.rs`、`components/plugin/`；**runtime 仍 LOCKED** | `usePluginStore` | HEAVY + NATIVE |
| 14 | Knowledge Graph | CAPABILITY | `useGraphStore`、`graph.rs`、`components/graph/` | `useGraphStore` | MEDIUM |
| 15 | Notes | CAPABILITY | bridge `save_note` / `collect_selection`、`vault.mjs` | （未登记 Owner） | LIGHT |
| 16 | Vault | SUB_CAPABILITY(Notes) | bridge `vault_open`、`useVaultStore`、`VaultPanel.vue` | `useVaultStore` | LIGHT |
| 17 | Resource Collection | CAPABILITY | `useResourceStore`、`get/set_resource_capture_settings`、`check-resource-capture-policy.py` | `useResourceStore` | MEDIUM |
| 18 | Task | CAPABILITY | `useTaskStore`、`tasks.rs`、`scheduler.rs`、`TaskPanel.vue` | `useTaskStore` | BACKGROUND |
| 19 | Session | CAPABILITY | `useSessionStore`、`session.rs`、`shutdown.rs` | `useSessionStore` | LIGHT |
| 20 | Script | CAPABILITY | `scripts.rs`、`script_runner.rs`、`ScriptPanel.vue` | （未登记 Owner） | PROCESS |
| 21 | Snippet | SUB_CAPABILITY(Script) | `snippets.rs`、`CommandSnippetPanel.vue` | （未登记 Owner） | LIGHT |
| 22 | Workbench | CAPABILITY | `useWorkbenchStore`、`workbench.rs`、`workbench_smoke.rs` | `useWorkbenchStore` | MEDIUM |
| 23 | Home | UI_COMPONENT | `useHomeStore`、`homeUi.ts`、`components/home/`；依赖 Bookmark | （派生/展示） | LIGHT |
| 24 | Clipboard | SUB_CAPABILITY(System) | `ClipboardPanel.vue`；**与 Terminal 共享 `useSystemStore`** | `useSystemStore` | LIGHT |
| 25 | Settings | INFRASTRUCTURE | `useSettingsStore`、`SettingsPanel.vue` | — | LIGHT |
| 26 | Tools | SERVICE | `tools.rs`、`ToolBox.vue` | — | LIGHT |
| 27 | MCP | ADAPTER | `mcp.rs`、`mcp_server.rs` | — | NETWORK |
| 28 | Sync | INFRASTRUCTURE | `sync.rs`、`browserSync.ts` | — | LIGHT |
| 29 | Layout / Shell | INFRASTRUCTURE | `useLayoutStore`、`components/layout/`、`App.vue` | `useLayoutStore` | LIGHT |
| 30 | `gridSession` 内部字段 | IMPLEMENTATION_DETAIL | Phase 6A 已裁决：内存缓存失效纪元，**非存储真源** | `useBrowserStore` | — |

---

## 3. 审计中发现的事实（不是目标，是现状）

1. **`useSystemStore` 同时是 Terminal 与 Clipboard 的 Owner** —— 这是真实耦合：Terminal（PROCESS/PTY）与剪贴板（LIGHT）被同一个 store 绑定，导致"关闭终端"与"保留剪贴板历史"无法独立编排。**这是 7E 必须面对的问题，不是今晚能解决的目标态。**
2. **Plugin runtime 仍 LOCKED**（既有冻结决定）：`plugin_invoke`、动态加载、网络监听均未开放。Plugin 在今晚只能登记为 **COMPATIBILITY_WRAPPED**，不能声明为可装配。
3. **没有任何能力当前能真正独立启停**：所有 store 在应用启动时经 Pinia 统一实例化，`App.vue` 静态引用 8 个 store（Browser / Git / Layout / Resource / Session / Settings / System / Workspace）。今晚可达成的是 **COMPATIBILITY_WRAPPED**，不是物理卸载。
4. **Skill / Notes / Script 三个候选在 Semantic Registry 中尚无登记 Owner** —— 它们若升级为 Capability，必须先走 SCR 登记 Owner，否则 Runtime 会成为绕过语义治理的后门（见 §Shell-Boundary）。

---

## 4. 统计

```text
CAPABILITY            18   Browser, Grid, Workspace, Terminal, Bookmark, Credential,
                           Database, Git, Agent, Skill, Plugin, KnowledgeGraph,
                           Notes, ResourceCollection, Task, Session, Script, Workbench
SUB_CAPABILITY         5   Files, FilePreview, Vault, Snippet, Clipboard
UI_COMPONENT           1   Home
SERVICE                2   Tools, Images
ADAPTER                1   MCP（另：bridge.ts 属 INFRA 通道）
INFRASTRUCTURE         3   Settings, Sync, Layout/Shell
IMPLEMENTATION_DETAIL  1   gridSession 内部
─────────────────────────
合计                  31
```
> 修正记录：初稿 §4 误写 CAPABILITY=15，实为 18（以 §2 表格逐行数为准）。此处已更正。
> 精确计数以 §2 表格为准；Images 归 SERVICE、Home 归 UI_COMPONENT。
