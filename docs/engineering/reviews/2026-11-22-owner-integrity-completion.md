# BrowserOS Windows/Linux：Owner、数据恢复与执行契约收口

日期：2026-11-22（Asia/Shanghai）。

## 交付基线与执行边界

- 仓库：`code5m/browser-os`。
- 冻结基线分支：`fix/windows-document-runtime-contract-20261122`。
- 父提交：`6c00ac0bee63ca901aba6927d1e07859ca059a8c`。
- 父树：`06966ea4f7b06617afd8e78560b0e5b357b8997e`。
- 独立交付分支：`fix/windows-owner-integrity-complete-20261122`。
- 本文与全部修复放在同一个提交，没有分批 PR，没有修改基线分支或 master。
- 遵照用户要求，没有执行项目命令、测试、编译、构建、lint、格式化、校验、安装或 GUI，也没有触发工作流或合并。
- 新增和更新的回归定义均为 **NOT_RUN**。“已修复”在本文中表示源码改动已经交付，不表示编译、测试或 Windows 原生验收 PASS。
- 通过 GitHub Git 对象交付。分支直接指向包含 `[skip ci]` 的最终提交，不推送中间分支，不创建 PR，不修改工作流。

## 审查方法

沿真实调用链检查五个边界：用户看到的版本与执行版本是否相同；配置和凭据是否来自同一快照；异步结果是否仍属于当前文档或导航；失败后谁继续拥有资源；磁盘状态不明时是否仍会覆盖数据。

继续使用现有业务 Owner。保留 Rust 原生 CAS、跨平台文档身份、加密 journal、Windows SID/DACL、AppContainer、ConPTY、TaskRegistry 和平台 Adapter；没有另建 Monaco Runtime、Document Manager、工具执行器或文件写入引擎。

## 本轮 13 组问题及源码处理

| # | 已确认的问题 | 本次处理 |
|---|---|---|
| 1 | AI/Database 在克隆配置后读取可变凭据，可能混用不同版本的地址和密钥 | 在配置 Owner 内同时捕获配置及 secret，持久化不可变 Keyring reference；Database Studio、Git 同步与 Push、AI 本机导入的模型及插件凭据一起迁移。 |
| 2 | MCP 配置、凭据引用及命令片段定义可能在审批之后改变 | MCP 原生 review 返回实际执行描述及精确修订；执行绑定 revision、本次批准的凭据和环境集合。Snippet 执行要求用户预览的定义 revision，覆盖 Coding、Script、Skill 和 Blueprint 入口。 |
| 3 | Windows 临时文件写入内容后才处理 ACL，可能短暂继承更宽权限 | 创建时使用私有安全描述符；写前审计；现有目标的 DACL 在字节写入前复制，并保留 protected 状态及 ACE 读回。未发布 stage 由原生句柄清理。 |
| 4 | 排队草稿任务读取可变全局状态，可能写入另一文档的内容 | 入队之前捕获 path、content、baseline、session；保存完成绑定已发布文档实例，不能清理后来重新打开的同路径实例。 |
| 5 | 原文件消失后缺少完整的加密草稿发现、恢复和另存入口 | 增加分页目录、认证读取、独立 fork、按 revision 更新/删除、no-replace 另存，并接入现有 Files 和 FileEditor。 |
| 6 | JSON loader 把读取或解析失败当作空列表，随后覆盖原数据 | 仅 NotFound 返回空；读失败、超限及损坏返回错误。损坏原件保留并按内容哈希备份；Script、Snippet、Git、Bookmark、Task、Plugin 和 trusted keys 调用链同步迁移。 |
| 7 | 未打开的新文件 proposal 缺少目录变动失效依据 | UI 将全部待应用路径登记到 Document Ownership；后端持久化祖先目录身份与缺失后缀证明，应用前重新读取。旧提案缺少证明时要求重新审核。 |
| 8 | Coding 取消早于运行登记时丢失，旧取消可能干扰新运行 | 每次运行使用独立 UUID run_id；有界提前取消表与原生运行登记衔接，取消绑定具体运行。 |
| 9 | 部分目录/页签入口在异步读前后分别发布，旧结果覆盖新导航 | Home、ActivityBar、地址栏、文件树、Inline、Overlay、Unified Tabs 和 Browser 排队入口共享导航意图，读成功后同步发布。启动目录预热只观察导航版本。 |
| 10 | Browser 清理吞掉原生关闭失败，仍报告释放成功 | 只移除关闭成功的资源；失败记录保持 Owner 并可重试。关闭不存在的 tab 幂等成功，真正 native close 失败保留插件与 bridge 状态。 |
| 11 | Grid 聚焦/失焦独立 worker 的发送顺序与最新可见状态不一致 | 使用一个可合并的最新可见意图 worker；reposition 共享发布顺序，保留 150ms 策略及原有平台几何、show/hide 机制。 |
| 12 | MCP/Sandbox、Terminal resize、DB query 的容量需要覆盖真实后台生命周期 | Sandbox 有界 native Owner 及独立 I/O guard；Terminal 只保留一个运行中 resize 和一个最新待执行尺寸；DB 查询登记归实际 worker。调用方停止等待不提前归还资源容量。 |
| 13 | Session 手动操作在命令/UI 路径同步执行磁盘及审计 I/O | 保存、列举、读取、删除、导出、恢复、flush、discard、策略更新进入有界 SESSION_IO；真正的页签 UI 创建仍在主线程。 |

## 配置、凭据及审批的一致性

共享 `shared/credential_versions.rs` 只保存非秘密的清理意图。新 Keyring 项写入之前先持久化意图；配置保存后，根据重新读取的磁盘 reference 判断哪些版本可以删除。即使原子替换已经发生、目录同步却返回错误，也不能依据旧内存配置猜测当前 live key。

各 Owner 在短锁内捕获本次执行所需的配置及 secret，随后释放锁。后续排队和网络阶段不再读取可变 Keyring 名称。失败的清理保留持久记录，并受容量限制。Unix 新目录项同步父目录；Windows 对目录、记录和临时残留拒绝重解析点。

Git 公开 RepoConfig 保持兼容，版本引用仅存在原生存储 wrapper。fetch 检查 origin URL；push 检查 pushurl 或 origin URL；credential callback 进一步绑定协议、主机、端口、路径和 query。缓存 remote 与本次配置不一致时拒绝执行。错误在持有 token 的边界统一脱敏，再交给任务和审计。

MCP review 绑定原生解析后的配置及本次动作，只返回模型配置名称、端点、引用和范围，不返回 secret。执行时重新比较 revision 和精确授权集合，批准之后再从凭据 Owner 内捕获 secret。命令适配器也不能临时读取“最新 revision”来替代用户已经看到的定义。

## AI 本机导入完整收口

Python 仅做有界的只读发现，使用带 request_id 的 native-export 管道协议。Rust 对数量、大小、字段和引用范围完成检查后，先构造完整合并计划，再逐项 stage 不可变凭据、保存配置并根据真实磁盘状态 reconcile。模型和插件凭据均受同一发布契约管理。

秘密只在原生管道和内存中传递，不进入 UI、审计、临时文件或错误日志。输入、输出、扫描文件数、目录条目、深度、总字节和运行时间均有上限。元数据同时拒绝完整 secret 及去掉可选 Bearer 前缀的实际 token。

新模型凭据按裸 token 保存；AI provider 读取既有模型密钥时也规范一次可选 Bearer 前缀，避免重复 Authorization 前缀。既有 Keyring 不批量重写；插件 Authorization 和 MCP 环境值保持原值。

现有模型及插件资料保留，重复导入不覆盖已有凭据版本。脚本默认的只读发现摘要保留；旧 `--apply` 直接写入口关闭并明确引导到 BrowserOS 的导入操作，避免绕开统一 Owner。配置 UI 显示未完成的凭据清理；刷新会重试清理，不再只显示导入或保存成功计数。

## 草稿、文档及目录变更

恢复目录不依赖源文件仍然存在，也不因 journal 中记录了某个路径就授予文件访问权限。记录 ID、密文认证、路径身份与 session 关系在后端核对；source_path 保留原生路径并兼容旧加密记录。

打开恢复记录先创建独立 fork，原会话可以继续写自己的 journal。恢复编辑绑定 fork 的对象及 revision。更新和显式删除采用原生版本化操作；损坏记录只有在目录提供可绑定的密文 revision 时才能显式删除。

恢复另存仅创建尚不存在的目标；目标仍需通过原生允许根目录及句柄授权。普通 Ctrl+S 不会重建丢失的源文件。正常编辑器保存只清理自身 session 的 journal，其他会话来源由用户按目录 revision 显式删除。

旧 write_journal 和 clear 也迁入同一原生 CAS，避免旧临时文件引擎或盲删绕过恢复 API 的保护。BrowserOS 协作锁和 CAS 不承诺对任意第三方进程形成内核级不可分割事务。

Pending proposal 即使文件尚未打开，也登记到文档 Owner。后端的目录身份及缺失后缀证明跨重启保留；不能因同名路径重新出现就沿用旧审批。缺少可信证明时，在备份、journal 或项目写入之前拒绝并要求重新审核。

## 存储失败及任务终态

严格 loader 保留损坏原件，并为同一内容复用同一哈希备份，避免重复读取制造无限备份。备份失败也不能将原数据解释为空。调用方传播错误，阻止后续正常覆盖流程。

Task/Plugin 原本独立的 loader 采用相同契约。Scheduler 在配置不可读时暂停调度；历史无法写入时，InFlight 保留不可变终态 TaskRunRecord。按 run_id 幂等追加成功后才回收，避免运行表淘汰后改变状态、完成时间或产生重复历史。

## 原生安全及生命周期的实际保证

Windows staging 从创建起就带受限 DACL，复制目标权限时同时保留继承保护状态。普通错误和 unwind 触发句柄级清理；完成写入、flush、identity 后，在 journal/锁等待区间启用可撤销 delete disposition，发布前撤销。

这不承诺任何硬崩溃都不遗留临时文件：写入和最终 rename 的窄窗口仍可能留下具有正确 ACL 的未发布文件。没有增加按文件名前缀盲删的清扫逻辑，也没有将不能撤销的 FILE_FLAG_DELETE_ON_CLOSE 用于即将发布的文件。

Browser 释放失败保留重试所需的状态。Grid、Sandbox、Terminal 和 DB 的容量覆盖实际 worker、I/O、进程或句柄生命周期。Session restore 在主线程回调完成前保留 lane permit，不用提前超时释放来制造可能晚到的无主页签。

## 回归定义及验收状态

新增或调整的定义覆盖文档身份、排队草稿、同路径重新打开、异步导航、恢复 fork、另存冲突、损坏存储、终态历史幂等、MCP 修订、命令审批、提前取消、持久 proposal 路径证明、stage ACL、资源容量及 AI 导入的失败/不确定发布。全部未执行。

| 项目 | 状态 |
|---|---|
| 本轮确认的 13 组源码问题及关联入口 | 纳入同一个交付提交 |
| Node / Rust / Python 回归定义 | NOT_RUN |
| cargo check / build、前端构建、lint、format、校验脚本 | NOT_RUN，遵照用户要求 |
| GitHub Actions / workflow_dispatch | 未主动触发；提交含 skip 标记，不创建 PR |
| Windows Native / AppContainer / LSP / ConPTY 实际运行 | NOT_RUN |
| NSIS 构建、安装包哈希及安装版启动 | NOT_RUN |
| Windows GUI / WebView2 / BFC 真实交互 | GUI_PENDING |
| Linux 实际回归和性能测量 | NOT_RUN |
| master、原开发分支及合并 | 未修改、未合并 |

本轮没有将上述已确认源码问题拆成后续修复阶段。运行验收按用户禁止执行的要求如实保留，不能据此宣布全量 Windows 通过或仓库不存在任何未知缺陷。

## 逐文件修改范围

本提交共包含 77 个文件（含本说明）。以下列表按路径列出修改范围；测试及验收定义没有执行。

| 文件 | 修改范围 |
|---|---|
| [crates/browseros-platform-security/src/windows.rs](../../../crates/browseros-platform-security/src/windows.rs) | Windows 创建时 ACL 和 DACL 保留 |
| [docs/engineering/reviews/2026-11-22-owner-integrity-completion.md](../../../docs/engineering/reviews/2026-11-22-owner-integrity-completion.md) | 修复说明、验收边界与逐文件清单 |
| [scripts/import-local-ai.py](../../../scripts/import-local-ai.py) | 有界只读 AI 发现及原生导出协议 |
| [src-tauri/Cargo.toml](../../../src-tauri/Cargo.toml) | 命令注册、权限及依赖 feature |
| [src-tauri/permissions/default-commands.toml](../../../src-tauri/permissions/default-commands.toml) | 命令注册、权限及依赖 feature |
| [src-tauri/src/bridge.rs](../../../src-tauri/src/bridge.rs) | Browser 关闭、失败保留及清理重试 |
| [src-tauri/src/bridge/automation_commands.rs](../../../src-tauri/src/bridge/automation_commands.rs) | IPC 调用方迁移及错误传播 |
| [src-tauri/src/bridge/credential_commands.rs](../../../src-tauri/src/bridge/credential_commands.rs) | IPC 调用方迁移及错误传播 |
| [src-tauri/src/bridge/filesystem_commands.rs](../../../src-tauri/src/bridge/filesystem_commands.rs) | IPC 调用方迁移及错误传播 |
| [src-tauri/src/bridge/integration_commands.rs](../../../src-tauri/src/bridge/integration_commands.rs) | 严格存储错误传播、终态持久化及 Owner |
| [src-tauri/src/bridge/terminal_commands.rs](../../../src-tauri/src/bridge/terminal_commands.rs) | Terminal resize 合并和资源容量 |
| [src-tauri/src/bridge/workspace_commands.rs](../../../src-tauri/src/bridge/workspace_commands.rs) | IPC 调用方迁移及错误传播 |
| [src-tauri/src/capabilities/agent/coding.rs](../../../src-tauri/src/capabilities/agent/coding.rs) | Coding 路径证明、取消与执行审批 |
| [src-tauri/src/capabilities/agent/coding_context.rs](../../../src-tauri/src/capabilities/agent/coding_context.rs) | Coding 路径证明、取消与执行审批 |
| [src-tauri/src/capabilities/agent/coding_context_windows.rs](../../../src-tauri/src/capabilities/agent/coding_context_windows.rs) | Coding 路径证明、取消与执行审批 |
| [src-tauri/src/capabilities/agent/coding_mcp_windows.rs](../../../src-tauri/src/capabilities/agent/coding_mcp_windows.rs) | Windows Sandbox / MCP 原生 I/O 生命周期 |
| [src-tauri/src/capabilities/agent/mcp_review.rs](../../../src-tauri/src/capabilities/agent/mcp_review.rs) | Coding 路径证明、取消与执行审批 |
| [src-tauri/src/capabilities/agent/mod.rs](../../../src-tauri/src/capabilities/agent/mod.rs) | Coding 路径证明、取消与执行审批 |
| [src-tauri/src/capabilities/agent/public_api.rs](../../../src-tauri/src/capabilities/agent/public_api.rs) | Coding 路径证明、取消与执行审批 |
| [src-tauri/src/capabilities/agent/windows_sandbox.rs](../../../src-tauri/src/capabilities/agent/windows_sandbox.rs) | Windows Sandbox / MCP 原生 I/O 生命周期 |
| [src-tauri/src/capabilities/agent/windows_sandbox/lifecycle.rs](../../../src-tauri/src/capabilities/agent/windows_sandbox/lifecycle.rs) | Windows Sandbox / MCP 原生 I/O 生命周期 |
| [src-tauri/src/capabilities/database/commands.rs](../../../src-tauri/src/capabilities/database/commands.rs) | DB 配置/凭据快照、Studio 和查询 Owner |
| [src-tauri/src/capabilities/database/database.rs](../../../src-tauri/src/capabilities/database/database.rs) | DB 配置/凭据快照、Studio 和查询 Owner |
| [src-tauri/src/capabilities/database/profiles.rs](../../../src-tauri/src/capabilities/database/profiles.rs) | DB 配置/凭据快照、Studio 和查询 Owner |
| [src-tauri/src/capabilities/database/studio.rs](../../../src-tauri/src/capabilities/database/studio.rs) | DB 配置/凭据快照、Studio 和查询 Owner |
| [src-tauri/src/capabilities/git/commands.rs](../../../src-tauri/src/capabilities/git/commands.rs) | Git 快照、远端绑定及错误脱敏 |
| [src-tauri/src/capabilities/git/sync.rs](../../../src-tauri/src/capabilities/git/sync.rs) | Git 快照、远端绑定及错误脱敏 |
| [src-tauri/src/capabilities/plugin/plugin.rs](../../../src-tauri/src/capabilities/plugin/plugin.rs) | 严格存储错误传播、终态持久化及 Owner |
| [src-tauri/src/capabilities/script/scripts.rs](../../../src-tauri/src/capabilities/script/scripts.rs) | 命令预览修订及公共执行契约 |
| [src-tauri/src/capabilities/session/commands.rs](../../../src-tauri/src/capabilities/session/commands.rs) | Session 有界异步磁盘与审计入口 |
| [src-tauri/src/capabilities/task/commands.rs](../../../src-tauri/src/capabilities/task/commands.rs) | 严格存储错误传播、终态持久化及 Owner |
| [src-tauri/src/capabilities/task/scheduler.rs](../../../src-tauri/src/capabilities/task/scheduler.rs) | 严格存储错误传播、终态持久化及 Owner |
| [src-tauri/src/capabilities/terminal/terminal.rs](../../../src-tauri/src/capabilities/terminal/terminal.rs) | Terminal resize 合并和资源容量 |
| [src-tauri/src/capabilities/workspace/draft_journal.rs](../../../src-tauri/src/capabilities/workspace/draft_journal.rs) | 加密草稿、条件写入与恢复 |
| [src-tauri/src/capabilities/workspace/mod.rs](../../../src-tauri/src/capabilities/workspace/mod.rs) | Workspace 文档、恢复 UI 及共享持久化 |
| [src-tauri/src/capabilities/workspace/safe_paths_windows.rs](../../../src-tauri/src/capabilities/workspace/safe_paths_windows.rs) | Windows 创建时 ACL 和 DACL 保留 |
| [src-tauri/src/capabilities/workspace/workspace.rs](../../../src-tauri/src/capabilities/workspace/workspace.rs) | Workspace 文档、恢复 UI 及共享持久化 |
| [src-tauri/src/framework/ai.rs](../../../src-tauri/src/framework/ai.rs) | AI 快照、原生导入、模型认证及运行取消 |
| [src-tauri/src/grid_lifecycle.rs](../../../src-tauri/src/grid_lifecycle.rs) | Grid 可见意图、发布顺序及容量 |
| [src-tauri/src/grid_process.rs](../../../src-tauri/src/grid_process.rs) | Grid 可见意图、发布顺序及容量 |
| [src-tauri/src/grid_process_windows.rs](../../../src-tauri/src/grid_process_windows.rs) | Grid 可见意图、发布顺序及容量 |
| [src-tauri/src/main.rs](../../../src-tauri/src/main.rs) | 命令注册、权限及依赖 feature |
| [src-tauri/src/native_58_gui_review.rs](../../../src-tauri/src/native_58_gui_review.rs) | 既有验收定义错误传播（NOT_RUN） |
| [src-tauri/src/shared/credential_versions.rs](../../../src-tauri/src/shared/credential_versions.rs) | 版本化凭据及持久清理意图 |
| [src-tauri/src/tasks.rs](../../../src-tauri/src/tasks.rs) | 严格存储错误传播、终态持久化及 Owner |
| [src/App.vue](../../../src/App.vue) | 导航意图与异步发布 |
| [src/bridge.ts](../../../src/bridge.ts) | 前后端接口与数据契约 |
| [src/capabilities/agent/model/codingWorkbench.mjs](../../../src/capabilities/agent/model/codingWorkbench.mjs) | Coding 路径证明、取消与执行审批 |
| [src/capabilities/agent/ui/CodingCommandPanel.vue](../../../src/capabilities/agent/ui/CodingCommandPanel.vue) | Coding 路径证明、取消与执行审批 |
| [src/capabilities/agent/ui/CodingOperationReview.vue](../../../src/capabilities/agent/ui/CodingOperationReview.vue) | Coding 路径证明、取消与执行审批 |
| [src/capabilities/agent/ui/CodingWorkbench.vue](../../../src/capabilities/agent/ui/CodingWorkbench.vue) | Coding 路径证明、取消与执行审批 |
| [src/capabilities/agent/ui/McpExecutionReview.vue](../../../src/capabilities/agent/ui/McpExecutionReview.vue) | Coding 路径证明、取消与执行审批 |
| [src/capabilities/browser/state/useBrowserStore.ts](../../../src/capabilities/browser/state/useBrowserStore.ts) | Browser 关闭、失败保留及清理重试 |
| [src/capabilities/database/state/useDatabaseStore.ts](../../../src/capabilities/database/state/useDatabaseStore.ts) | DB 配置/凭据快照、Studio 和查询 Owner |
| [src/capabilities/home/state/useHomeStore.ts](../../../src/capabilities/home/state/useHomeStore.ts) | 导航意图与异步发布 |
| [src/capabilities/script/public.ts](../../../src/capabilities/script/public.ts) | 命令预览修订及公共执行契约 |
| [src/capabilities/script/ui/ScriptRunDialog.vue](../../../src/capabilities/script/ui/ScriptRunDialog.vue) | 命令预览修订及公共执行契约 |
| [src/capabilities/skill/state/useSkillStore.ts](../../../src/capabilities/skill/state/useSkillStore.ts) | 命令预览修订及公共执行契约 |
| [src/capabilities/skill/ui/SkillManagerPanel.vue](../../../src/capabilities/skill/ui/SkillManagerPanel.vue) | 命令预览修订及公共执行契约 |
| [src/capabilities/workspace/model/draftRecovery.d.ts](../../../src/capabilities/workspace/model/draftRecovery.d.ts) | 加密草稿、条件写入与恢复 |
| [src/capabilities/workspace/model/draftRecovery.mjs](../../../src/capabilities/workspace/model/draftRecovery.mjs) | 加密草稿、条件写入与恢复 |
| [src/capabilities/workspace/state/useFileStore.ts](../../../src/capabilities/workspace/state/useFileStore.ts) | Workspace 文档、恢复 UI 及共享持久化 |
| [src/capabilities/workspace/ui/FileEditor.vue](../../../src/capabilities/workspace/ui/FileEditor.vue) | Workspace 文档、恢复 UI 及共享持久化 |
| [src/capabilities/workspace/ui/FilePanel.vue](../../../src/capabilities/workspace/ui/FilePanel.vue) | Workspace 文档、恢复 UI 及共享持久化 |
| [src/components/layout/ActivityBar.vue](../../../src/components/layout/ActivityBar.vue) | 导航意图与异步发布 |
| [src/components/system/AiConfiguration.vue](../../../src/components/system/AiConfiguration.vue) | AI 导入结果及凭据清理状态展示 |
| [src/composables/useUnifiedTabs.ts](../../../src/composables/useUnifiedTabs.ts) | 导航意图与异步发布 |
| [src/settings/ai.ts](../../../src/settings/ai.ts) | AI 导入结果及凭据清理状态展示 |
| [src/shared/ui/BlueprintStudio.vue](../../../src/shared/ui/BlueprintStudio.vue) | 命令预览修订及公共执行契约 |
| [src/stores/useLayoutStore.ts](../../../src/stores/useLayoutStore.ts) | 导航意图与异步发布 |
| [src/types.ts](../../../src/types.ts) | 前后端接口与数据契约 |
| [tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/commands.rs](../../../tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/commands.rs) | Browser 关闭、失败保留及清理重试 |
| [tests/check-ai-native-import.py](../../../tests/check-ai-native-import.py) | 回归定义（NOT_RUN） |
| [tests/check-cross-workbench-file-store.mjs](../../../tests/check-cross-workbench-file-store.mjs) | 回归定义（NOT_RUN） |
| [tests/check-unified-tab-actions.mjs](../../../tests/check-unified-tab-actions.mjs) | 回归定义（NOT_RUN） |
| [tests/check-workspace-draft-recovery.mjs](../../../tests/check-workspace-draft-recovery.mjs) | 回归定义（NOT_RUN） |
| [tests/check-workspace-recovery-navigation.mjs](../../../tests/check-workspace-recovery-navigation.mjs) | 回归定义（NOT_RUN） |
