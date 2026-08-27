# 后续需求 TODO 列表

> 整理自 2026-08-26 的需求讨论。每项标注：目标、范围边界、与现有架构的关系、建议切入点。
> 排期优先级为初步建议（P0 最高），可按实际节奏调整。

---

## 1. 脚本库中心：快速调用各种 shell 脚本  `P1`

**目标**：用户在控制台内维护一套 shell 脚本库，点击即可在本地/目标环境快速执行，结果回显。

**范围**
- 脚本库管理（新增 / 编辑 / 删除 / 分类标签）。
- 一键执行（选择脚本 → 跑 → 输出捕获）。
- 参数化脚本（占位符 `${ARG}` 在执行前弹窗填值）。
- 脚本来源：本地文件 + 可选的内置常用模板（备份、清理、部署等）。

**与现有架构关系**
- 现有 `bridge.rs` 已具备命令执行能力（`request_sync` 用 `std::thread::spawn` 后台跑）。可复用「后台线程 + 事件回传前端」模式做脚本执行，避免 UI 阻塞。**注意**：高吞吐的 mpsc+pump task 解耦是 #9（M3）的改造，#1 在 M2 先行，首期不依赖该改造。
- 输出回显可借鉴终端改造的 `Channel` 推送模型（详见 `fileterm-study/终端改造-详细设计.md` §3.2），避免大输出卡死。

**建议切入点**
1. `domain.rs` 加 `ScriptMeta`（id / name / category / path / args）。
2. `workspace.rs` 持久化脚本库（与成果库同目录）。
3. `bridge.rs` 加 `run_script(meta, args) -> Channel<LogLine>`。
4. 前端 `ActivityBar` 加「脚本库」面板。

---

## 2. 小工具框架：内置 + 可扩展的 HTML 小工具  `P1`

**目标**：控制台内置一组离线可用的 HTML 小工具（JSON 格式化、Base64、时间戳、正则、Cron、Markdown 预览等），并做成**可扩展的小工具框架**——程序后续支持各种小工具（内置种子 + 用户自定义丢进目录即加载）。

**范围**
- 工具以独立单文件 HTML（自包含 CSS/JS，数据不出本机）挂在控制台「工具箱」，类似"应用"。
- **内置种子工具（文件已落盘 `src-tauri/src/tools/`，但加载框架未实现）**：**⚠️ 现状（2026-08-27 复核）**：5 个 HTML 仅物理落盘，`build.rs` 仍极简无 `include_dir!`、`tauri.conf.json` 无 `bundle.resources`、`bridge.rs` 无 `list_tools`，故当前应用**无法加载/打开这些工具**。须待 #2 的 `list_tools`+子 webview 打开+嵌入机制三处落地后才可用，不要误以为"已落位即可用"。
  - `cron-tool.html`：Cron 生成/检测/未来执行时间推算（纯前端 JS + localStorage）。
  - `regex-tool.html`：Regex Forge 正则工坊——自动识别 Java/Python/Rust/Node/IDEA/VSCode 格式、高亮匹配、多语言转换。
  - `json-tool.html`：JSON 格式化（2 空格）/ 压缩 / 校验 / 键排序（纯前端，自包含）。
  - `base64-tool.html`：Base64 文本↔编码、UTF-8 安全编码、URL-Safe 变体。
  - `timestamp-tool.html`：时间戳↔日期双向转换（秒/毫秒、本地/UTC/ISO 8601、实时时钟）。
- **扩展机制**：用户把自写的单文件 HTML 丢进 `workspace/tools/`（或 `src/tools/`）即被工具箱动态枚举加载，无需改代码。
- 工具可通过 `bridge` 暴露的本地能力（读写文件、调脚本）做增强（可选）。

**与现有架构关系**
- 复用现有 browser-tabs 的 `WebviewBuilder` 子 webview 加载本地 `file://` 或 `asset:` 资源的能力。**注意**：内置工具在打包后不在运行目录，需用 `include_dir!` 嵌入或 `tauri.conf.json` 的 `bundle.resources` 声明；用户工具 `workspace/tools/` 走运行期读盘。
- 工具清单 = 内置 `src/tools/` + 用户 `workspace/tools/` 合并枚举；与 #15 插件系统协同（工具可包装为插件）。
- 单文件 HTML 不引入构建步骤，保持"丢进去就能用"。

**建议切入点**
1. `src-tauri/src/tools/` 内置种子工具（cron-tool / regex-tool 已落位）。
2. `bridge.rs` 加 `list_tools() -> Vec<ToolMeta>`（扫描内置+用户目录，进 ACL）；打开走子 webview/iframe 加载。
3. 主控制台加「工具箱」入口，动态枚举并渲染卡片。
4. `ToolMeta`（name / path / builtin: bool / icon）登记，新增工具无需改前端硬编码。

---

## 3. 验证：关闭窗口能否正确释放资源  `P0`（待验证，非纯开发）

**目标**：确认关闭窗口（主窗口 / 子 webview / 终端页签）时，相关资源被彻底回收，无泄漏（僵尸进程、孤儿 PTY、文件句柄、内存堆积）。

**待验证清单**
- [ ] 关闭宫格子 webview（`grid-0..N`）：GTK 子窗口 + WebView2/WebKit 实例是否销毁？
- [ ] 关闭终端页签：PTY master/slave、子 shell 进程树是否被 kill（参考 `bridge.rs` 的 `term_kill` / child drop 逻辑）？
- [ ] 关闭主窗口：整个 App 进程是否干净退出，还是残留后台线程 / channel 监听？
- [ ] 连续开关 N 次后内存是否线性增长（泄漏）？

**与现有架构关系**
- 现有 `grid_process.rs` 的 `GridChildHandle`（`pub struct`，line 75）含 `pub child: Child`（line 77），**无 `impl Drop`**；多进程改造（2026-08-24 落地）后关闭逻辑收口到 `GridProcessManager` 的 `kill_child`（line 619）/`restart`（line 631）/`shutdown_all`（line 680）显式 `child.kill()`（原 `close_grid`/`close_one` 已随多进程改造删除/改名）。需补"单一真退出入口"统一收口，避免残留。
- 终端改造详细设计里已规划 worker 线程退出机制（reader EOF → 线程自然结束）。
- 本项目日志系统有 `session-*.log`，可借此观察关闭前后的进程/句柄变化。

**建议切入点**
1. 写一键验证脚本：`lsof` / `ps --ppid` / `cat /proc/<pid>/status` 对比开关前后。
2. 在 `bridge.rs` 的窗口 `on_window_close` 钩子里补显式清理（kill 子进程树、drop channel）。
3. 若发现泄漏，定位到具体资源（PTY 还是 webview）再修。

**注意**：此项先验证再开发。当前 `run-gui.sh` 启动的应用可直接用来复现开关窗口场景。

---

## 4. 常用 Linux 命令库：快速执行常用命令  `P1`

**目标**：把高频 Linux 命令做成「命令库」，点击/搜索即执行，不用手敲。与脚本库（#1）的差异是：命令库是**单条命令片段**（如 `df -h`、`systemctl status`、`du -sh *`），脚本库是**多行文件**。

**范围**
- 内置常用命令分类（系统 / 网络 / 磁盘 / 进程 / 文本处理）。
- 搜索 + 收藏常用命令。
- 一键执行 + 结果回显（复用 #1 的执行/回显通道）。

**建议切入点**
- 与 #1 共用 `run_command(line, args)` 后端能力，前端做"命令片段库"独立面板。

---

## 5. Git 等版本控制功能  `P0`

**目标**：在控制台内提供 Git 可视化操作（不止现有 `request_sync / confirm_sync` 推送闸门），覆盖日常提交工作流。

**范围**
- 仓库状态可视化（改动文件、分支、未推/未拉）。
- 提交（stage / commit / push）、拉取、分支切换。
- diff 预览、历史查看。
- 多仓库管理（现有 `RepoConfig` 已支持多仓库配置）。

**与现有架构关系**
- 现有 `sync.rs` 已实现 git2 三方合并、`request_sync` / `confirm_sync` 闸门 + 审计（`audit.json`）。
- 安全红线：token 只存 keyring，推送需确认——新功能必须遵守，不能绕过确认闸门。

**建议切入点**
1. `bridge.rs` 加 `git_status(repo) / git_diff(repo, file) / git_commit(repo, msg) / git_branch_*`。
2. 前端加「Git」面板（复用成果库目录视图）。
3. 命令注册后**必须同步加进 `permissions/default-commands.toml`**（本项目历史坑：ACL 白名单缺命令会被静默拒绝，详见 `未完成的任务.md`）。

---

## 6. 数据库等功能  `P1`

**目标**：在控制台内连接并操作数据库（至少 SQLite / MySQL / PostgreSQL），做查询、表浏览、结果导出到成果库。

**范围**
- 连接管理（多数据源、凭据存 keyring，复用 `keyring_store.rs`）。
- SQL 执行 + 结果表格展示。
- 查询结果一键存入本地成果库（带溯源）。
- 只读优先，写操作需二次确认（对齐现有安全闸门理念）。

**与现有架构关系**
- 凭据隔离可复用 `keyring_store.rs`（系统密钥库）。
- 审计可复用 `audit.json` 写入规范。
- 结果导出复用 `workspace.rs` 成果落盘。

**建议切入点**
1. `Cargo.toml` 引 `sqlx`（或 `rusqlite` + `mysql` + `postgres`）。
2. `bridge.rs` 加 `db_query(conn_id, sql) -> rows`。
3. 前端加「数据库」面板（连接表单 + SQL 编辑器 + 结果表）。

---

## 7. A2P / A2A 等 Agent 协议  `P2`

**目标**：支持 Agent-to-Protocol / Agent-to-Agent 通信协议，让本应用能作为 Agent 节点对外提供能力或与其他 Agent 协作。

**范围**（待澄清，先列可能方向）
- A2A：接收/发起 Agent 间任务委派、消息路由。
- A2P：把本应用的本地能力（脚本执行、git、浏览器操作）以协议接口暴露给外部 Agent 调用。
- 鉴权、会话、能力声明（capability manifest）。

**与现有架构关系**
- 现有 `BridgeCall` 权限内核可演进为「能力清单 + 调用闸门」，天然适配 A2A/A2P 的能力暴露与鉴权。
- 已有审计机制可记录跨 Agent 调用。

**建议切入点**
- 先定义本应用对外暴露的能力清单（复用 #1~#6 的命令），再做协议层（HTTP/gRPC/消息总线）。此项偏架构，建议放在 #1~#6 稳定后再做。

---

## 8. 浏览器收藏 + 设为默认浏览器 + 增加图标  `P0`

**目标**：完善浏览器体验——收藏夹、系统级默认浏览器绑定、桌面/菜单图标。

**范围**
- 收藏功能：收藏当前页（标题 + URL + 缩略图/分类），收藏夹面板管理，点击即开。
- 设为默认浏览器：注册 `xdg-settings set default-web-browser`（Linux）/ 系统协议处理器（`http/https` scheme handler），让外部链接用本应用打开。
- 增加图标：桌面快捷方式、开始菜单项、文件管理器关联；打磨 `src-tauri/icons/` 全套尺寸（32/128/512 + 平台特定 icns/ico）。

**与现有架构关系**
- 现有 `browser-tabs` 插件管理页签，`BrowserHost` 负责 url 加载——收藏可挂在其导航完成事件 `tab-navigated` 上。
- 图标已有 512 圆角 `icon.png`（README 已说明），需补齐其余尺寸与安装脚本。
- 默认浏览器涉及 `tauri.conf.json` 的 `protocol` / 系统 `xdg` 注册，可能要加一个 `setup` 命令或安装后钩子。

**建议切入点**
1. `bridge.rs` 加 `add_bookmark(url, title) / list_bookmarks() / remove_bookmark()`（+ 进 ACL）。
2. 前端 `BrowserHost` 加地址栏「收藏」按钮 + 收藏夹侧栏。
3. 默认浏览器：`setup-linux.sh` 里加 `xdg-settings` 注册 + `*.desktop` 文件；macOS/Windows 用 Tauri 的 `setAsDefault` 或打包配置。
4. 图标：用现有 `icon.png` 生成全套尺寸（`npm run tauri icon` 或 `tauri icon`）。

---

## 9. 借鉴/集成开源 fileterm 终端项目  `P1`

**目标**：吸收开源项目 **fileterm**（已在 `fileterm-study/` 做过 V1~V5 五轮源码级审核与借鉴分析）的终端能力，落地到本应用的终端/控制台。

**已有基础（来自 `fileterm-study/` 分析结论）**
- Tauri v2 `Channel` IPC：前端从 `@tauri-apps/api/core` 导入；底层 `Arc<ChannelInner>`，可跨 `std::thread::spawn` 发送（`文件` `终端改造-详细设计.md` §3.2）。
- `portable_pty 0.8.1`：`MasterPty::resize(&self, PtySize)`（非 `set_size`）、`try_clone_reader()`、`take_writer()` 已实证。
- PTY 输出推送模型：用 `mpsc(128) + 独立 pump task` 解耦，避免 `Channel::send` 同步阻塞（本项目当前为直接 `send`，单终端低吞吐可接受）。
- 退避重试：`TERMINAL_DATA_RETRY_MAX_BACKOFF_MS=30_000` 指数退避（本项目暂无，建议补）。
- worker 线程退出机制：`term_kill` drop child → reader EOF → 线程自然结束。
- 资源监控维度差异：本项目 `resource_stats` 为进程树 RSS 常驻监控（StatusBar/ActivityBar），fileterm 为全机 CPU%/内存%。

**范围**
- 终端多实例 / 页签化（复用现有终端页签 + `bridge.rs` 的 `term_*` 命令）。
- 引入 mpsc + pump task 解耦，支撑高吞吐输出不卡 UI。
- 补 `Channel` 跨线程透传 + 断线退避重试。
- 把 fileterm 的可用特性（如临时历史 `TEMPORARY_HISTORY_LIMIT=40`、resize 静默窗口 `TERMINAL_RESIZE_OUTPUT_QUIET_MS`）按本项目需要采纳。

**建议切入点**
1. 先读 `fileterm-study/终端改造-详细设计.md`（V5 标记，含 §0 依赖溯源表、§3.2 跨线程+阻塞双注、worker 退出机制、§6 风险表）。
2. `bridge.rs` 终端输出改为 mpsc + pump task；`term_kill` 确保 child 进程树回收（与 #3 资源释放联动）。
3. 新命令注册后同步进 `permissions/default-commands.toml`。

---

## 10. 支持图片展示  `P1`

**目标**：在控制台/成果库/工具箱内支持图片的展示与预览（当前成果以 `.md`/`.html` 文本为主，缺图片渲染能力）。

**范围**
- 成果库图片预览：右键保存的图文成果中的图片、本地图片文件直接预览。
- 画廊/缩略图：多图浏览、网格视图、点击放大。
- 图片来源：本地文件（`workspace/` 下）、网页选区保真的图片（现有 `injected/collect.js` 已绝对化 `img` 链接）。
- 拖拽/粘贴上传图片到成果库或聊天/笔记。

**与现有架构关系**
- 现有 `injected/collect.js` 已做富文本选区保真（克隆 DOM、内联图片链接），图片展示可消费其产物。
- 成果库持久化在 `workspace.rs`，图片可作为 artifact 的附件（需扩展 `domain.rs` 的 `Artifact` 结构，增加 `images: Vec<ImageRef>`）。
- 渲染复用前端 Vue 组件 + 静态资源服务（`asset:` 协议或本地路径加载）。

**建议切入点**
1. `domain.rs` 的 `Artifact` 增加图片附件字段；`workspace.rs` 落盘时同步存图片到 `workspace/<id>/images/`。
2. 前端加图片预览组件（支持缩放、灯箱）。
3. 成果列表/详情页支持图文混排渲染。

---

## 11. 定时任务调度  `P1`

**目标**：在控制台内提供定时任务能力，让脚本/命令/工具按 cron 或间隔周期自动执行，无需人工点击。

**范围**
- 任务定义：名称、类型（执行脚本 #1 / 运行命令片段 #4 / 调用 HTML 工具 #2 / 自定义 shell）、调度表达式（cron 或 `every Ns/min/h`）、启用开关、上次/下次运行时间。
- 调度引擎：应用常驻后台，到点触发任务，捕获输出并落日志。
- 结果可回写成果库（#10）或仅记运行历史。
- 持久化任务列表（与脚本库同目录 `workspace/tasks.yaml` 或 `tasks.json`）。

**与现有架构关系**
- 执行通道直接复用 #1 的 `run_script` / #4 的 `run_command`（`Channel<LogLine>` 回显模型），定时任务只是"触发方"。
- 调度器挂在 `main.rs` 的 `setup` 钩子里（`tokio::spawn` 常驻 task 循环扫描 next_run），与 #3 单一真退出入口协同（退出时取消所有 timer）。
- 每个任务执行仍走审计 `audit.json`（安全红线）+ 凭据 `KeyringStore`。

**建议切入点**
1. 新增 `scheduler.rs`：`TaskDef` 领域模型 + `Scheduler`（`tokio::time::sleep` 轮询或 `tokio_cron_scheduler` crate），`setup` 时 `tokio::spawn` 启动。
2. `bridge.rs` 加 `task_list() / task_add(def) / task_update(def) / task_remove(id) / task_run_now(id)`（**必须进 `permissions/default-commands.toml`**）。
3. `domain.rs` 加 `TaskDef`（cron / command / enabled / last_run / next_run）。
4. 前端加「定时任务」面板（CRUD + 开关 + 运行历史）。
5. 退出收口：在 #3 的 `request_app_exit` 单一入口里 `scheduler.abort()`。

---

## 12. Agent 与 Skill 生态  `P2`

**目标**：在控制台内构建可扩展的 Agent 与 Skill 生态——既能内置/外接 Agent 能力，又能以"Skill（技能包）"形式沉淀可复用能力，供用户或 Agent 调用。

**范围**
- **Agent 生态**：
  - 内置 Agent 宿主：应用内可承载一个本地 Agent（对接 LLM API 或外部 CLI Agent 方言，借鉴 dbx `ai_cli_agent.rs` 的 Codex/ClaudeCode/CodeBuddy/Qoder/OpenCode/Cursor/Grok 多方言归一思路）。
  - Agent 可调用的"能力清单"= 本项目已有命令（#1 脚本 / #4 命令 / #5 git / #6 数据库 / #8 浏览器 / #11 定时任务）。
  - 与 #7 A2P/A2A 协同：Agent 既可作为 A2A 节点，也可通过 MCP（#7 内嵌 rmcp）把能力暴露给外部 Agent。
- **Skill 生态**：
  - Skill = 可复用能力单元（类似 VibeKit 的 SKILL.md 升级版：带元数据 + 执行脚本 + 触发条件）。
  - 来源：内置常用 skill（如"一键清理日志""备份工作区"= 复用 #1 脚本）、用户自定义 skill 目录（`workspace/skills/` 下 Markdown/YAML 描述 + 执行体）、可选从远程拉取。
  - 触发方式：手动调用 / Agent 自动匹配 / 定时任务（#11）挂载。
- **记忆/上下文**：借鉴 dbx `agent_kv.rs` 做 Agent 记忆层（KV 持久化），注意本项目有独立成果库，`agent_kv` 仅作 AI 上下文，不混为用户成果。

**与现有架构关系**
- 能力底层全部复用现有命令（#1~#11），Agent/Skill 是"编排层"而非"新执行层"。
- 安全红线全覆盖：Agent/Skill 执行任何动作（执行/出网/落盘）均走 `audit.json` + 确认闸门 + `KeyringStore`，绝不开放绕过通道（对应 dbx 红线"命令不加 ACL"不可借鉴）。
- 与 #7 共用 `phantom-core` 下沉 + 内嵌 `rmcp`，Skill 可同时注册为 MCP 工具。

**建议切入点**
1. `domain.rs` 加 `AgentDef` / `SkillDef`（name / description / trigger / exec / acl_level）。
2. `bridge.rs` 加 `skill_list() / skill_install(path) / skill_run(id, args) / agent_chat(msg) -> Channel<Token>`（**进 ACL**）。
3. 新增 `agent_runtime.rs` / `skill_runtime.rs`：skill 执行沙盒（复用 #1 执行通道）、agent 对话循环（流式 token 回传用 Channel）。
4. 前端加「Agent」面板（对话）+「Skill 市场/管理」面板。
5. 与 #7 协议层共用护栏（McpGlobalPolicy + fail-closed）。

---

## 13. 知识图谱  `P2`

**目标**：在控制台内构建可检索、可可视化的知识图谱——把成果库（#10）、脚本（#1）、命令（#4）、Skill（#12）、网页收藏（#8）等散落资产，抽取为「实体—关系—属性」的图谱，支撑关联浏览、影响分析、智能问答上下文。

**范围**
- **实体来源**：成果库 Artifact（#10）、脚本 ScriptMeta（#1）、命令片段（#4）、Skill（#12）、书签 Bookmark（#8）、Git 仓库/分支（#5）。
- **关系类型**：`引用`/`属于分类`/`由Agent生成`/`定时触发`/`同仓库`/`相似` 等（可扩展）。
- **抽取方式**（两阶段）：
  - 轻量期：基于元数据 + 正则/规则抽取（如 Artifact 内链、脚本引用的命令、Skill 触发的任务），纯本地、离线。
  - 智能期：借助 #12 Agent 做语义抽取（实体识别 + 关系推断），结果存图谱（标注"AI生成"，可人工校正）。
- **存储**：本地图存储——首期用 SQLite + 邻接表/JSON 存图（复用 #6 的 sqlx 与 Keyring 思路，不引 Neo4j 等重依赖）；可选导出 GraphML/JSON。
- **可视化与查询**：前端图谱视图（力导向图，节点=实体、边=关系）+ 图查询（"X 影响了哪些成果""Y 被哪些脚本依赖"）。
- **消费方**：#12 Agent 问答时把子图作为上下文注入（RAG 式），提升回答准确性。

**与现有架构关系**
- 实体全部来自既有领域模型（`domain.rs` 的 `Artifact`/`ScriptMeta`/`RepoConfig`/`Bookmark`/`TaskDef`/`SkillDef`），图谱是**派生索引层**，不重复存储主数据。
- 复用 #6 的数据库连接与查询能力；复用 #10 的图片/富文本展示做节点预览。
- 智能抽取复用 #12 的 Agent 能力与 `agent_kv` 记忆层。
- 安全红线：图谱抽取若触发执行/出网动作（如 Agent 抽取需调 LLM），走 `audit.json` + 确认闸门 + `KeyringStore`。

**建议切入点**
1. `domain.rs` 加 `GraphNode`（id / kind / ref_id / label / props）/ `GraphEdge`（src / dst / relation / weight / source=rule|ai）。
2. `graph.rs`：图谱构建器（扫描既有领域对象 → 产出 nodes/edges，持久化到 `workspace/graph.db` 或复用 #6 库）、查询 API（`neighbors(id)` / `paths(a,b)` / `by_kind(kind)`）。
3. `bridge.rs` 加 `graph_build() / graph_query(q) / graph_neighbors(id) / graph_export()`（**进 ACL**）。
4. 前端加「知识图谱」面板（力导向可视化 + 查询框 + 节点详情）。
5. 与 #12 协同：Agent 问答前 `graph_query` 取子图注入上下文。

---

## 14. 浏览器会话存档：请求/资源可见 + 关闭保存删除  `P1`

**目标**：浏览器访问的内容在本地全程可见——能看到页面发起的所有网络请求与加载资源（JS/CSS/图片/XHR/文档），关闭页面时弹出「保存会话 / 删除会话」选择，保存后本地可回看该次浏览的完整痕迹（含请求列表、资源、截图/快照）。

**范围**
- **请求与资源可见**：监听每个 tab 的网络请求（`on_web_resource_response_received` / WebKit `WebResource` 事件），汇总为请求列表（URL / 方法 / 状态码 / 类型 / 大小 / 耗时）。资源可按类型（文档/脚本/样式/图片/媒体/XHR）筛选与预览（图片直接看、文本可查）。
- **本地存档**：关闭 tab 或主动「保存会话」时，把该会话的请求清单 + 关键资源 + 页面快照落盘到 `workspace/sessions/<id>/`（首期用 `workspace.rs` 文件落盘 + #10 图片/富文本渲染；#6 数据库落地后可升级为库存储，注意 #6 在 M4 晚于本需求的 M1，故首期不强依赖 #6）。
- **关闭 UX**：关闭页签/窗口时弹「保存 / 删除」选择（不可静默丢）；已保存会话在「历史会话」面板可回看、可删除。
- **隐私红线**：存档默认仅本地，不自动出网；存档落盘走 `audit.json`，资源含敏感内容时标注。

**与现有架构关系**
- 当前 `tauri-plugin-browser-tabs` 仅有 `on_navigation` / `on_new_window` 拦截（已核查 `commands.rs:57/74`），**无请求层监听**——需新增 `on_web_resource_response_received` 钩子（Tauri v2 支持；Linux WebKit 后端可用），把事件经 `app.emit` 推前端。
- 前端 `collect.js` / `resources.js` 已在做 DOM 级收集，可扩展为"请求层 + DOM 层"双源。
- 存档复用 #10 的图片/富文本渲染 + `workspace.rs` 落盘 + #6 的存储/查询。
- 与 #8 收藏互补：收藏=主动存 URL，会话存档=被动存完整浏览痕迹。

**建议切入点**
1. `tauri-plugin-browser-tabs` 的 `WebviewBuilder` 加 `on_web_resource_response_received`（emit `ResourceReceived` 事件）。
2. `bridge.rs` 加 `session_save(tab_id) / session_list() / session_get(id) / session_delete(id) / session_export(id)`（**进 ACL**）。
3. `domain.rs` 加 `BrowserSession`（tab_id / url / title / requests: Vec<ResourceReq> / snapshot / created_at）。
4. 前端加「会话存档」面板（请求瀑布 + 资源预览 + 关闭弹窗保存/删除）。
5. 与 #3 资源释放协同：关闭 tab 时先 flush 会话缓冲再 kill 子 webview。

---

## 15. 插件系统  `P2`

**目标**：提供可扩展的插件机制，让脚本（#1）、HTML 工具（#2）、Skill（#12）、浏览器会话处理器等能以"插件"形式注册、启用、配置，形成生态。

**范围**
- **插件模型**：`PluginManifest`（name / version / kind=script|tool|skill|session-handler / entry / permissions / config schema）。
- **加载与隔离**：从 `workspace/plugins/` 扫描加载（Markdown/YAML manifest + 执行体），复用 #1 执行通道与 #12 skill_runtime 沙盒。
- **能力注册**：插件可声明要暴露的命令/工具（自动并入 #7 MCP 工具集与 #12 Skill 列表）。
- **权限分级**：插件声明 `acl_level`，安装需确认（对标 Skill 红线），越权动作走确认闸门。
- **管理 UI**：「插件市场/管理」面板（安装/启用/禁用/卸载/配置）。

**与现有架构关系**
- 完全复用既有能力层（#1 脚本 / #2 工具 / #12 Skill / #14 会话），插件系统只是**统一编排与生命周期外壳**，不新建执行逻辑。
- 与 #7 共用 `phantom-core` 下沉 + 内嵌 rmcp：插件能力即 MCP 工具。
- 安全红线：插件执行任何动作走 `audit.json` + 确认闸门 + `KeyringStore`（对标 dbx 红线"命令不加 ACL"不可借鉴）。

**建议切入点**
1. `domain.rs` 加 `PluginManifest`。
2. `bridge.rs` 加 `plugin_list() / plugin_install(path) / plugin_enable(id) / plugin_disable(id) / plugin_uninstall(id)`（**进 ACL**）。
3. 新增 `plugin_runtime.rs`：manifest 解析 + 生命周期（load/unload/reload）+ 能力注册到 #7/#12。
4. 前端「插件管理」面板。
5. 与 #12 共用护栏（McpGlobalPolicy + fail-closed）。

---

## 优先级总览与里程碑

> 以下为基于源码核查的初步排期（单人节奏估算，含联调/自测，不含需求反复）。

### 优先级表
| ID | 需求 | 优先级 | 类型 | 依赖 |
|----|------|--------|------|------|
| 3  | 关闭窗口资源释放验证 | P0 | 验证+修复 | 无（先验证） |
| 5  | Git 功能 | P0 | 开发 | 现有 sync.rs |
| 8  | 浏览器收藏+默认浏览器+图标 | P0 | 开发 | browser-tabs |
| 14 | 浏览器会话存档(请求/资源可见+关闭保存删除) | P1 | 开发 | browser-tabs / #10 |
| 1  | 脚本库中心 | P1 | 开发 | 执行通道 |
| 2  | HTML 工具 | P1 | 开发 | 子 webview 加载 |
| 4  | Linux 命令库 | P1 | 开发 | 复用 #1 通道 |
| 6  | 数据库功能 | P1 | 开发 | keyring/审计 |
| 9  | 借鉴 fileterm 终端项目 | P1 | 开发 | 现有 term_* |
| 10 | 支持图片展示 | P1 | 开发 | workspace/Artifact |
| 11 | 定时任务调度 | P1 | 开发 | 复用 #1/#4 执行通道 |
| 7  | A2P/A2A 协议 | P2 | 架构 | #1~#6 稳定后 |
| 12 | Agent 与 Skill 生态 | P2 | 生态 | #1/#4/#6/#7/#11 稳定后 |
| 13 | 知识图谱 | P2 | 生态 | #1/#4/#8/#10/#12 稳定后 |
| 15 | 插件系统 | P2 | 生态 | #1/#2/#12/#14 稳定后 |

### 里程碑与工时估算（约 11~15 周，单人）
| 里程碑 | 覆盖 | 子任务数 | 估时 | 交付标准 |
|--------|------|----------|------|----------|
| **M0 安全基线** | #3 + 基线评估(M0-0) | 7（M0-0~6） | 0.5~0.7周 | 性能+可读性基线归档（见 `logs/baseline-2026-08-27.md`）、验证脚本+报告、单一退出入口、子 webview 回收、崩溃恢复、gtk/wry 死亡依赖清理 |
| **M1 浏览器与版本控制** | #8, #14, #5 | 9（M1-1~9） | 2.5周 | 收藏/默认浏览器/图标；会话请求可见+关闭保存删除；Git 面板 |
| **M2 本地资产与执行** | #1, #4, #2, #10 | 9（M2-1~9） | 3周 | 脚本库/命令库/工具箱(5内置工具)/图片预览 |
| **M3 终端增强** | #9 | 4（M3-1~4） | 1周 | mpsc+pump、term_kill、退避重试 |
| **M4 数据与调度** | #6, #11 | 8（M4-1~8） | 2周 | 数据库面板+护栏；定时任务调度 |
| **M5 协议与智能生态** | #7, #12, #13, #15 | 12（M5-1~12） | 3周 | 内嵌 rmcp MCP；Agent+Skill；知识图谱；插件系统 |
| 缓冲(测试/联调/需求微调) | GEN-test | — | 1~1.5周 | 前端 vitest + 契约测试 |

> 注：并行可压缩——M1~M3 中无强依赖的项可由多人分担；上述为单人串行下限。#14 因需新增 WebKit 请求拦截钩子（Linux 后端验证），估时偏保守（+0.5周已含在 M1）。子任务共 49 项（M0:7/M1:9/M2:9/M3:4/M4:8/M5:12），与 `详细设计与实施计划.md` 里程碑表一致。M0-0 基线评估约 +0.2 周（一次性，后续每 Phase 仅对比不改基线）。

## 通用注意事项（来自历史教训）
- **每新增一个 `#[tauri::command]`，必须同步加进 `permissions/default-commands.toml` 的 `commands.allow`**（主窗口调用；子 webview 另需 `permissions/remote-collect.toml` + `capabilities/browser-remote.json` 引用），否则被 ACL 静默拒绝（终端无日志、前端 catch 吞错，极难排查）。当前已注册 **59 个命令**（2026-08-27 实测，非早年文档写的 63）。
- 任何出网/落盘/执行动作写入 `audit.json`（安全红线）。
- 凭据只存系统密钥库（keyring），不出现在前端/网页 JS。
- 后台执行用 `std::thread::spawn`，避免 UI 阻塞；结果通过 `app.emit` 事件回传。

> **⚠️ 现状勘误（2026-08-27 源码复核）**：上方"必须同步加进 `permissions/default-commands.toml`"仍为硬规则，但**命令总数已不是早年文档写的 63，实测为 59 个**（见 `详细设计与实施计划.md` §0.1）。新增命令 PR 务必同时改该 toml，否则被 ACL 静默拒绝。

> **⚠️ 性能/可读性基线前置（2026-08-27 新增，同日已实测采集）**：当前项目**零性能基准、零 clippy/lint 门槛**（`Cargo.toml` 无 `[[bench]]`/`clippy`，`package.json` 无 test/lint/bench，PROJECT-RULES 仅有功能正确版本基准）。✅ 可读性基线已于 2026-08-27 实测：`cargo clippy --all-targets` 唯一 **13 warning**（插件另有 1）、前端 `npm run build` 主 JS **505KB**/dist **548KB**（Vite 告警 >500KB 需 manualChunks 代码分割），已归档 `logs/baseline-2026-08-27.md`。在动 M1~M5 任何功能代码前，必须先按 `详细设计与实施计划.md` 的 **M0-0** 采集并归档性能+可读性基线（`logs/baseline-<date>.md`）。每阶段完成须对比基线，硬门槛：性能回退 >10% 或 `cargo clippy` 新增 warning 须显式说明方可合入。否则多进程/mpsc/数据库/Agent 流式等改造会**静默劣化且无法归因**——这正是你担心的"改完系统崩溃、性能降低却查不出原因"。无基线对比的 PR 不准合入。

---

## 11. 待执行源码修复项（审核发现，尚未落地，等确认）

> 以下三项为文档审核中确认的真实缺陷/待办，**源码尚未改动**。已分别对齐 `详细设计与实施计划.md` 的 M0 子任务编号，落地后可勾除。

### 11.1 M0-1 写资源验证脚本 `scripts/verify-resources.sh`（补性能基线）
- **现状**：M0-0 可读性基线已采集，但性能基线因该脚本未写 + `target/release` 未构建而空缺（`logs/baseline-2026-08-27.md` 已标"待补"）。
- **动作**：新增 `scripts/verify-resources.sh`，开关窗口前后对比 `ps`/`lsof`/`/proc/<pid>/status`；终端页签关闭后确认 PTY 进程树已 kill；连续开关 N 次记录 VmRSS 斜率。
- **后续**：release 构建后补"冷启动耗时"基线，固化进 `scripts/baseline-check.sh` 做 pre-merge 门禁。

### 11.2 M0-6 清理 `gtk`/`wry` 死亡依赖（来自审核报告高危25，P0）
- **现状**：`Cargo.toml` 第 25-30 行注释声称 gtk/wry 用于"方案 C3"，实际项目走方案 D（`Window::add_child`）。经 2026-08-27 全仓核查 `gtk`/`wry` **crate API 零引用**（仅注释提及 WebKitGTK/wry 行为；`bridge.rs:1362` 的 `freedesktop_icons::default_theme_gtk()` 非 gtk crate 调用，删 `gtk = "0.18"` 不破坏它）。
- **动作**：删除 `gtk = "0.18"` 与 `wry = "0.55"` 两项依赖；删除前 `grep -rn "gtk::\|wry::"` 全仓 + `tauri-plugin-browser-tabs` 子 crate 确认零引用；预期缩小构建时间/二进制体积/依赖攻击面。

### 11.3 前端主 JS chunk 代码分割（manualChunks）
- **现状**：`npm run build` 主 JS `dist/assets/index-*.js` **505KB**（Vite 告警 >500KB 需代码分割）；`useBrowserStore.ts` 同时被静态 import 与动态 import，动态拆分失效。
- **动作**：在 `vite.config.ts` 用 `build.rollupOptions.output.manualChunks` 拆分 vendor/业务 chunk；解除 `useBrowserStore.ts` 静/动态混用；目标把主 chunk 压到 500KB 以内，避免触发基线体积涨幅 >15% 红线。
