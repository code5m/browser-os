# Full-Stack Capability Inventory — mvp-browser-os-v3

> STAGE B 产物（Capability Library Expansion v1）。事实来自 `src/`、`src-tauri/src/`、`docs/architecture/capability-registry/capabilities.yaml`
> 与代码核验（code-explorer 子代理 + 人工复核）。最后更新：2026-09-23。
>
> 成熟度口径（诚实，不谎报）：
> - C0 REGISTERED：仅登记，无工作运行时
> - C1 WRAPPED：接入 Runtime / 有 store + 后端命令，但不可组合启停
> - C2 ISOLATED：实现经明确边界隔离（public/manifest/ui）
> - C3 OPTIONAL：absent → Shell 成立 + contribution 缺席 + 实现不激活 + 无隐藏依赖（需证据）
> - C4 RUNTIME_CONTROLLABLE：运行时 enable/disable
> - C5 RESOURCE_RELEASABLE：activate→资源存在，deactivate/destroy→资源释放（需证据）
>
> `profiles.yaml` 的 `composability` 列是**演示口径保守声明**（"今晚真正接入 Runtime 的只有 bookmark"），
> 与实际物理能力目录/门禁 tag 不完全一致——本表以**物理代码 + 门禁证据**为成熟度主依据，并标注 `profiles.yaml` 偏差。

## 汇总

| Capability | 物理位置 | 状态(status) | 成熟度 | 资源 | 组合态(profiles) |
|---|---|---|---|---|---|
| browser | `src/capabilities/browser` | COMPATIBILITY_WRAPPED | C3 OPTIONAL* | WebView(重) | TARGET_COMPOSABLE |
| grid | `src/components/browser` | NOT_INTEGRATED | C1 | WebView(极重) | TARGET_COMPOSABLE |
| workspace | `src/capabilities/workspace` | COMPATIBILITY_WRAPPED | C3 OPTIONAL* | LIGHT/MEDIUM | TARGET_COMPOSABLE |
| terminal | `src/capabilities/terminal` | COMPATIBILITY_WRAPPED | C3 OPTIONAL* | PTY/PROCESS | TARGET_COMPOSABLE |
| bookmark | `src/capabilities/bookmark` | NOT_INTEGRATED | C2 (C3 PENDING)** | LIGHT | COMPATIBILITY_WRAPPED |
| git | `src/capabilities/git` | COMPATIBILITY_WRAPPED | C2 ISOLATED | MEDIUM/NETWORK(瞬态) | COMPATIBILITY_WRAPPED |
| database | `src/components/workspace/DatabasePanel.vue` | NOT_INTEGRATED | C1 | NETWORK+SECRET(重) | TARGET_COMPOSABLE |
| agent | `src/components/workspace/AgentChatPanel.vue` | NOT_INTEGRATED | C0 (运行时未实现) | MEDIUM/NETWORK | TARGET_COMPOSABLE |
| skill | `src/components/workspace/SkillManagerPanel.vue` | NOT_INTEGRATED | C0 (OWNER_PENDING) | LIGHT | TARGET_COMPOSABLE |
| plugin | `src/components/plugin` | NOT_INTEGRATED | C1 (LOCKED 无执行) | HEAVY/NATIVE | NOT_COMPOSABLE_BY_DESIGN |
| knowledge_graph | `src/components/graph` | NOT_INTEGRATED | C1 (live query) | MEDIUM | TARGET_COMPOSABLE |
| task | `src/stores/useTaskStore.ts` | NOT_INTEGRATED | C1 (live) | BACKGROUND/PROCESS | TARGET_COMPOSABLE |
| session | `src/stores/useSessionStore.ts` | NOT_INTEGRATED | C1 (live) | LIGHT | NOT_COMPOSABLE_BY_DESIGN |
| script | `src/capabilities/workspace/state` | NOT_INTEGRATED | C1 (live, spawns) | PROCESS | TARGET_COMPOSABLE |
| notes | `src/stores/useVaultStore.ts` | NOT_INTEGRATED | C1 | LIGHT | TARGET_COMPOSABLE |
| resource_collection | `src/stores/useResourceStore.ts` | NOT_INTEGRATED | C1 (live, 后台捕获) | MEDIUM/BACKGROUND | TARGET_COMPOSABLE |
| credential | `src-tauri/src/core/keyring_store.rs` | NOT_INTEGRATED | C1 (安全边界) | SECURITY_SENSITIVE | NOT_COMPOSABLE_BY_DESIGN |
| workbench | `src/stores/useWorkbenchStore.ts` | NOT_INTEGRATED | 框架/Shell（非后端能力） | — | TARGET_COMPOSABLE |

`*` browser/workspace/terminal 有 `capabilities/*` 物理目录 + manifest + public，且存在 `capability-phase8d-browser-composable-code-pass` / `capability-phase8e-terminal-composable-code-pass` / `capability-phase8c-workspace-composable-code-pass` tag，证明 C3 机制已达成；但 `profiles.yaml` 仍保守声明 TARGET_COMPOSABLE（演示口径）。本表据物理证据记 C3。
`**` bookmark：HANDOFF 已更正（2026-09-20）——官方仍 C2，C3 = PENDING（8B.1 机制在 WIP 但当时未提交/未打 tag）。

**CAPABILITY_INVENTORY_UNKNOWN = 0**（所有疑问均已解析或明确记为已知债务，无悬空 UNKNOWN）。

---

## 已成型能力（physical `capabilities/*` dir）

### browser — C3 OPTIONAL
- 用户目的：网页浏览主表面 + 网格(Grid) + 地址栏 + 导航。
- UI owner：`src/capabilities/browser/ui/*`、`src/components/browser/*`。
- State owner：`useBrowserStore`（`src/capabilities/browser/state/useBrowserStore.ts`，id=browser）—— 经 Semantic Registry GOVERNED（gridOpen 唯一 owner）。
- Intent owner：`useBrowserStore`（navigate/tab.open/tab.close/activateGrid）。
- App logic：`useBrowserStore` + `src/capability` runtime。
- Native owner：`bridge.browserNavigate/...` → Rust `bridge.rs` 浏览器命令。
- Resource owner：WebView（createGrid 经 `isBrowserResourceAllowed()` 准入闸门；见 grid）。
- Credential：无。
- Persistence：session。
- Dependencies：`bridge`。
- Contribution：浏览器主面 + Grid 子窗。
- Legacy coupling：`gridOpen` 派生 `desiredGridVisibility=gridOpen && mainView==='grid'`（Semantic Governance 冻结，禁止重存）。
- Absence：profile 不含 browser → 不注册 → 主浏览器资源不创建（C3 机制由 phase8d tag 证明）。

### grid — C1（极重 WebView 资源）
- 用户目的：多网页并排网格视图。
- UI owner：`src/capabilities/browser/ui/*`、`src/components/browser/*`。
- State owner：`useBrowserStore`（`gridOpen`/`gridSession`）。
- Intent owner：`useBrowserStore.buildGrid/closeGrid*`。
- App logic：`useBrowserStore` + `createGrid` 单一调用点（gate: `isBrowserResourceAllowed()`）。
- Native owner：`bridge.createGrid/closeGrid/gridOpen/...` → Rust `create_grid`/`close_grid`/`grid_open`（bridge.rs:3851/3915/3949）。
- Resource owner：多子 WebView 窗口（`grid-child-N` 独立 Tauri 二进制，always_on_top + UDS IPC + layout_enforcer）。VERY_HEAVY。
- Credential：无。
- Persistence：disk（archive）。
- Dependencies：`browser`,`bridge`。
- Contribution：Grid 子窗表面。
- Legacy coupling：`gridOpen` 由 Semantic Governance 冻结（HIDE ONLY / explicit closeGrid = DESTROY）。
- 评级：C1（registered + store + 命令 + 资源闸门），但 `resident=true, activatable=false` → 非可组合；目标 C3/C5（资源释放需证据）。

### workspace — C3 OPTIONAL
- 用户目的：文件树 / 仓库(RepoPanel) / 脚本 / 片段 / 产物。
- UI owner：`src/capabilities/workspace/ui/*`、`src/components/workspace/*`。
- State owner：`useRepoStore`（`src/capabilities/workspace/state/useRepoStore.ts`）等（Phase 8C 拆 owner）。
- Intent owner：各 store。
- Native owner：`bridge.fs*` / 脚本运行等。
- Resource owner：LIGHT/MEDIUM（fs）。
- Credential：`configureRepo` 经后端 keyring 存 repo token（DEV-03 固化）。
- Dependencies：`browser`,`bridge`。
- Contribution：RepoPanel（Git 经 REPO_SUBVIEW 槽贡献进此）。
- Legacy coupling：DatabasePanel/ScriptPanel 仍 always-loaded（DEV-06 诚实标注）。

### terminal — C3 OPTIONAL
- 用户目的：终端 PTY。
- UI owner：`src/capabilities/terminal/ui/*`。
- State owner：`useTerminalStore`（`src/capabilities/terminal/state`，Phase 8E 从 useSystemStore 迁出专属 owner）。
- Intent owner：`useTerminalStore`（spawn/write/kill/resize）。
- Native owner：`bridge.term*` → Rust PTY 命令（term_spawn_channel 等）。
- Resource owner：PTY/child process（destroyable=true）。
- Credential：无。
- Dependencies：`bridge`。
- Contribution：Dock 终端页签。
- 评级：C3（phase8e-terminal-composable-code-pass tag）。

### bookmark — C2（C3 PENDING）
- 用户目的：书签列表/星标/面板。
- UI owner：`src/capabilities/bookmark/ui/*`。
- State owner：`useBookmarkStore`（`src/stores/useBookmarkStore.ts`）。
- Intent owner：`useBookmarkStore`（list/add/remove/togglePanel）。
- Native owner：`bridge.bookmark*`（可选 bridge）。
- Resource owner：LIGHT。
- Dependencies：`browser`。
- Contribution：书签星标 / 面板。
- 评级：C2 官方（HANDOFF 更正：C3 PENDING，8B.1 机制未正式提交+打 tag）。

### git — C2 ISOLATED（STAGE A 已收口）
- 详见 `docs/architecture/capabilities/git/FULL-STACK-BOUNDARY.md`。
- State owner：`useGitStore`（语义 owner；物理仍在 `src/stores/`，债务）。
- Resource：瞬态 git 子进程 + 5 分钟写任务门，无长驻重资源。
- Credential：前端零；后端推送瞬间读 keyring。
- Dependencies：`credential`,`workspace`,`bridge`。
- 评级：C2。App.vue 静态直连写闸门（债务1）阻断 C3。

---

## 待迁移能力（TARGET_COMPOSABLE / NOT_INTEGRATED）

### database — C1（重资源，目标 C3/C5）
- 用户目的：连接数据库、执行查询、取消查询。
- UI owner：`src/components/workspace/DatabasePanel.vue`（always-loaded Workspace 侧栏）。
- State owner：`useDatabaseStore`（`src/stores/useDatabaseStore.ts`，id=database）。
- Intent owner：`useDatabaseStore.connect/disconnect/requestRun/confirmRun/cancelQuery`。
- App logic：两阶段运行闸门（`runGate`/`requiresConfirm`）。
- Native owner：`bridge.dbConnect/dbQuery/dbDisconnect/dbListConnections/dbCancel` → Rust `db_connect`(bridge.rs:6308)/`db_query`(6354)/`db_disconnect`(6435)/`db_cancel`(6134)/`db_list_connections`(6116)；注册 main.rs:1527-1536。
- Resource owner：**真实网络 DB 连接（NETWORK+SECRET）**。关键：`db_connect` 探针连接后即弃（不全局驻留池），`db_query` 每次重新 `DbPool::connect`（bridge.rs:6330 注释）。
- Credential：YES —— `db_connect` 写密码到 OS keyring（`db:<conn_id>`），`db_query` 重读；前端永不留密码（DEVA-03 固化）。
- Persistence：disk + os_keyring。
- Dependencies：`credential`,`bridge`。
- Legacy coupling：always-loaded 面板（DEV-06）；达到 C3 需抽为可选能力包（Debt-8E-5）。
- 评级：C1。C3 需 absent→不挂载面板/不建连接；C5 需证明 deactivate→连接释放（当前每查询重连，无长驻池，资源治理较简单但仍需明确 absent 证据）。

### agent — C0（运行时未实现）
- 用户目的：Agent 对话 / 运行 / 记忆。
- UI owner：`src/components/workspace/AgentChatPanel.vue` + `AgentManagerPanel.vue`。
- State owner：`useAgentStore`（`src/stores/useAgentStore.ts`）。
- Intent owner：`useAgentStore`（chat/run/install/validate…）。
- Native owner：**仅只读命令实现**（`agent_parse`/`agent_validate`/`agent_permission_preview`，bridge.rs:6529/6539/6549）；`agent_chat`/`agent_list`/`agent_install` 等**Rust 中不存在**。`backendReady = AGENT_SKILL_COMMANDS_AVAILABLE = false`（bridge.ts:102）→ 前端 `guard()` 拦下，永不 invoke。
- Resource owner：无（运行时未接）。
- Credential：无前端 keyring；store 内 SECRET 脱敏。
- Dependencies：`bridge`，可选 `knowledge_graph`。
- 评级：C0（注册 + store + 只读后端，无工作运行时）。目标：先实现后端 agent_chat 再评 C2。

### skill — C0（OWNER_PENDING）
- 用户目的：技能列表 / 调用。
- UI owner：`src/components/workspace/SkillManagerPanel.vue`。
- State owner：**无专属 store**；技能状态寄居 `useAgentStore.skills`（registry `semanticOwner:null`/OWNER_PENDING_SCR 一致）。`loadSkills` 受 `backendReady=false` 限制 → 恒空。
- Intent owner：`useAgentStore`（委托）。
- Native owner：**仅只读**（`skill_parse`/`skill_validate`/`skill_permission_preview`）；`skill_list`/`skill_run`/`skill_install` **Rust 不存在**。
- Resource owner：无执行路径。
- Credential：无。
- Dependencies：`agent`,`bridge`。
- 评级：C0。需先定 semanticOwner（SCR）+ 实现 skill_list/run 后端。

### plugin — C1（LOCKED 无执行）
- 用户目的：插件清单 / manifest / 密钥。
- UI owner：`src/components/plugin/*`。
- State owner：`usePluginStore`（`src/stores/usePluginStore.ts`）。
- Intent owner：`usePluginStore`（list/get/install/enable/disable/keys*）。
- Native owner：`bridge.plugin*` → Rust `plugin_install/enable/disable/list/get/keys_*`(bridge.rs:7834-7992)；**无 plugin_run/invoke**。
- Resource owner：无执行（governanceStatus=LOCKED）。
- Credential：无 OS keyring；仅可信公钥指纹（16-hex，不暴露原始密钥）。
- Dependencies：`bridge`。
- 评级：C1（registered + governed + LOCKED）。目标：保持 LOCKED 或明确 runtime 边界后评 C2；**禁止 installed==enabled==active==resource 混淆**。

### knowledge_graph — C1（live query）
- 用户目的：图谱查询 / 节点获取 / 统计。
- UI owner：`src/components/graph/GraphPanel.vue`。
- State owner：`useGraphStore`（`src/stores/useGraphStore.ts`）。
- Intent owner：`useGraphStore`（loadGraph/loadNode/loadStats）。
- Native owner：`bridge.graphQuery/graphNodeGet/graphStats` → Rust `graph_query`(7710)/`graph_node_get`(7729)/`graph_stats`(7746)（graph.rs 实现）；`GRAPH_COMMANDS_AVAILABLE=true`。
- Resource owner：无（只读查询既有图存储）。
- Credential：无。
- Dependencies：`bridge`，可选 `agent`。
- 评级：C1（live）。目标 C3。

### task — C1（live）
- 用户目的：定时任务调度 / 运行 / 取消。
- UI owner：`src/components/workspace/TaskPanel.vue`。
- State owner：`useTaskStore`（`src/stores/useTaskStore.ts`，id=tasks）。
- Intent owner：`useTaskStore`（schedule/run/cancel/setEnabled）。
- Native owner：`bridge.task*` → Rust `task_list/add/update/remove/run_now`（bridge.rs:1004-1148）；`TASK_COMMANDS_AVAILABLE=true`。`task_run_now` 经 script_runner spawn 子进程。
- Resource owner：BACKGROUND/PROCESS（spawn）。
- Credential：无（secret 参数不持久化）。
- Dependencies：`bridge`，可选 `script`。
- 评级：C1（live）。目标 C3。

### session — C1（live）
- 用户目的：会话持久化 / 恢复。
- UI owner：`src/capabilities/browser/ui/SessionPanel.vue`。
- State owner：`useSessionStore`（`src/stores/useSessionStore.ts`）。
- Intent owner：`useSessionStore`（save/restore/delete/export/setPolicy）。
- Native owner：`bridge.session*` → Rust `session_save/.../set_session_policy`（bridge.rs:1686-1891）。
- Resource owner：LIGHT（磁盘快照；WebView 经 `session_restore`→`tab_new`）。
- Credential：无（URL 敏感参数脱敏后持久化）。
- Dependencies：`bridge`。
- 评级：C1（live）。分类：NOT_COMPOSABLE_BY_DESIGN（resident=true，安全/常驻）。

### script — C1（live, spawns）
- 用户目的：脚本 / 片段运行。
- UI owner：`src/capabilities/workspace/ui/ScriptPanel.vue` + `CommandSnippetPanel.vue`。
- State owner：`useScriptStore`/`useSnippetStore`（`src/capabilities/workspace/state/`）。
- Intent owner：两 store（run/cancel/list/...）。
- Native owner：`bridge.script*`/`snippet*`/`runScript`/`runCommand` → Rust `run_script`/`run_command`(bridge.rs:3343/3292) spawn 子进程。
- Resource owner：PROCESS（spawn）。
- Credential：无。
- Dependencies：`bridge`。
- 评级：C1（live）。目标 C2/C3。

### notes — C1
- 用户目的：笔记 / 收藏（vault）。
- UI owner：`src/components/workspace/VaultPanel.vue`。
- State owner：`useVaultStore`（`src/stores/useVaultStore.ts`，id=vault）—— registry `semanticOwner:null` 与实际 store 不符（已知偏差）。
- Intent owner：`useVaultStore`（open/select/follow）。
- Native owner：`bridge.saveNote/vaultOpen` → Rust `save_note`(2055)/`vault_open`(6150)。
- Resource owner：LIGHT（磁盘 markdown）。
- Credential：无。
- Dependencies：`bridge`。
- 评级：C1。需把 registry semanticOwner 从 null 改为 useVaultStore（SCR 对齐）。

### resource_collection — C1（live, 后台捕获）
- 用户目的：按 tab 资源（网络请求）采集 / 瀑布。
- UI owner：`src/components/browser/ResourcePanel.vue` + `ResourceWaterfall.vue`。
- State owner：`useResourceStore`（`src/stores/useResourceStore.ts`，id=resource）。
- Intent owner：`useResourceStore`（applyReceived/setEnabled/...）。
- Native owner：`bridge.listTabResources/clearTabResources/*CaptureSettings/resourceStats` → Rust `list_tab_resources`/`clear_tab_resources`/`get_resource_capture_settings`/`set_resource_capture_settings`/`resource_stats`（bridge.rs:1463/1481-1484）。
- Resource owner：后端请求拦截/瀑布 watcher（后台网络采集，非前端创建）。
- Credential：无（DTO 服务端脱敏）。
- Dependencies：`bridge`。
- 评级：C1（live）。目标 C3。

### credential — C1（安全边界，非可组合）
- 用户目的：凭据（keyring）存取。
- UI owner：`src/components/browser/CredentialList.vue`。
- State owner：Rust `KeyringStore`（`src-tauri/src/core/keyring_store.rs`，SERVICE="com.jizhijiandan.mvp"）；**无前端 KeyringStore**。
- Intent owner：Rust KeyringStore（save_token/get_token/delete_token）。
- Native owner：前端仅 `import_browser_credentials`/`list_browser_credentials`/`fill_browser_credential`/`configure_repo`（bridge.ts:478-488/157）→ Rust `import_browser_credentials`(6775)/`list_browser_credentials`(6807)/`fill_browser_credential`(6971)/`configure_repo`(2122)。**无通用 `keyring_save/get/delete` 前端命令**。
- Resource owner：SECURITY_SENSITIVE（keyring I/O）。
- Credential：核心 —— 前端永不直接读写 keyring；`fillBrowserCredential` 只传 `credential_id`+`tabId`，Rust 读 keyring 注入表单（DEV-03）。
- Persistence：os_keyring（敏感）。
- Dependencies：`bridge`。
- 评级：C1（registered + governed + keyring-backed）。**分类：安全边界 / 共享基础设施（NOT_COMPOSABLE_BY_DESIGN）**——按 section 24 不强行 optional；安全性优先于 Capability 数量。
- 已知偏差：registry `provides: credential.save/get/delete` 未作为离散前端命令实现（仅 browser-credential 路径 + 内部 keyring）。

### workbench — 框架/Shell（非后端能力）
- 用户目的：命令面板 / 布局编排。
- UI owner：无专属组件；`useWorkbenchStore`。
- State owner：`useWorkbenchStore`（`src/stores/useWorkbenchStore.ts`）。
- Intent owner：`useWorkbenchStore.open/toggleTools`（委托 `useBrowserStore.activateGrid()` 等）。
- Native owner：**无 `workbench_*` 命令**（bridge.ts / main.rs 均无）；`open('grid')`→`browser.activateGrid()`。
- 评级：**框架/Shell 编排**，非独立后端能力（section 15）。不应强行独立成 Capability；保持为 Workbench Shell 特性。

---

## 框架 vs 能力 分类裁定（section 15）

- **框架/Core 保留**：Workbench Shell、布局容器、通用导航、通用 contribution 宿主、theme、窗口控制、Capability Center、Settings 框架、通用命令基础设施、恢复、诊断、Semantic Governance。
- **能力（真实后端/状态边界）**：browser/grid/workspace/terminal/bookmark/git/database/agent/skill/plugin/knowledge_graph/task/session/script/notes/resource_collection/credential。
- **降级为框架**：workbench（无原生命令，纯编排）。
- **安全边界（非 optional）**：credential（keyring 核心）。

## 已知债务（诚实，不静默消失）

1. **bookmark 成熟度漂移**：registry `status: NOT_INTEGRATED` 与 phase8b tag（C3）冲突；HANDOFF 已更正为官方 C2 / C3 PENDING。需统一 registry 状态。
2. **agent/skill 运行时未实现**：前端 store + 只读后端命令存在，但 `agent_chat`/`skill_list`/`skill_run` 等 Rust 命令缺失 → 真实能力为 C0 壳。
3. **skill 无专属 owner**：状态寄居 `useAgentStore.skills`，需 SCR 定 semanticOwner 后抽离。
4. **notes registry owner 偏差**：`semanticOwner:null` 但实际 `useVaultStore` 存在。
5. **credential provides 动词偏差**：`credential.save/get/delete` 未离散实现；仅 browser-credential 路径 + 内部 keyring。
6. **database 未可组合**：always-loaded 面板，达到 C3 需抽可选能力包（Debt-8E-5）。
7. **git 物理状态未迁入**：`useGitStore` 仍在 `src/stores/`；App.vue 静态直连写闸门（阻断 C3）。
8. **profiles.yaml 组合态保守**：browser/workspace/terminal 物理已 C3，但 profiles 仍 TARGET_COMPOSABLE（演示口径，非代码事实）。

## 下一步（STAGE C 起）

优先迁 **database**（首个需重点验证 credential + 连接 + 原生 + 资源生命周期的能力）：抽 `capabilities/database/`，经 public 边界，absent→不挂载面板/不建连接，证明 C3/C5 资源释放证据。
