# BrowserOS Windows / Linux 双平台实施方案 V2（审查修订版）

- 日期：2026-11-14
- 状态：**Proposed V2 / 等待 Owner 批准；不是已生效的 PROJECT-RULES 决策**
- 目标分支：`fix/bfc-readback-workbench-hardening-20261114`
- 固定源码审查基线：`d80f59b0c04457e743a5a18e6e8acae249ea7c6c`
- 对照：[原始 V1](./browseros-windows-linux-support-20261114.md) · [V1 审核报告](../engineering/reviews/2026-11-14-windows-linux-plan-audit.md) · [V2 Backlog](../engineering/reviews/2026-11-14-windows-linux-implementation-backlog-v2.md)
- 保留策略：原始 V1 不改；V2 作为建议替代方案，必须经过 Owner 评审后才能转换为任务授权。

## 0. 决策摘要

采用**风险前置、最小可运行链路、每项原生能力独立放行**的交付方式。保留 V1 的 R1 Windows BFC Beta 与 R2 双平台工作台目标，但提前处理 R1 必需的原生文件拒绝与持久化安全、本地 BFC 服务合同、WebView2 不确定性；把 Windows 真实 POC 提前到完整 Windows 编译底座之前，避免提前重构。

**不可推论：** 本文不代表 Windows 可编译、可安装、WebView2 脚本有回执、B 站已经成功读取、BFC 服务可在 Windows 运行或 R1/R2 已验收。所有平台能力默认未验证，直到有证据。

## 1. V1 与 V2 的关键区别

| 维度 | V1 | V2 |
| --- | --- | --- |
| WebView2 技术验证 | XP-004 Windows 编译之后开始 XP-005 | XP-005A 独立 Windows POC 提前；XP-005B 再集成 |
| R1 的文件安全 | 完整 XP-011 位于 R2 | 完整 XP-011 保留 R2，但 R1 命令 fail-closed 与持久化最小安全门前移 |
| 本地 BFC | 默认连接 127.0.0.1:4198 并端到端验收 | XP-000 先冻结 Windows 侧服务提供、版本、安装、CSRF、回执、失败合同 |
| Windows 10 | 22H2 与 Windows 11 同列首发 | Windows 11 x64 主目标，Windows 10 22H2 x64 仅有条件兼容 |
| 能力协商 | 前端显示/禁用能力 | Rust 强制拒绝 + 前端可用性显示 + 远程权限审计 |
| 浏览器事件 | 导航/资源事件适配 | 区分导航请求、提交、完成、失败和代际；页面健康与完整资源瀑布分离 |
| Windows 打包 | XP-018 后期 OS 集成 | XP-018A 早期最小 NSIS 验证；XP-018B 完整 OS 集成后续 |
| Agent 并行 | 任务表与第一批描述存在依赖冲突 | 允许调查并行，但提交顺序受合同与依赖约束，各任务隔离分支 |
| PR 限制 | 默认 150 行 | 150 行作为拆分提示，安全原子性/测试完整性优先 |
| Linux 迁移 | 路径替换与回归 | 显式保护旧数据目录、WebKit Profile、锁定 GTK 语义、用户数据恢复 |

## 2. 真实基线与架构限制

### 2.1 已从源码核实

- `src-tauri/src/main.rs`、`grid_process.rs` 使用 Unix Socket、进程组、信号以及 X11/GTK 专用初始化；不能直接把相同代码视为 Windows 后端。
- `tauri-browser-tabs/.../commands.rs` 中 `eval_result` 非 Linux 明确返回 `GRID_PLATFORM_UNSUPPORTED`；`platform/windows.rs` 当前并非 WebView2 完整后端。
- `src-tauri/src/capabilities/bfc/commands.rs` 使用独立 loopback BFC 服务（`127.0.0.1:4198`），负责 health/session/CSRF/import/receipt。
- `workspace/safe_paths.rs` 在非 Linux 下拒绝多项句柄式安全操作；`workspace/cas.rs` 却有非 Linux 路径式写入 fallback。
- `session.rs` `atomic_write` 通过固定 `json.tmp` 和 `fs::rename` 持久化；Windows 原子覆盖和崩溃语义尚未验收。
- `workspace/workspace.rs` `data_dir` 在 Tauri app_data_dir 下再拼 `mvp-browser-os`，Profile 位于插件的 app_data_dir 下；迁移不得暗中改动既有 Linux 真实数据布局。
- `src-tauri/capabilities/bfc-readback.json` 的远程授权仅允许 Bilibili HTTPS 域的 `tab-*`；`browser-remote.json` 另有较宽 remote-collect 权限，应在任何权限变更前进行并集审查。
- `AGENTS.md` 与 `PROJECT-RULES.md` 有锁定的 WebView 布局/隐藏/关闭页签规则；V2 不改业务成功语义。

这些是**静态观察**，不是 Windows 功能 PASS。

### 2.2 首发平台范围与非目标

| 平台 | V2 目标 | 不承诺 |
| --- | --- | --- |
| Windows | Windows 11 x64，标准用户，WebView2 Evergreen；R1 先单机 BFC | ARM64、Windows Server、受限 Runtime 环境 |
| Windows 10 | 22H2 x64 **可选兼容**，须单独验证，用户系统安全维护/ESU 前提需说明 | 与 Windows 11 等等级默认支持 |
| Linux | Ubuntu 22.04/24.04 x64，X11，既有 GTK/WebKitGTK 路径 | Wayland/RPM/未经验证发行版 |
| 安装包 | Windows NSIS；Linux DEB；AppImage 有证据后新增 | `bundle.targets=all` 等于全部安装包可用的宣传 |

Windows 10 22H2 常规支持结束于 2025-10-14，参见 https://learn.microsoft.com/en-us/lifecycle/announcements/windows-10-end-of-support 。

### 2.3 与 BFC 写回整理模式的边界

R1/R2 的 BFC 仅包含**用户授权的可见 DOM 回读、本地快照及核验导入**。B 站原收藏夹创建、复制、移动、删除、批量整理是另一个显式授权的账号写回项目，不因跨平台适配获得权限；不得读取/导出 Cookie，不开 CDP 调试端口，不通过操作 B 站 API 绕过既有安全合同。Windows 方案只保证读取与本地导入的可验证边界。

## 3. R1：Windows BFC Beta（最小可交付）

R1 **必须有**：安装版主窗与原生子标签；受控 WebView2 共享浏览器 Profile，主 UI 隔离；网页真实导航与失败事件；带大小上限、可取消、可区分失败的脚本回执；BFC 原有 begin/submit/status/finish、MID/TTL/tab 限制、分页、partial、去重、结果未知、幂等；服务启动/不存在提示；本地导入、重启后登录态；R1 实际开放的会话/书签等持久化经过 Windows 安全验证；Windows 原生权限与未支持命令拒绝；NSIS 安装/升级/卸载；独立 Windows GUI 验收。

R1 **不可隐形开放**：Grid 子进程、终端、脚本执行、数据库、Agent/LSP 沙箱、完整资源瀑布、未验收的编辑器文件 CAS、文件移动等。前端标记为 `unavailable` 或 `blocked`，**Rust 命令入口同样拒绝**；测试必须尝试直接调用这些命令并确认无副作用。

R1 的 `bfc.readback` 只在 WebView2 回读、本地 BFC 依赖、真实登录、导入及结果核对全部具备时标记 `available`；如果 Windows BFC 服务未交付，只能声明“BrowserOS 回读前端/原生 POC”，不能叫完整 BFC Beta。

## 4. R2：双平台完整能力

R2 从 R1 扩展为：Windows SecureFileSystem（handle 绑定、Junction/Reparse/ACL/UNC/ADS/长路径与冲突）、ProcessControl/Job Object、Named Pipe、ConPTY/PowerShell、Git/MCP 子进程、SQLite 后 MySQL/PostgreSQL、Credential Manager、AI Provider/配置导入、BrowserOS Grid、Agent/LSP 最小权限隔离、Windows OS 集成；Linux DEB/AppImage 与 X11 真实验收。

**R2 的“完整”须按能力逐项公告**。若 Agent/LSP 安全沙箱无法满足既有隔离合同，可推迟该能力并将产品版本定义为 `R2 Core`，不可宣称“所有能力完全等价”。

## 5. 目标架构和契约

~~~text
Vue WebView-aware Shell
    |
    v
BrowserRuntime adapter / Capability manifest (read-only)
    |
    v
Rust command ACL + per-feature native enforcement
    |
    +-- BrowserEngine { Linux: WebKitGTK; Windows: WebView2 }
    +-- AppPaths { Linux: XDG+legacy data; Windows: app data }
    +-- SecureFileSystem { Linux: pinned fd; Windows: handle+ACL }
    +-- ProcessControl / IpcChannel { Unix PG+UDS; Windows Job+Named Pipe }
    +-- SystemIntegration / CredentialStore / DB / AI
    |
    +-- BfcRuntime -- loopback service 127.0.0.1:4198 [versioned dependency]
~~~

**依赖规则：** 前端不得直连 OS API；远程网页不新增未经审批的桥权限；平台层不可反向越过现有 `check_invocation_source`、Tauri capabilities、BFC tab binding、KeyringStore 和数据写入校验。

### 5.1 CapabilityManifest：有状态，不是伪布尔 PASS

建议以稳定版本 DTO 返回：`capabilityId`、`state` (`available|degraded|unavailable|blocked`)、`reasonCode`、`platform`、`evidenceClass`（设计/静态/CI/GUI/业务验证）；该 API 是**候选合同**，必须在 XP-000 审核后冻结。

最低区分：`browser.basic`, `browser.persistentProfile`, `browser.scriptResult`, `browser.navigationHealth`, `browser.httpAccessSignals`, `browser.resourceWaterfall`, `bfc.readback`, `bfc.localImport`, `workspace.safeRead`, `workspace.safeMove`, `workspace.cas`, `grid`, `terminal`, `scripts`, `database.sqlite`, `database.external`, `ai.provider`, `agent.coding`, `agent.lspSandbox`。

能力 manifest 只描述事实；**不能作为跳过原生权限验证的票据**。对远程页的 BFC submit 仅限 HTTPS Bilibili `tab-*`，begin/status/finish 主 UI；所有新入口继续 Tauri ACL + 原生 Owner 检查。

### 5.2 WebView2 执行与事件合同

- 两类 WebView：可信主 UI 与不可信外部浏览器子 WebView 使用不同环境与独立存储；浏览器子视图共享一个应用专属、当前用户可访问的 Profile，不能各 tab 分离导致登录态丢失。
- 固定 User Data Folder 路径与兼容环境选项；测试两个 tab 并行、重启、WebView2 进程失败、窗口重建及权限异常。参考 Microsoft：https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/user-data-folder 。
- 原生脚本回执明确 `success(json)`、`injection_failed`、`script_timeout`、`navigation_destroyed`、`tab_closed`、`process_failed`、`result_too_large`；不伪造 `logged_out`。维护 executionId、tab generation、文档 URL 身份；迟到回调不得污染新 tab。
- 导航请求≠加载完成。非 Linux 当前 `on_navigation` 上报 `NavigationFinished` 不能作为完成证据。定义请求/提交/完成/失败、main frame、文档代际；404/412/证书/TLS/崩溃分别有可观察信息，不可从缺失资源事件猜 HTTP 412。
- 资源瀑布扫描不是 R1 强制能力，BFC 必需的页面健康与安全提示是。无法从 WebView2 可靠捕获 412 时必须标 `unknown/access_limited` 并停止，不得编造 HTTP 状态。
- Linux 原生 WebKit 仍按 PROJECT-RULES.md 锁定的 `gtk_fixed_move + size_allocate`、禁止`queue_resize` 与隐藏时保留尺寸策略运行；Windows 另做缩放、Z-order、DPI、多屏和热插拔实测。

### 5.3 BFC 服务依赖与失败语义

在 XP-000 设立单独 `BfcLocalServiceContract`（设计名称）：生产者/版本号/健康和 capabilities/schema/CSRF 流程/loopback-only/超时与回执核对/安装升级卸载/最低兼容版本。不得默认 Windows 能运行现有 Linux 服务。服务版本不匹配时 `bfc.localImport=blocked`，BFC UI 显示原因而不是伪造成功。

现有 Rust `ImportFailure` 的 `NotSent`、`Rejected`、`Unknown` 三类结果必须保留。网络发送后的超时应维持 `outcome_unknown`，不得盲目重发；对重复 scan_id、取消和跨平台升级建立测试。导入端不得向 B 站发新请求。

### 5.4 文件/路径/持久化合同

- `AppPaths` 为数据根、Profile、会话、缓存、日志、临时、图片、用户笔记目录统一发现；优先兼容 Linux 旧版实际布局，不引入第二套默认目录。必须检查 HOME、XDG、`/tmp`、`/usr/bin/curl`、`/usr/bin/python3`、配置导入、安装版写权限。
- R1 的关键路径只读/写入白名单：需要书签/会话时记录每个持久化函数及数据敏感级别；验证覆盖/并发/中途崩溃/文件锁/升级/卸载保留；失败不得静默清空或覆盖用户数据。
- 所有未经过 Windows 文件安全审核的 `write_file_cas`、move、safe-read 命令在 Windows Rust 入口返回稳定 unsupported 错误，不进入普通 `fs::rename` fallback；绝不能仅靠前端关闭。
- R2 Windows SafeFS 实现须使用 OS 句柄验证根与最终对象身份、拒绝目录链接重解析、明确同卷/跨卷、ACL、文件共享模式和 Windows 替换语义。不得以 `canonicalize` 或提前检查同名文件代替原子无覆盖移动。
- Linux 现有 `safe_paths` 的 descriptor-pinned 路径及临时文件权限保持行为等价。对非协作外部写入的 CAS 竞争条件应明确记录为已知限制，不宣称绝对内核 CAS。

### 5.5 进程、IPC、沙箱与网络

- Windows Grid 若进入 R2：当前用户 ACL 的 Named Pipe、端点身份验证、帧上限、generation/sequence、重连后旧响应隔离；Job Object 负责整个子树退出，但**不等价应用沙箱**。
- Windows 终端采用受控 ConPTY 与 PowerShell，不要求用户安装 WSL/Git Bash；取消、kill、resize、中文路径和子进程回收必须有测试。
- Agent/LSP 只能在达到可证明的权限隔离后开放；研究受限 Token/AppContainer/受控工作区访问，不能把仅有 Job Object 的进程称为隔离沙箱。
- AI HTTP 可优先复用仓库已有 reqwest，但必须逐项对齐现存请求/流式/超时/取消/并发/密钥保护语义，不随意改协议；Python 配置导入优先 Rust 原生方案或明示发现策略。
- Windows Credential Manager 和 Linux Secret Service 需分别验证无密钥、锁定、不可用、升级保留和明确错误码。

## 6. 分阶段交付与出口

| 阶段 | 范围 | 进入/退出门槛 |
| --- | --- | --- |
| G0 合同冻结 | XP-000、XP-001 | 明确 R1 非目标、BFC 服务、原生能力及数据风险，清单可检索 |
| G1 技术可行性 | XP-005A（可与清单并行） | 在独立真实 Windows 原型中证明 Profile/回执/子视图，失败时阻塞 R1 |
| G2 平台编译底座 | XP-002、003、004 | Windows experimental 记录真实红绿；Linux 回归；平台 fail-closed |
| G3 BFC 垂直实现 | XP-005B、006、007、008、009 | 全量夹具 + 事件代际 + 安全失败分类；BFC 服务合同核实 |
| G4 R1 交付 | XP-011R1、018A、010 | Windows 安装/升级、真实 GUI、登录→回读→导入→重启登录态、数据安全 PASS |
| G5 R2 扩展 | XP-011～017、018B；XP-019 可并行 | 每项能力有独立 OS 测试，通过才开放 |
| G6 发布验收 | XP-020 | Windows/Linux 独立证据、无偷换 PASS、已知限制有用户可见提示 |

执行顺序可以把 G0 与不改生产代码的 G1 原型并行，但**任何会改变产品权限/持久化的开发必须先通过 G0**。

## 7. 测试/验收矩阵

| 维度 | Linux 22.04/24.04 X11 | Windows 11 x64 |
| --- | --- | --- |
| 前端、类型/契约/架构检查 | 每个相关 PR | 每个相关 PR |
| Rust check/test | native 目标 | native 目标 |
| GUI/子 WebView/焦点/多标签 | 实机独立 | 实机独立，100/125/150/200% DPI+多屏 |
| Profile 与重启 | WebKit 真实登录态 | WebView2 真实登录态 |
| BFC 流程 | 模拟+真实站点受控 | 模拟+真实站点受控 |
| 文件与数据 | descriptor/权限/崩溃 | Reparse/ACL/锁/长路径/中文用户名 |
| 安装/升级/卸载 | DEB、后续 AppImage | NSIS，标准用户及 Runtime 缺失 |
| 进程残留、任务取消 | PG、UDS | Job、Pipe（R2） |
| 可执行功能权限 | 原生 ACL 与 Owner | 原生 ACL 与 Owner/unsupported |
| 不支持能力表现 | 明确提示 | 明确提示 + 无副作用 |

BFC 专项最低场景：未登录/MID 缺失/访问受限/412/TLS、第一页和多页、空占位、页内与跨页 BVID 重复、页码不前进、partial、超时、注入失败、页面切走、跨域及错误 tab、迟到回执、关闭/崩溃、CSRF 和 BFC 服务故障、重复导入/取消、结果未知、重启登录态、不泄漏 Cookie。测试夹具不得使用用户真实凭据。

必须分别保存状态：`CODE_PASS`、`PACKAGE_PASS`、`GUI_PENDING`、`GUI_PASS`、`BLOCKED`；必要时加 `SECURITY_PENDING`/平台子状态，但不得更改已有门禁的成功语义。证据包括 commit/OS build/Rust 和 WebView2 版本/产物 SHA/命令日志/截图/期望实际/未覆盖/责任人；CI 的 Linux PASS 不得写成 Windows GUI PASS。

## 8. 重大风险与回退

1. WebView2 不满足带回执与共同 Profile：XP-005A 阻塞，不开启 CDP/跳过权限；需调整 BrowserEngine 或缩减 R1 而不是宣称 Beta。
2. BFC 本地服务 Windows 不可用：仅记录客户端技术验证；不允许提交 R1 已完成。
3. Windows 文件安全能力不满足：关闭对应 Rust 命令，并验证无写入副作用；不要提供危险 fallback。
4. Linux 锁定布局行为回退：按 PROJECT-RULES.md 的已验证版本及差异截图回滚，不顺便重构 GTK。
5. CI 变更治理失败：同步治理索引并保留红灯；不可删除失败断言。
6. Profile 迁移/升级导致登录态丢失：停止发布，备份并恢复原目录；不得未经同意清除用户数据。
7. Agent 同一文件冲突：按任务隔离分支、单一负责人、代码所有权及 reviewer 签署；不得合并未审查 Diff。

## 9. 工作量与验收责任

V1 的 R1 约 6～9 人周、R2 约 13～20 人周仅作为初始假设；**XP-005A、BFC 服务调查和 Windows 文件安全验证完成后重新估算**。工程师负责原生实现证据，QA 负责目标 OS GUI/安装回归，高级模型负责关键安全合同和 Diff 评审，Owner 决定支持范围及最终功能开放。

## 10. Owner 必须批准的未决事项

1. Windows 10 22H2 是否保留“有条件兼容”承诺及 ESU 用户责任说明？
2. BFC 服务在 Windows 的供应、安装、版本锁定与维护人是谁？
3. R1 具体开放哪些书签/会话/本地能力？哪些原生命令必须彻底关闭？
4. XP-005A 选择哪个 Windows 环境及原生实现层，实测失败的备选方案是什么？
5. Windows 正式安装包是否使用当前用户 NSIS、自动更新是否纳入首发、签名和发布渠道如何管理？
6. R2 若 Agent/LSP 隔离仍未通过，是延后整个 R2 还是明示“R2 Core”？
7. 单任务分支与评审门禁如何映射到 GitHub 现有治理？

**所有未决默认不批准执行，不得由 Agent 自行猜测。**

## 11. 进入开发前必读和证据索引

按顺序阅读：`AGENTS.md` → `PROJECT-RULES.md` → [V1 审核](../engineering/reviews/2026-11-14-windows-linux-plan-audit.md) → 本 V2 → [V2 Backlog](../engineering/reviews/2026-11-14-windows-linux-implementation-backlog-v2.md) → 对应源码测试；涉及 CI/发布时还要 `docs/engineering/README.md`、`governance.json` 和相关工程目录索引。普通 Agent 不得因拿到任务就更改 `tauri-browser-tabs/**` 或 `src-tauri/capabilities/**` 原生危险区。

审查源码可在固定基线查看：
- https://github.com/code5m/browser-os/blob/d80f59b0c04457e743a5a18e6e8acae249ea7c6c/src-tauri/src/main.rs
- https://github.com/code5m/browser-os/blob/d80f59b0c04457e743a5a18e6e8acae249ea7c6c/tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/commands.rs
- https://github.com/code5m/browser-os/blob/d80f59b0c04457e743a5a18e6e8acae249ea7c6c/src-tauri/src/capabilities/bfc/commands.rs
- https://github.com/code5m/browser-os/blob/d80f59b0c04457e743a5a18e6e8acae249ea7c6c/src-tauri/src/capabilities/workspace/cas.rs
- https://github.com/code5m/browser-os/blob/d80f59b0c04457e743a5a18e6e8acae249ea7c6c/src-tauri/src/session.rs

本文件仅是**V2 候选设计**，任何工程变更须独立 task 授权与可核验证据。
