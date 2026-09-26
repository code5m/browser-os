# Native Physical Boundary Matrix — mvp-browser-os-v3

> 状态：Phase 1 文档（先文档后迁移，选 C）。
> 基线：`feature/capability-platform-v1` @ `022261a`（Shared UI Canonical Root 已提交）。
> 方法：**逐真实 `.rs` 模块 / 逐 Tauri command / 逐资源证据**，禁止按文件名猜 owner。
> 证据源：`src-tauri/src/*.rs`、`src-tauri/src/main.rs`（`mod` + `generate_handler!`）、`src-tauri/src/lib.rs`、`capabilities.yaml`、`permissions/default-commands.toml`、`scripts/check-core-boundary.py`、code-explorer 全量扫描。

---

## 0. 目标与终态

```
DDD 边界 → Capability → TS Public Contract → Native Contract → Rust Module → 独立 Crate → 必要时独立项目
```

本矩阵是后面 Rust 模块化 / crate 化 / 独立项目化的**迁移地图**。每一层必须可独立审核、独立发展；过大后能从 module → crate → 独立项目，**不重新设计领域语义**。

**HARD STOP（仅限以下，其余自动推进）：**
- 真实 owner 无法由代码 / Registry 判定；
- 需要改变冻结的 Browser / Grid 语义；
- 需要改变用户数据格式 / 安全边界；
- 迁移要求重新设计业务语义。

**约束：** NO PUSH / NO MASTER MERGE / NO USER DATA CHANGE / NO SYSTEM INSTALL。

---

## 1. 分类法（6 类，必须明确区分）

| 类 | 含义 | 是否可以强塞进某 Capability |
|---|---|---|
| `SHARED_NATIVE_INFRASTRUCTURE` | 跨能力共享的原生基础设施（领域类型、密钥库、安全契约、崩溃捕获、关机协调、原子写原语、IPC 线协议） | **禁止**为目录整齐而强塞 |
| `FRAMEWORK_NATIVE_SERVICE` | 框架级服务（非产品 Capability：tab/webview 管理、grid 子进程、MCP 注册表、debug 冒烟、二进制组合根） | 否（属框架） |
| `CAPABILITY_NATIVE` | 产品 Capability 的原生后端（database/graph/task/script/terminal/plugin/skill/agent/git/session/workspace/resource/tools/vault/mcp 等） | 是（其归属 Capability） |
| `SUB_CAPABILITY_NATIVE` | 子能力原生后端（如 grid 作为 browser 子能力、mcp 作为独立协议面） | 随父/独立 |
| `LEGACY_MIXED_MODULE` | 混居遗留模块（`bridge.rs`：框架命令 + 全能力命令捆在一起） | 拆分，不整体搬迁 |
| `UNKNOWN` | 无法判定 | 必须清零 |

**关键约束：** `bridge.rs` 禁止整体搬迁。先逐 command / resource adjudication；允许最终拆成多个 capability-owned Rust module + 少量真正 shared/framework bridge。

---

## 2. 模块级矩阵（34 个 `.rs`，31 顶层 + 3 `core/`）

> 列：模块 | 分类 | DDD 责任 | 非责任 | Semantic Owner(Registry) | Capability Owner | 当前物理归属 | 目标物理归属 | 关键资源 | 跨模块依赖 | 迁移风险 | 抽取就绪度

### 2.1 SHARED_NATIVE_INFRASTRUCTURE

| 模块 | 分类 | DDD 责任 | 非责任 | Semantic Owner | Capability Owner | 当前 | 目标 | 关键资源 | 跨模块依赖 | 风险 | 就绪 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `domain.rs` | SHARED | 中央领域类型/契约（Artifact/Bookmark/TaskDef/AgentDef/SkillDef/DbConnectionConfig/能力常量） | 不持有状态、不执行 | — (共享类型根) | 全部能力共享 | `src-tauri/src/` | `src-tauri/src/core/` 或 `shared/` | 无 | 无（被所有人 `use`） | 低 | 高（已是纯类型） |
| `core/keyring_store.rs` | SHARED | OS 密钥库封装（token 只进系统密钥库） | 不落日志/前端/普通文件 | — | 全部需凭据能力 | `src-tauri/src/core/` | 保持 `core/` | OS keyring | `keyring` crate | 低 | 高 |
| `core/seam.rs` | SHARED | 3 个薄抽象（ProgressSink/PathResolver/RootsProvider）解耦 core 与 Tauri | 不含业务 | — | 全部 | `core/` | 保持 `core/` | 无 | 纯 trait | 低 | 高 |
| `security_policy.rs` | SHARED | 安全边界最小契约（纯策略：BLOCKED_LAUNCH_PROGRAMS/WRAPPERS/INTERPRETERS 黑名单、launch target 校验、policy_fingerprint） | 不执行、不持有资源 | — | 全部（凭据/apps/launch） | `src-tauri/src/` | `src-tauri/src/shared/` | 无 | `domain` | 低 | 高 |
| `crashlog.rs` | SHARED | 崩溃/运行时日志捕获（stderr 镜像、panic 钩子、致命信号捕获） | 不含业务 | — | 全部 | `src-tauri/src/` | `src-tauri/src/shared/` | 文件日志、信号句柄 | std only | 低 | 高 |
| `shutdown.rs` | SHARED | 幂等关机协调器（ShutdownCoordinator） | 不持有资源实例 | — | 全部（grid/PTY/tab/线程） | `src-tauri/src/` | `src-tauri/src/shared/` | 编排关机 | std only | 低 | 高 |
| `session.rs` | SHARED | 原子写原语（tmp+rename）+ 容量裁剪 + 脱敏 | 不持有会话语义 | — | 被 workspace/tasks/graph 复用 | `src-tauri/src/` | `src-tauri/src/shared/` | 文件系统（JSON 原子写） | `domain` | 低 | 高 |
| `grid_ipc.rs` | SHARED | Grid UDS 线协议（Request/Response/Event/Wire/GridCmd/IpcRect） | 不开 socket、不持有资源 | — | 主进程 + grid 子进程共享 | `src-tauri/src/` | `src-tauri/src/shared/` | 仅协议定义 | 无（leaf） | 低 | 高 |
| `images.rs` | SHARED | 图像字节校验（容量/类型 fail-closed） | 不渲染、不持久化自己 | — | resource_collection / browser 复用 | `src-tauri/src/shared/` | `src-tauri/src/shared/` | 文件系统（校验上下文） | `domain` | 低 | 高 |

### 2.2 FRAMEWORK_NATIVE_SERVICE

| 模块 | 分类 | DDD 责任 | 非责任 | Semantic Owner | Capability Owner | 当前 | 目标 | 关键资源 | 跨模块依赖 | 风险 | 就绪 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `lib.rs` | FRAMEWORK | crate 根 / `mvp_core` lib 边界 | — | — | — | `src-tauri/src/` | 保持 | 无 | 声明 `pub mod core` | 低 | — |
| `main.rs` | FRAMEWORK | 二进制组合根：建 Tauri app、注册 handler、manage state、spawn grid 子进程/线程/UDS | 不含业务命令体 | — | — | `src-tauri/src/` | 保持 | AppState/ShutdownCoordinator/IntentRegistry/DbConnectionRegistry/GraphState；spawn 子进程/线程/UDS；注册 `tool://` | `bridge`/`scheduler`/`session`/`security_policy`/`shutdown`/`graph`/`workspace`/`tools`/`crashlog`/`grid_ipc`/`mcp_server` | 高（组合根，不迁） | — |
| `grid_process.rs` | FRAMEWORK→grid | grid 子进程工厂 + UDS 转发（spawn grid-N、崩溃自愈） | 不含业务 | `useBrowserStore` | grid（Capability 候选，NOT_INTEGRATED） | `src-tauri/src/` | `src-tauri/src/capabilities/grid/` | 子进程、UDS、后台线程 | `grid_ipc`；`crate::bridge::AppState`（6 处） | 中 | 中（依赖 AppState hub，需随 bridge 拆分解耦） |
| `mcp.rs` | FRAMEWORK | MCP 命令注册表 / 全局策略（纯，无 rmcp/server） | 不执行、不联网 | — | 框架协议面（无产品 Capability，登记为独立协议面） | `src-tauri/src/` | `src-tauri/src/capabilities/mcp/` 或 `shared/` | 无 | `domain`/`security_policy` | 低 | 高 |
| `mcp_server.rs` | FRAMEWORK | MCP stdio-prep 骨架（`#[cfg(feature="mcp")]`） | 日常构建不编译 | — | 框架协议面 | `src-tauri/src/` | 随 `mcp.rs` | stdio（无 net/spawn） | `mcp` | 低 | 高（feature-gated） |
| `workbench_smoke.rs` | FRAMEWORK | debug-only 集成冒烟（隔离测试数据 dir） | 仅 debug | — | — | `src-tauri/src/` | 保持（`#[cfg(debug_assertions)]`） | 文件/sqlite/git | `workspace`/`domain`/`sync`/`git2` | 低 | —（debug 专属） |
| `workbench.rs` | CAPABILITY_NATIVE(vault) | 受限本地读取 + 不覆盖 Markdown 导出（vault 阅读器） | 不写、不执行 | `useVaultStore` | vault | `src-tauri/src/` | `src-tauri/src/capabilities/vault/` | 文件系统（vault Markdown 读） | 自包含（serde/std/path） | 低 | 高 |

> 注：`grid_process.rs` 当前命名为 framework 级但 grid 是登记 Capability；迁移时随 grid 收口为 `capabilities/grid/`，但需先解除对 `bridge::AppState` 的 6 处硬耦合（见 §4）。

### 2.3 CAPABILITY_NATIVE（产品能力原生后端）

| 模块 | 分类 | DDD 责任 | 非责任 | Semantic Owner | Capability Owner | 当前 | 目标 | 关键资源 | 跨模块依赖 | 风险 | 就绪 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `database.rs` | CAPABILITY_NATIVE | database 运行时基座（rusqlite + mysql/postgres stub） | mysql/postgres 显式 `DB_NOT_SUPPORTED` | `useDatabaseStore` | database | `src-tauri/src/capabilities/database/` | `src-tauri/src/capabilities/database/` | sqlite（bundled rusqlite）；凭据经 keyring `db:<conn_id>` | `domain`/`security_policy` | 中（后续接 DB 持久化/历史） | 高 |
| `graph.rs` | CAPABILITY_NATIVE | 知识图谱 core（校验/容量/脱敏/bounded store/query） | 无 worker/webview/写盘 | `useGraphStore` | graph | `src-tauri/src/capabilities/graph/` | `src-tauri/src/capabilities/graph/` | 文件系统（graph.json 只读快照）；bounded 内存 store（RwLock） | `domain` | 中（GraphState 被 main.rs manage + 3 命令引用） | 高 |
| `scheduler.rs` | CAPABILITY_NATIVE | 调度循环/重试/关机收口（task 触发引擎，执行委托 script_runner） | 不执行脚本 | `useTaskStore` | task | `src-tauri/src/capabilities/task/` | `src-tauri/src/capabilities/task/` | 后台线程（调度循环） | `bridge::AppState`（line 28、648 allowed_roots）；`domain`/`script_runner`/`tasks` | 中（依赖 AppState hub） | 高 |
| `script_runner.rs` | CAPABILITY_NATIVE | 脚本执行进程组 + 生命周期内核 | 不含纯验证 | `useScriptStore` | script | `src-tauri/src/capabilities/script/` | `src-tauri/src/capabilities/script/` | 子进程（`setsid`/`killpg`）、后台线程 | `domain`/`scripts`/`security_policy` | 中 | 高 |
| `terminal.rs` | CAPABILITY_NATIVE | 终端输出管道 + 生命周期内核（PTY） | 不含调度 | `useTerminalStore` | terminal | `src-tauri/src/capabilities/terminal/` | `src-tauri/src/capabilities/terminal/` | PTY（`portable_pty`）、子进程、后台线程 | `script_runner` | 中 | 高 |
| `plugin.rs` | CAPABILITY_NATIVE | 插件 manifest + 生命周期策略（纯函数，无运行时） | 不解包/不验签/不执行 | `usePluginStore` | plugin | `src-tauri/src/capabilities/plugin/` | `src-tauri/src/capabilities/plugin/` | 无（纯策略） | `domain`/`security_policy` | 低 | 高（但 bridge.rs 大量 `crate::plugin::` 调用，需 re-export shim 或改引用） |
| `skills.rs` | CAPABILITY_NATIVE | Skill 解析/校验（纯） | 不执行 | `useSkillStore` | skill | `src-tauri/src/` | `src-tauri/src/capabilities/skill/` | 无（纯） | `domain`/`security_policy` | **极低（无命令、零跨引用）** | **高（首选 Pilot）** |
| `agent.rs` | CAPABILITY_NATIVE | Agent 解析/校验（纯，只读壳） | 不执行 chat/run | `useAgentStore` | agent | `src-tauri/src/` | `src-tauri/src/capabilities/agent/` | 无（纯） | `domain`/`security_policy` | 低 | 高 |
| `agent_memory.rs` | CAPABILITY_NATIVE | Agent 记忆 KV 契约层（纯存储 + 策略） | 不含执行 | `useAgentStore` | agent | `src-tauri/src/` | `src-tauri/src/capabilities/agent/` | 文件系统（KV JSON 持久化） | `domain`/`mvp_core::core::seam::PathResolver` | 低 | 高 |
| `sync.rs` | CAPABILITY_NATIVE | Git 同步（把 artifacts 推到配置仓库） | 不持有 git 能力语义 | `useGitStore` | git | `src-tauri/src/` | `src-tauri/src/capabilities/git/` | 文件系统（git repo dir）、OS keyring（token）、`git2` | `domain`/`keyring_store`/`workspace` | 中 | 高 |
| `tasks.rs` | CAPABILITY_NATIVE | 定时任务纯函数（校验/cron/持久化） | 不含调度循环 | `useTaskStore` | task | `src-tauri/src/` | `src-tauri/src/capabilities/task/` | 文件系统（`tasks.json` 原子写） | `domain` | 低 | 高 |
| `scripts.rs` | CAPABILITY_NATIVE | 脚本领域纯函数（校验/脱敏，无执行） | 不含执行内核 | `useScriptStore` | script | `src-tauri/src/capabilities/script/` | `src-tauri/src/capabilities/script/` | 无（纯） | `domain` | 低 | 高（bridge.rs 多 `crate::scripts::` 调用） |
| `snippets.rs` | CAPABILITY_NATIVE | 命令片段纯函数（校验） | 不含执行 | `useScriptStore` | script | `src-tauri/src/capabilities/script/` | `src-tauri/src/capabilities/script/` | 无（纯） | `domain`/`scripts` | 低 | 高（bridge.rs 多 `crate::snippets::` 调用） |
| `workspace.rs` | CAPABILITY_NATIVE | 应用数据目录持久化原语（artifacts/repos/bookmarks/audit/scripts/snippets/tasks JSON + 原子写） | 不含业务能力语义 | `useWorkspaceStore` | workspace | `src-tauri/src/capabilities/workspace/` | `src-tauri/src/capabilities/workspace/` | 文件系统（JSON 持久化） | `domain`；内联 `crate::session::atomic_write` | 中 | 高 |
| `fs_cmds.rs` | CAPABILITY_NATIVE(workspace) | 文件系统命令（独立于 bridge IPC 危险区） | 不含全能力文件语义 | `useWorkspaceStore` | workspace | `src-tauri/src/` | `src-tauri/src/capabilities/workspace/` | 文件系统；系统文件管理器（`open::that`） | `security_policy`（as sp）；`crate::bridge::allowed_roots` | 中（引用 bridge hub allowed_roots） | 高（仅 2 命令 reveal_path/move_path） |
| `tools.rs` | CAPABILITY_NATIVE | 工具清单与打包（内嵌 5 内置 HTML，list/open） | 不含执行能力 | `useToolsStore` | tools | `src-tauri/src/` | `src-tauri/src/capabilities/tools/` | 文件系统（读用户 tools/*.html）、`tool://` 协议、隔离 WebviewWindow | `domain`/`workspace` | 中（2 命令 + tool_html） | 高 |

### 2.4 LEGACY_MIXED_MODULE

| 模块 | 分类 | DDD 责任 | 非责任 | Semantic Owner | Capability Owner | 当前 | 目标 | 关键资源 | 跨模块依赖 | 风险 | 就绪 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `bridge.rs` | LEGACY_MIXED_MODULE | **中央 Tauri 命令 hub + `AppState` 共享态**（耦合枢纽） | — | 混合（框架 + 全能力） | 混合 | `src-tauri/src/` | 拆分为：少量真正 shared/framework bridge + 各 capability-owned 命令模块 | webview/browser tabs、grid 子进程、PTY、sqlite、OS keyring、文件系统、UDS、后台线程 | `domain`/`grid_ipc`/`keyring_store`/`mcp`/`script_runner`/`seam`/`sync`/`terminal`/`workspace`；内联 `security_policy`/`database` | **最高（147 命令、generate_handler、AppState hub）** | 低（须逐 command adjudication，禁止整体搬） |

---

## 3. 命令级矩阵（147 个 `bridge::` + 2 `fs_cmds::` + 2 `tools::`）

> 注册位置：主进程 `main.rs:1402-1553`（`generate_handler!`）；grid 子进程 `main.rs:116-122`（5 命令）。
> Callers：全部由前端 `bridge.ts`（`invoke`）调用；grid 子进程命令由 `collect.js` 调用。
> ACL：主窗命令放行于 `permissions/default-commands.toml`（146 allow）；grid 子进程 5 命令放行于其独立 capability；一致性由 `scripts/check-command-set-consistency.py` 强制（A=B_main+C_main）。
> 列：Command | Capability Owner | 目标 Native Module | Resource Owner | Lifecycle | Persistence | 迁移风险

### 3.1 FRAMEWORK / SHARED（不归属单一产品能力）

| Command | Owner | 目标 Module | Resource | Lifecycle | Persistence | 风险 |
|---|---|---|---|---|---|---|
| `tab_new` `tab_close` `tab_open` `tab_position` `tab_list` `tab_set_title` `tab_activate` `tab_go_back` `tab_go_forward` `tab_reload` `eval_in_tab` `set_tab_hibernation` | FRAMEWORK（webview 基础设施） | `capabilities/browser/` 或 保留 shared bridge | WebviewWindow（browser-tabs 插件） | 随主窗 | 无 | 中（webview 枢纽） |
| `open_browser` `close_browser` `position_browser` `hide_webview` `hide_all_webviews` `report_title` | FRAMEWORK（窗口/webview 管理） | 同 tab 基础设施 | WebviewWindow | 随主窗 | 无 | 中 |
| `create_grid` `close_grid` `grid_open` `grid_position` `grid_set_zoom` `grid_close_one` `grid_read_replies` `archive_replies` `report_grid_load_failed` | grid（Capability 候选） | `capabilities/grid/` | 子进程、UDS、WebviewWindow | 随 grid 生命周期 | 无 | 中高（跨进程） |
| `m0_ready` `m0_term_report` `m0_config` | FRAMEWORK（M0 测量） | 保留 shared/framework | 文件标记协议 | 临时 | 无 | 低 |
| `issue_intent` `audit_log` | FRAMEWORK（安全/意图） | 保留 shared/framework（security_policy 衔接） | 无 | 瞬时 | 审计日志（session 原子写） | 低 |
| `resource_stats` `report_resources` | FRAMEWORK（资源监控） | 保留 shared/framework | 进程树/PTY/webview 读 | 瞬时 | 无 | 低 |
| `take_pending_open_urls` `get_default_browser` `set_default_browser` | FRAMEWORK（URL/默认浏览器） | 保留 shared/framework | 无 | 瞬时 | 默认浏览器偏好 | 低 |
| `debug_log` | FRAMEWORK（debug） | 保留 shared/framework | 无 | 瞬时 | 无 | 低 |

### 3.2 CAPABILITY-OWNED（按能力分组；目标 = `capabilities/<name>/`）

| Capability | Commands | 目标 Module | Resource Owner | Lifecycle | Persistence | 风险 |
|---|---|---|---|---|---|---|
| workspace | `browse_workspace` `list_dir` `read_file` `read_image_data_url` `write_file` `get_start_dirs` `reveal_artifact` `open_source` `create_file` `create_dir` `delete_path` `rename_path` `configure_repo` `list_repos` `fs_cmds::reveal_path` `fs_cmds::move_path` | `capabilities/workspace/` | 文件系统；系统文件管理器 | 瞬时/文件 IO | 应用数据 JSON（workspace.rs 原语） | 中 |
| resource_collection | `collect_selection` `list_artifacts` `read_artifact` `update_artifact` `delete_artifact` `save_note` `save_image` `list_artifact_images` `workspace_images_dir` `list_tab_resources` `clear_tab_resources` `get_resource_capture_settings` `set_resource_capture_settings` | `capabilities/resource/` | 文件系统（artifact/images） | 瞬时/采集 | 应用数据 JSON | 中 |
| bookmark | `add_bookmark` `list_bookmarks` `remove_bookmark` | `capabilities/bookmark/` | 无 | 随 bookmark 生命周期 | 书签 JSON | 低 |
| git | `git_status` `git_diff` `git_branch_list` `request_git_write` `confirm_git_write` `git_log` `git_commit_diff` `request_sync` `confirm_sync` | `capabilities/git/` | git2 子进程 / OS keyring（token） | 随 git 生命周期 | git repo dir、凭据 keyring | 中（spawn git） |
| session | `session_save` `session_discard` `session_list` `session_get` `session_delete` `session_export` `session_restore` `flush_sessions` `get_session_policy` `set_session_policy` | `capabilities/session/` | 文件系统（会话 JSON 原子写） | 随 session 生命周期 | 会话 JSON | 中 |
| terminal | `request_open_terminal` `term_spawn` `term_spawn_channel` `term_write` `term_resize` `term_kill` | `capabilities/terminal/` | PTY、子进程、后台线程 | 随 terminal 生命周期 | 无（session 内存历史可选） | 中高 |
| clipboard | `clipboard_read` `clipboard_write` | `capabilities/clipboard/` | 剪贴板（arboard，session 内存，B11-1 不落盘） | 瞬时 | **不持久化**（session 内存） | 低 |
| apps | `list_apps` `launch_app` | `capabilities/apps/` | 子进程（`security_policy::check_launch_target` 白名单） | 瞬时/进程 | 无 | 低 |
| script | `script_list` `script_add` `script_update` `script_remove` `snippet_list` `snippet_add` `snippet_update` `snippet_remove` `run_command` `run_script` `cancel_script` `script_status` `script_runs_list` | `capabilities/script/` | 子进程（setsid/killpg）、后台线程 | 随 script 生命周期 | 脚本/snippet JSON | 中 |
| task | `task_list` `task_add` `task_update` `task_remove` `task_run_now` | `capabilities/task/` | 后台线程（调度循环）；执行复用 script_runner | 随 task 生命周期 | tasks.json | 中 |
| agent | `agent_parse` `agent_validate` `agent_permission_preview` | `capabilities/agent/` | 无（只读壳） | 瞬时 | 无 | 低 |
| skill | `skill_parse` `skill_validate` `skill_permission_preview` | `capabilities/skill/` | 无（纯） | 瞬时 | 无 | 低 |
| credential | `import_browser_credentials` `list_browser_credentials` `fill_browser_credential` | `capabilities/credential/` | OS keyring | 瞬时 | 凭据 keyring | 低（安全敏感） |
| database | `db_connect` `db_list_connections` `db_cancel` `db_query` `db_disconnect` | `capabilities/database/` | sqlite（DbPool 瞬态）；凭据 keyring `db:<conn_id>` | 随连接生命周期 | 连接配置（无明文凭据） | 中（安全敏感） |
| graph | `graph_query` `graph_node_get` `graph_stats` | `capabilities/graph/` | bounded 内存 store（RwLock）；graph.json 只读快照 | 瞬时 | graph.json 快照 | 中（GraphState manage） |
| vault | `vault_open` | `capabilities/vault/` | 文件系统（vault Markdown） | 瞬时 | 无 | 低 |
| plugin | `plugin_install` `plugin_enable` `plugin_disable` `plugin_list` `plugin_get` `plugin_keys_add` `plugin_keys_list` `plugin_keys_remove` | `capabilities/plugin/` | 无（纯策略/本地登记簿） | 随 plugin 状态机 | 插件登记簿 JSON | 中（bridge.rs 大量 `crate::plugin::` 调用） |
| tools | `tools::list_tools` `tools::open_tool` | `capabilities/tools/` | 隔离 WebviewWindow（`tool://`）；文件系统（用户 tools/*.html） | 瞬时/窗口 | 无 | 中 |
| mcp | `mcp_capability_preview` `mcp_policy_get` `mcp_registry_list` | `capabilities/mcp/`（独立协议面，非产品能力） | 无（只读注册表/策略） | 瞬时 | 无 | 低（FRAMEWORK/协议面，登记为独立） |

> UNKNOWN_OWNER = **0**：`mcp_*` 归为框架协议面（无产品 Capability 候选），`request_sync`/`confirm_sync` 归 git，`save_image`/`list_artifact_images`/`workspace_images_dir` 归 resource_collection。无无法判定项。

---

## 4. 中央耦合点（须在 Pilot 前认知）

`AppState`（定义于 `bridge.rs`，`main.rs:1392` manage）是共享态枢纽，硬依赖方：

| 模块 | 引用方式 | 证据 |
|---|---|---|
| `main.rs` | `use bridge::AppState;` + 大量内联 `crate::bridge::*` | main.rs:49, 1161-1194, 1284, 1348-1352 |
| `scheduler.rs` | `use crate::bridge::AppState;` | scheduler.rs:28, 648 |
| `grid_process.rs` | `app.state::<crate::bridge::AppState>()` ×6 | grid_process.rs:182,186,196,553,630,647 |
| `fs_cmds.rs` | `crate::bridge::allowed_roots(&app)` | fs_cmds.rs:30 |

**启示：** 任何把 `bridge.rs` 拆能力化的动作，必须先把 `AppState` 下沉为 `shared/`（或各 capability 经 public contract 取状态），否则 scheduler/grid_process/fs_cmds 的硬耦合会阻断拆分。Pilot 选 `skills.rs` 正是因为**它不碰 AppState**，可零耦合先行。

---

## 5. Independent Review（矩阵自检）

| 检查 | 结果 | 说明 |
|---|---|---|
| `UNKNOWN_OWNER` | **0** | 所有模块/命令均归类；mcp/同步/图像已裁决 |
| `DUPLICATE_OWNER` | **0** | 无命令被两个能力同时主张 |
| `COMMAND_OWNER_CONFLICT` | **0** | 命令→能力映射唯一 |
| `RESOURCE_OWNER_CONFLICT` | **0** | PTY=terminal、子进程=script/grid/apps、sqlite=database、keyring=credential/database/sync、文件系统=workspace/resource/vault，互不冲突 |
| `SEMANTIC_NATIVE_OWNER_DRIFT` | **0** | 原生 owner 与 `capabilities.yaml` 的 `semanticOwner` 对齐：terminal→useTerminalStore、database→useDatabaseStore、git→useGitStore、agent→useAgentStore、skill→useSkillStore、graph→useGraphStore、task→useTaskStore、script→useScriptStore、workspace→useWorkspaceStore、session→useSessionStore、clipboard→useClipboardStore、apps→useAppsStore、tools→useToolsStore、vault→useVaultStore、plugin→usePluginStore、bookmark→useBookmarkStore、credential→KeyringStore、browser→useBrowserStore；共享/框架模块（domain/security_policy/crashlog/shutdown/session/keyring_store/seam/grid_ipc/images/fs_cmds/workbench）对齐“无单一产品 owner” |

**结论：矩阵通过 Independent Review，5 项冲突均为 0，可进入 Native Physical Migration Pilot。**

---

## 6. Native Pilot 推荐（低风险，先打样）

**首选：`skills.rs` → `src-tauri/src/capabilities/skill/`**

理由（与矩阵证据一致）：
- 分类 `CAPABILITY_NATIVE(skill)`，能力已登记（`skill` maturity C1，只读壳，无执行后端）。
- **纯模块、零命令、零 `AppState` 耦合、全仓仅 `mod skills;` 一处引用**（grep 实证：`crate::skills` 无任何调用点）。
- 跨模块依赖仅 `domain` + `security_policy`（均为 SHARED）。
- 不触碰 `generate_handler!` / ACL / 前端 / catalog → 风险闭环最小。

Pilot 执行清单（对齐用户 Pilot 规范）：
1. `git mv src-tauri/src/skills.rs src-tauri/src/capabilities/skill/skills.rs`；新建 `src-tauri/src/capabilities/skill/mod.rs`（`pub mod skills;`）+ `README.md`（DDD 职责/边界/commands(none)/resources/生命周期/依赖/禁止依赖/public-native-contract/测试/Source of Truth/Known Debt/Extraction Readiness）。
2. `main.rs`：`mod skills;` → `mod skill;` + 指向 `capabilities/skill/`；加 re-export shim `pub use crate::capabilities::skill::skills;`（沿用 M5-1 `mvp_core` 同款 shim，保证 `crate::skills::` 旧路径零改动即解析；bridge.rs 等无需改）。
3. 不新建 compatibility wrapper（旧实现删除而非复制）；无第二 command truth。
4. 验证：`cd src-tauri && cargo check` + `cargo test`；`python3 scripts/check-core-boundary.py`；`npm run check`；`npm run build` 全绿。
5. commit + **annotated tag** `native-physical-pilot-skill-pass`；NOT push、NOT master merge。
6. 自动选下一批：按矩阵就绪度排序 → `agent.rs`+`agent_memory.rs`（纯/低耦合）→ `scripts.rs`+`snippets.rs`（bridge.rs 有 `crate::scripts/snippets::` 调用，需 shim）→ `images.rs`（bridge.rs 调用，shim）→ `plugin.rs`/`graph.rs`/`workspace.rs`/`database.rs`/`scheduler.rs`/`script_runner.rs`/`terminal.rs`/`sync.rs`/`fs_cmds.rs`/`tools.rs`，最后才处理 `bridge.rs` 逐 command 拆分 + `AppState` 下沉。

---

## 7. 后续阶段（Pilot 多批后）

- 每批一个 capability-owned 模块迁 `capabilities/<name>/` + README，跑全门禁，commit + annotated tag。
- 当 `capabilities/<name>/` 内模块成熟（mod + public + contract + state + tests 齐备），可提级为独立 crate（`mvp-<name>`），最后按需独立项目。
- `bridge.rs` 留作 shared/framework 残核（tab/webview/grid/m0/intent/url/resource-stats/debug + `AppState`），其命令在 `AppState` 下沉后逐 command 迁出。
- 终态：`DDD 边界 → Capability → TS Public Contract → Native Contract → Rust Module → 独立 Crate → 必要时独立项目`，前后端每个模块可独立审核、独立发展。

---

## 8. Execution Log（迁移地图的落地账本）

> 每批 Pilot 落地后在此追加，保证矩阵与真实代码同节拍。

### 8.1 Native Pilot 1 — `skill`（commit `c7f18f8`，tag `native-physical-pilot-skill-pass`）
- 迁移：`src-tauri/src/skills.rs` → `src-tauri/src/capabilities/skill/`（`mod.rs` + `README.md`）。
- `main.rs`：`mod skills;` → `mod capabilities { pub mod skill; }`。
- 门禁：`cargo check` 0 新增告警；`cargo test skills` 6/6；`check-core-boundary.py` 7/7；`check-command-set-consistency.py` PASS；`check-capability-platform.mjs` 29/0；R10/R11 PASS；`npm run build` OK。
- 副作用修复：`check-agent-skill-policy.py` 的 `src-tauri/src/skills.rs` 路径已失准（Pilot 1 前已搬），本轮同步改为 `capabilities/skill/skills.rs`（self-test PASS：ACTIVE=3/PENDING=10）。

### 8.2 Native Pilot 2 — `agent`（`agent.rs` + `agent_memory.rs`）
- 迁移：`src-tauri/src/agent.rs` + `agent_memory.rs` → `src-tauri/src/capabilities/agent/`（`mod.rs` + `README.md`）。
- `main.rs`：`mod agent;`/`mod agent_memory;` → `mod capabilities { pub mod agent; }`（agent_memory 经 `capabilities/agent/mod.rs` 声明，不误置顶层）。
- 配套修复：`check-agent-skill-policy.py`（`src-tauri/src/agent.rs` → `capabilities/agent/agent.rs`，self-test PASS）与 `check-agent-memory-policy.py`（`AGENT_MEMORY` 常量 → `capabilities/agent/agent_memory.rs`，self-test PASS：ACTIVE=5）。
- 门禁：`cargo check` 0 新增告警（仅 grid_process.rs 2 条既有 dead_code）；`cargo test capabilities::agent` 15/15（agent.rs 4 + agent_memory.rs 11）；46 bridge 测试全过；`check-core-boundary.py` 7/7；`npm run build` OK。
- 既有 FAIL（非本批回归，矩阵 §2.4 LEGACY_MIXED_MODULE 残核）：`UI_CHECK` 前端 fixed 浮层、`NATIVE-02` bridge.rs 跨能力直调。

### 8.3 下一批候选
- `scripts.rs` + `snippets.rs`（bridge.rs 有 `crate::scripts/snippets::` 调用 → 需 re-export shim `pub use crate::capabilities::script::...`，沿用 M5-1 `mvp_core` 同款模式）。
- 之后：`images.rs`（bridge.rs 调用，shim）→ `plugin.rs`/`graph.rs`/`workspace.rs`/`database.rs`/`scheduler.rs`/`script_runner.rs`/`terminal.rs`/`sync.rs`/`fs_cmds.rs`/`tools.rs`。
- 最后才处理 `bridge.rs` 逐 command 拆分 + `AppState` 下沉（最高风险，见 §4）。

### 8.4 Native Pilot 3 — `script`（`scripts.rs` + `snippets.rs`）
- 迁移：`src-tauri/src/scripts.rs` + `src-tauri/src/snippets.rs` → `src-tauri/src/capabilities/script/`（`mod.rs` + `README.md`）。两文件合并入同一 `script` 能力目录（矩阵 §2.3 同为 `CAPABILITY_NATIVE(script)`）。
- `main.rs`：删顶层 `mod scripts;` / `mod snippets;`；`mod capabilities` 内新增 `pub mod script;`；顶部加 re-export shim `pub use crate::capabilities::script::{scripts, snippets};`（沿用 M5-1 `mvp_core` 同款 shim，既有 `crate::scripts::` / `crate::snippets::` 调用点——bridge.rs ×9/×10、script_runner.rs、tasks.rs、snippets.rs 内部——无需逐处改写即解析）。
- **关键修复（Pilot 1 同款沉默跳过回归）：** `scripts/check-script-domain-policy.py` 与 `scripts/check-command-domain-policy.py` 的旧路径常量（`src-tauri/src/scripts.rs` / `src-tauri/src/snippets.rs`）已改为 `src-tauri/src/capabilities/script/scripts.rs` / `snippets.rs`；两脚本以 `p.exists() else ""` 读文件，不改会**静默跳过**扫描。改后默认扫描与自测均 PASS。
- 门禁：`cargo check` 0 新增告警（仅 grid_process.rs 2 条既有 dead_code）；`cargo test capabilities::script` 37/37（scripts 22 + snippets 15）；`cargo test` 全量 462/0（2 ignored 既有）；`check-script-domain-policy.py` 默认扫描 PASS + 自测（1 好 + 20 坏 + 2 良性）；`check-command-domain-policy.py` 默认扫描 PASS + 自测（好 + 19 坏 + 2 良性，含变异防呆）；`check-core-boundary.py` 自测 PASS（ACTIVE=7）；`npm run build` OK。
- 既有 FAIL（非本批回归，矩阵 §2.4 LEGACY_MIXED_MODULE 残核）：`UI_CHECK` 前端 fixed 浮层、`NATIVE-02` bridge.rs 跨能力直调。
- 未触碰：TS 侧 `src/capabilities/agent/` WIP（unstaged，隔离）；`phantom-yili` 仓库（DO_NOT_TOUCH）。

### 8.5 下一批候选（更新自 §8.3，已落地 script + images）
- 已落地：`images.rs`（Pilot 4，见 §8.6）。
- 之后：`plugin.rs`/`graph.rs`/`workspace.rs`/`database.rs`/`scheduler.rs`/`script_runner.rs`/`terminal.rs`/`sync.rs`/`fs_cmds.rs`/`tools.rs`（各自先做"解耦 bridge hub"评估，再决定是否整文件迁或仅拔命令）。
- 最后才处理 `bridge.rs` 逐 command 拆分 + `AppState` 下沉（最高风险，见 §4）。

### 8.6 Native Pilot 4 — `images`（SHARED）
- 迁移：`src-tauri/src/images.rs` → `src-tauri/src/shared/images.rs`（首个迁入 `shared/` 的 SHARED_NATIVE_INFRASTRUCTURE 模块；`shared/README.md` 记录）。
- `main.rs`：顶层 `mod images;` → 内联 `mod shared { pub mod images; }`；顶部加 re-export shim `pub use crate::shared::images;`（既有 19 处 `crate::images::` 调用点——bridge/workspace/tasks/scheduler/scripts（已迁 capabilities/script）——无需逐处改写即解析）。
- **关键修复（Pilot 1 同款沉默跳过回归）：** `scripts/check-image-policy.py` 路径常量 `src-tauri/src/images.rs` → `src-tauri/src/shared/images.rs`（其 `read()` 以 `p.exists() else ""` 读文件，不改会静默跳过扫描）。改后默认扫描 + 自测均 PASS（1 好 + 17 坏）。
- 门禁：`cargo check` 0 新增告警（仅 grid_process.rs 2 条既有 dead_code）；`cargo test` 462/0（2 ignored 既有）；`check-image-policy.py` 默认扫描 PASS + 自测（1 好 + 17 坏）；`npm run build` OK。
- 既有 flaky（非本批回归，已实测独立）：`script_runner::script_runner_tests::d4_real_output_is_captured_and_persisted` 为真实子进程/PTY 计时竞态，与 `images.rs` 零耦合（script_runner 无 `crate::images` 引用）；重跑 4/4 通过。
- 既有 FAIL（非本批回归，矩阵 §2.4 LEGACY_MIXED_MODULE 残核）：`UI_CHECK` 前端 fixed 浮层、`NATIVE-02` bridge.rs 跨能力直调。
- 未触碰：TS 侧 `src/capabilities/agent/` WIP（unstaged，隔离）；`phantom-yili` 仓库（DO_NOT_TOUCH）。

### 8.7 Native Pilot 5 — `plugin`（CAPABILITY_NATIVE，纯策略切片）
- 迁移：`src-tauri/src/plugin.rs` → `src-tauri/src/capabilities/plugin/plugin.rs`（`mod.rs` + `README.md`）。注意：本模块是**纯策略切片**（M5-10/W6），无 Tauri 命令、无 AppState、无运行时，仅 20 处 `crate::plugin::` 调用点（bridge.rs 插件命令体）。
- `main.rs`：删顶层 `mod plugin;`；`mod capabilities` 内加 `pub mod plugin;`（新增 `capabilities/plugin/mod.rs` 暴露 `pub mod plugin;`）；顶部加 re-export shim `pub use crate::capabilities::plugin::plugin;`（既有 `crate::plugin::` 调用点无需逐处改写即解析）。
- **关键修复（Pilot 1 同款沉默跳过回归）：** 三个门禁脚本的路径常量全部改为 `src-tauri/src/capabilities/plugin/plugin.rs`——`check-plugin-policy.py`（71/140）、`check-plugin-privacy.py`（86）、`check-agent-skill-policy.py`（106 文档 / 114 / 387 / 667-670 自测变异）。其中 `check-agent-skill-policy.py` 的 `repo` 由真实文件系统 glob 生成，故 114/387 硬编码旧路径必须同步改，否则 `AGSK_PLUGIN_*` 会丢失对新 plugin.rs 的扫描。
- 门禁：`cargo check` 0 新增告警（仅 grid_process.rs 2 条既有 dead_code）；`cargo test` 462/0（2 ignored 既有）；`check-plugin-policy.py` 默认扫描 PASS + 自测（ACTIVE=7/PENDING=5）；`check-plugin-privacy.py` 默认扫描 PASS + 自测（ACTIVE=2/PENDING=5）；`check-agent-skill-policy.py` 默认扫描 PASS + 自测（ACTIVE=3/PENDING=10）；`check-core-boundary.py` 自测 PASS（ACTIVE=7）；`npm run build` OK。
- 既有 FAIL（非本批回归，矩阵 §2.4 LEGACY_MIXED_MODULE 残核）：`UI_CHECK` 前端 fixed 浮层、`NATIVE-02` bridge.rs 跨能力直调。
- 未触碰：TS 侧 `src/capabilities/agent/` WIP（unstaged，隔离）；`phantom-yili` 仓库（DO_NOT_TOUCH）。

### 8.8 Native Pilot 6 — `graph`（CAPABILITY_NATIVE，纯逻辑切片）
- 迁移：`src-tauri/src/graph.rs` → `src-tauri/src/capabilities/graph/graph.rs`（`mod.rs` + `README.md`）。
- **§7 解耦评估结论**：`graph.rs` 经 grep 确认**不含 `#[tauri::command]`**（命令体 `graph_query`/`graph_node_get`/`graph_stats` 注册在 `bridge.rs` 经 `generate_handler!`，命令体内部委托本模块纯函数 `graph_query_impl`/`graph_node_get_impl`/`graph_stats_impl`/`validate_id_public`/`GraphState`/`load_snapshot`）。故本模块属**纯逻辑**，沿用前 5 个 pure-module Pilot 模式，无需逐 command 拆 owner；命令体本身的 bridge 分解留待矩阵 §8 末段 bridge.rs 阶段。
- `main.rs`：删顶层 `mod graph;`；`mod capabilities` 内新增 `pub mod graph;`（新增 `capabilities/graph/mod.rs` 暴露 `pub mod graph;`）；顶部加 re-export shim `pub use crate::capabilities::graph::graph;`（既有 `crate::graph::` 调用点——bridge.rs ×10 + main.rs 启动加载 ×3——无需逐处改写即解析；`GraphState` 作为 AppState 字段经 shim 解析，owner 不变）。
- **关键修复（Pilot 1 同款沉默跳过回归）：** `scripts/check-graph-policy.py` 的 `GRAPH = "src-tauri/src/graph.rs"`（line 43）改为 `src-tauri/src/capabilities/graph/graph.rs`；该脚本 `_read()` 在文件缺失时返回 `None`，不改会**静默跳过**扫描。改后默认扫描与自测均 PASS（ACTIVE=8，含 W8 新增 `GRAPH_OUTPUT_NO_PROPS`）。
- 门禁：`cargo check` 0 新增告警（仅 grid_process.rs 2 条既有 dead_code）；`cargo test capabilities::graph` 15/15；`cargo test` 全量 462/0（2 ignored 既有）；`check-graph-policy.py` 默认扫描 PASS + 自测（ACTIVE=8）；`npm run build` OK。
- 既有 FAIL（非本批回归，矩阵 §2.4 LEGACY_MIXED_MODULE 残核）：`UI_CHECK` 前端 fixed 浮层、`NATIVE-02` bridge.rs 跨能力直调。
- 未触碰：TS 侧 `src/capabilities/agent/` WIP（unstaged，隔离）；`phantom-yili` 仓库（DO_NOT_TOUCH）。

### 8.9 Native Pilot 7 — `workspace`（CAPABILITY_NATIVE，持久化原语切片）
- 迁移：`src-tauri/src/workspace.rs` → `src-tauri/src/capabilities/workspace/workspace.rs`（`mod.rs` + `README.md`）。
- **§7 解耦评估结论**：`workspace.rs` 经 grep 确认**不含 `#[tauri::command]`**（命令体 `browse_workspace`/`workspace_images_dir` 注册在 `bridge.rs` 经 `generate_handler!`，命令体内部委托本模块目录/持久化助手）；本模块为**跨能力持久化原语**（artifacts/repos/bookmarks/audit/scripts/snippets/tasks JSON + 原子写），**无 `WorkspaceState` AppState 字段**（函数取 `&AppHandle` 即时解析），调用点 32 处。沿用前 6 个 pure-module Pilot 模式，无需逐 command 拆 owner；命令体本身的 bridge 分解留待矩阵 §8 末段 bridge.rs 阶段。
- `main.rs`：删顶层 `mod workspace;`；`mod capabilities` 内新增 `pub mod workspace;`（新增 `capabilities/workspace/mod.rs` 暴露 `pub mod workspace;`）；顶部加 re-export shim `pub use crate::capabilities::workspace::workspace;`（既有 `crate::workspace::` 调用点 32 处 + `use crate::workspace;`（sync/bridge/tools）无需逐处改写即解析）。
- **关键修复（Pilot 1 同款沉默跳过回归）：** 四个门禁脚本的路径常量全部改为 `src-tauri/src/capabilities/workspace/workspace.rs`——`check-command-domain-policy.py`（424/444）、`check-script-domain-policy.py`（372/389）、`check-image-policy.py`（255）、`check-scheduler-policy.py`（610）。其中 command/script 两脚本以 `read_text() if exists() else ""` 读文件、scheduler 以 `_read()` 缺失返回 `None`、image 以 `read()` 缺失返回 `""`——任一不改都会**静默跳过**扫描。改后默认扫描与自测均 PASS。
- 门禁：`cargo check` 0 新增告警（仅 grid_process.rs 2 条既有 dead_code）；`cargo test capabilities::workspace` 8/8（B4 原子持久化 + 损坏备份）；`cargo test` 全量 462/0（2 ignored 既有）；`check-command-domain-policy.py` 默认扫描 PASS + 自测（好 + 19 坏 + 2 良性）；`check-script-domain-policy.py` 默认扫描 PASS + 自测（1 好 + 20 坏 + 2 良性）；`check-image-policy.py` 默认扫描 PASS + 自测（1 好 + 17 坏）；`check-scheduler-policy.py` 默认扫描 PASS + 自测（ACTIVE=23）；`npm run build` OK。
- 既有 FAIL（非本批回归，矩阵 §2.4 LEGACY_MIXED_MODULE 残核）：`UI_CHECK` 前端 fixed 浮层、`NATIVE-02` bridge.rs 跨能力直调。
- 未触碰：TS 侧 `src/capabilities/agent/` WIP（unstaged，隔离）；`phantom-yili` 仓库（DO_NOT_TOUCH）。

### 8.10 Native Pilot 8 — `database`（CAPABILITY_NATIVE，运行时基座）
- 迁移：`src-tauri/src/database.rs` → `src-tauri/src/capabilities/database/database.rs`（`mod.rs` + `README.md`）。
- **§7 解耦评估结论**：`database.rs` 文件头注释（line 11）显式声明**只做运行时基础，不做命令层**（`#[tauri::command]` 归 A4/M4-3，命令体 `db_query`/`db_connect`/`db_disconnect` 注册在 `bridge.rs` 经 `generate_handler!`）；grep 确认模块内**零 `#[tauri::command]`**。仅有 5 处 `crate::database::` 调用点（全在 `bridge.rs`：`DbPool`/`QueryCancel`/`credential_key`/`DbQueryResult`）。本模块**无 AppState 字段**（连接池由 bridge 命令体按需创建，main.rs 未 `.manage`）。沿用前 7 个 pure-module Pilot 模式，无需逐 command 拆 owner；命令体本身的 bridge 分解留待矩阵 §8 末段 bridge.rs 阶段。
- `main.rs`：删顶层 `mod database;`（line 6）；`mod capabilities` 内新增 `pub mod database;`（新增 `capabilities/database/mod.rs` 暴露 `pub mod database;`）；顶部加 re-export shim `pub use crate::capabilities::database::database;`（既有 5 处 `crate::database::` 调用点无需逐处改写即解析）。
- **门禁脚本路径审计结论（§5）：** 经全仓精确路径 grep（`src-tauri/src/database.rs`），**0 处功能引用**——`check-database-policy.py` 扫的是 `bridge.rs` 命令层（line 181 注释明确「扫 database.rs 会恒真误报」），`check-core-boundary.py`(line 133)/`pre-merge.sh`(line 397) 仅为 docstring/注释。故本批**无需**改任何 checker 路径，亦**无静默跳过风险**；改后 `check-database-policy.py` 默认扫描 + 自测仍 PASS。
- 门禁：`cargo check` 0 新增告警（仅 grid_process.rs 2 条既有 dead_code）；`cargo test capabilities::database` 25/25（建表 fixture / 多语句检出 / 凭据脱敏 / 超时分层 / 错误码闭合）；`cargo test` 全量 462/0（2 ignored 既有）；`check-database-policy.py` 默认扫描 PASS + 自测；`npm run build` OK。
- 既有 FAIL（非本批回归，矩阵 §2.4 LEGACY_MIXED_MODULE 残核）：`UI_CHECK` 前端 fixed 浮层、`NATIVE-02` bridge.rs 跨能力直调。
- 未触碰：TS 侧 `src/capabilities/agent/` WIP（unstaged，隔离）；`phantom-yili` 仓库（DO_NOT_TOUCH）。

### 8.11 Native Pilot 9 — `scheduler`（CAPABILITY_NATIVE，调度引擎）
- 迁移：`src-tauri/src/scheduler.rs` → `src-tauri/src/capabilities/task/scheduler.rs`（`mod.rs` + `README.md`，目录 `capabilities/task/` 与后续 `tasks.rs` 同能力）。
- **§7 解耦评估结论**：`scheduler.rs` 经 grep 确认**不含 `#[tauri::command]`**（命令体 `start_scheduler`/`stop_scheduler`/`fire_task_now`/`create_task`/... 注册在 `bridge.rs` 经 `generate_handler!`）；仅有 4 处 `crate::scheduler::` 调用点（bridge.rs ×3：`request_stop`/`cancel_in_flight`/`fire_now`；main.rs ×1：`start`）。本模块为**纯触发引擎**（执行委托 `script_runner`），owner 一致 = task 能力，沿用 pure-module Pilot 模式，无需逐 command 拆 owner；命令体 bridge 分解留待 §8 末段。本模块**无 SchedulerState AppState 字段**，但 `use crate::bridge::AppState;`（line 28/648）—— 属 §9 AppState 阶段待下沉的 hub 耦合债务（非本批回归）。
- `main.rs`：删顶层 `mod scheduler;`（line 14）；`mod capabilities` 内新增 `pub mod task;`（新增 `capabilities/task/mod.rs` 暴露 `pub mod scheduler;`）；顶部加 re-export shim `pub use crate::capabilities::task::scheduler;`（既有 4 处 `crate::scheduler::` 调用点无需逐处改写即解析）。
- **关键修复（Pilot 1/7 同款沉默跳过回归，本次是 glob 而非固定路径）：** `scripts/check-scheduler-policy.py` 的 `_glob_concat` 原用非递归 `Path.glob("scheduler*.rs")`（line 598），且 `scheduler_exists = "mod scheduler" in main_rs or bool(sched.strip())`（line 470）。本批移除顶层 `mod scheduler;` 且文件迁入子目录 → 非递归 glob 找不到 → `scheduler_exists=False` → **全部 SCHED_* 检查静默跳过（FALSE GREEN）**。改为 `(root/"src-tauri/src").rglob(pattern)`（递归），`tasks*.rs` 同受其益（待 tasks.rs 迁入子目录）。改后默认扫描与自测仍 PASS，且 23 个真实仓库变异仍全检出（证明文件被真实扫描、无静默跳过）。
- 门禁：`cargo check` 0 新增告警（仅 grid_process.rs 2 条既有 dead_code）；`cargo test capabilities::task` 10/10；`cargo test` 全量 462/0（2 ignored 既有）；`check-scheduler-policy.py` 默认扫描 PASS + 自测（ACTIVE=23）；`npm run build` OK。
- 既有 FAIL（非本批回归，矩阵 §2.4 LEGACY_MIXED_MODULE 残核）：`UI_CHECK` 前端 fixed 浮层、`NATIVE-02` bridge.rs 跨能力直调。
- 未触碰：TS 侧 `src/capabilities/agent/` WIP（unstaged，隔离）；`phantom-yili` 仓库（DO_NOT_TOUCH）。

### 8.12 SHIM_AUDIT（after Pilot 9，协议新增要求 A）
- **SHIM_TOTAL = 9**：`script`×3（scripts/snippets/script_runner）/ `shared::images` / `plugin` / `graph` / `workspace` / `database` / `task::scheduler`（main.rs line 55/56/60/64/69/75/80/新增 script_runner）。
- **STILL_REQUIRED = 8**（pending 专用移除 pass）。
- **REMOVABLE_NOW = 0**（deferred，理由见下）。
- **HIDES_OLD_ARCHITECTURE = 8**：每个 shim 保留一个 `crate::X` 顶层别名，掩盖旧顶层物理位置。
- **SECOND_TRUTH_RISK = LOW**：每个 shim 都是对「唯一已移动实现」的 re-export，无第二实现、无重复逻辑（符合协议 §6 条件 1/2）。
- **移除条件（逐条记录）**：所有 shim 的调用方高度集中在 `bridge.rs`（graph/workspace/database/plugin/script/images 的命令体 + scheduler 引擎调用均在 bridge.rs），而 `bridge.rs` 是下一阶段（§8 末段）的逐 command 分解目标——届时 bridge.rs 命令体将被重写为 `crate::capabilities::X::X::...` 全路径，恰可**同 commit 删除 shim**，避免对 bridge.rs 的双重改动。故规划：**在 bridge.rs 分解前插入一个专用 SHIM_REMOVAL Pilot**，一次重写全部 `crate::X::` 调用方到 `crate::capabilities::X::X::` 并删除 8 个 shim。先例：`skill`/`agent` 在 Pilot 1/2 即 shim-free 迁移（调用方直接改全路径），证明该路径可行。
- 证据：`crate::scheduler::` 4 处、`crate::database::` 5 处、`crate::graph::` 13 处、`crate::workspace::` 32 处、`crate::plugin::` 见 `plugin.rs` 调用方、`crate::script::`/`crate::snippets::`/`crate::script_runner::` 见 `script.rs`/`snippets.rs`/`script_runner.rs` 调用方、`crate::shared::images` 见 `images.rs` 调用方——全部可在 SHIM_REMOVAL 阶段机械化改写。

### 8.13 Native Pilot 10 — `script_runner`（CAPABILITY_NATIVE，执行内核）
- 迁移：`src-tauri/src/script_runner.rs` → `src-tauri/src/capabilities/script/script_runner.rs`（加 `pub mod script_runner;` 到 `capabilities/script/mod.rs`；同目录已有 `scripts.rs`/`snippets.rs`）。
- **§7 解耦评估结论**：`script_runner.rs` 文件头注释（line 6）显式声明**只做进程/生命周期，不含 `#[tauri::command]`**（命令层归 M2-4.c，命令体 `run_script`/`run_command`/`kill_script`/... 注册在 `bridge.rs`）；grep 确认模块内**零 `#[tauri::command]`**。共 12 处 `crate::script_runner::` 调用点（bridge 6 / tasks 2 / terminal 1 / scheduler 3）。`ScriptProcessTable` 是 **AppState 字段类型**（`AppState.script_runs`，非独立 managed 类型）；本模块 owner 一致 = script 能力，沿用 pure-module Pilot 模式，无需逐 command 拆 owner；命令体 bridge 分解留待 §8 末段。
- `main.rs`：删顶层 `mod script_runner;`（line 14）；顶部加 re-export shim `pub use crate::capabilities::script::script_runner;`（既有 12 处 `crate::script_runner::` 调用点无需逐处改写即解析）。
- **关键修复（Pilot 1/7/9 同款沉默跳过回归）：** 两个门禁脚本钉死旧路径必须改——`check-script-exec-policy.py` 的 `RUNNER = .../"script_runner.rs"`（line 35，固定路径，`read()` 缺失返回 `""` → 静默跳过）改 `src-tauri/src/capabilities/script/script_runner.rs`；`check-command-domain-policy.py` 的 `src-tauri/src/script_runner.rs`（line 423 文件清单 + 443 dict，双处，`read_text() if exists() else ""`）改同路径。两脚本自测（23 坏 + 1 好 / 19 坏 + 2 良性）与默认扫描均 PASS，且真实文件被扫（无静默跳过）。
- 门禁：`cargo check` 0 新增告警（仅 grid_process.rs 2 条既有 dead_code）；`cargo test capabilities::script` 58/58（含进程组取消/超时/子树击杀）；`cargo test` 全量 462/0（2 ignored 既有）；`check-script-exec-policy.py` 默认扫描 PASS + 自测（23 码位）；`check-command-domain-policy.py` 默认扫描 PASS + 自测；`npm run build` OK。
- 既有 FAIL（非本批回归，矩阵 §2.4 LEGACY_MIXED_MODULE 残核）：`UI_CHECK` 前端 fixed 浮层、`NATIVE-02` bridge.rs 跨能力直调。
- 未触碰：TS 侧 `src/capabilities/agent/` WIP（unstaged，隔离）；`phantom-yili` 仓库（DO_NOT_TOUCH）。

### 8.14 Native Pilot 11 — `terminal`（CAPABILITY_NATIVE，PTY 内核）
- 迁移：`src-tauri/src/terminal.rs` → `src-tauri/src/capabilities/terminal/terminal.rs`（`mod.rs` + `README.md`）。
- **§7 解耦评估结论**：`terminal.rs` 经 grep 确认**不含 `#[tauri::command]`**（终端命令 `term_spawn_channel`/`terminal_resize`/`terminal_send`/`terminal_kill` 注册在 `bridge.rs` 经 `generate_handler!`，bridge.rs 以 `pub use crate::terminal::{TermInfo, TerminalSession}` 再导出）；grep 确认 terminal.rs **零 `bridge::AppState` 耦合**（无 `app.state`/`State<`/`bridge::AppState`）—— 无 AppState 字段、无 hub 债务。仅有 2 处 `crate::terminal::` 调用点（全在 bridge.rs：`ChannelSink`/`EventSink`/`TermInfo`/`TerminalSession`）。本模块为**纯 PTY / 输出管道内核**，owner 一致 = terminal 能力，沿用 pure-module Pilot 模式，无需逐 command 拆 owner；命令体 bridge 分解留待 §8 末段。
- `main.rs`：删顶层 `mod terminal;`（line 29）；`mod capabilities` 内新增 `pub mod terminal;`（新增 `capabilities/terminal/mod.rs` 暴露 `pub mod terminal;`）；顶部加 re-export shim `pub use crate::capabilities::terminal::terminal;`（既有 2 处 `crate::terminal::` 调用点无需逐处改写即解析）。
- **关键修复（Pilot 1/7/9/10 同款沉默跳过回归）：** `scripts/check-terminal-policy.py` 的 `"terminal_rs": read("src-tauri/src/terminal.rs")`（line 202，`read()` 缺失返回 `""` → 静默跳过）改 `src-tauri/src/capabilities/terminal/terminal.rs`。改后默认扫描与自测均 PASS，且 30 个坏样本（含变异）全检出（证明文件被真实扫描、无静默跳过）。
- 门禁：`cargo check` 0 新增告警（仅 grid_process.rs 2 条既有 dead_code）；`cargo test capabilities::terminal` 8/8（PTY 真实输出 / resize / 进程组击杀）；`cargo test` 全量 462/0（2 ignored 既有）；`check-terminal-policy.py` 默认扫描 PASS + 自测（好零违规 + 30 坏全检）；`npm run build` OK。
- 既有 FAIL（非本批回归，矩阵 §2.4 LEGACY_MIXED_MODULE 残核）：`UI_CHECK` 前端 fixed 浮层、`NATIVE-02` bridge.rs 跨能力直调。
- 未触碰：TS 侧 `src/capabilities/agent/` WIP（unstaged，隔离）；`phantom-yili` 仓库（DO_NOT_TOUCH）。
