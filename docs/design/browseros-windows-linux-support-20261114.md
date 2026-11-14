# BrowserOS Windows / Linux 双平台支持调研与实施方案

- 文档日期：2026-11-14
- 状态：Proposal，供后续实施与评审使用
- 目标分支：`fix/bfc-readback-workbench-hardening-20261114`
- 代码调研基线：`41b0561b7e9358220eecea656344284a8a63793d`
- 关联任务清单：`../engineering/reviews/2026-11-14-windows-linux-implementation-backlog.md`
- 本文性质：源码级差距分析、目标架构、阶段计划与验收合同；不是 Windows/Linux GUI 已通过证据

## 1. 执行结论

当前代码以 Linux 为主实现平台。Windows 适配不是修改 `tauri.conf.json` 后同时生成两个安装包，而是一次有明确边界的 Windows 原生后端适配，并同时补齐 Linux 的发布矩阵。

核心判断：

1. Windows 当前不能按完整功能直接编译运行。多个无条件编译模块依赖 Unix Socket、Unix FD、信号、进程组、`/proc` 和 `bwrap`。
2. 本分支的 BFC 会话、票据、分页校验、本地导入和回执核对基本可以复用；Windows 的主要缺口是原生浏览器脚本执行、持久 Profile、页面事件和窗口生命周期。
3. Linux 已有主要实现和 DEB 构建链，但当前强制 X11，不能把配置中的 `bundle.targets=all` 当成所有发行版、所有包格式均已验证。
4. 推荐先交付 Windows BFC Beta，再补全双平台工作台能力；不得把暂时禁用的高级能力包装成完整 Windows 支持。
5. 实施时先建立双平台编译门，再打通 BFC 垂直链路，最后扩展进程、终端、数据库、Agent、宫格与系统集成功能。

## 2. 调研方法与证据边界

### 2.1 已执行

- 拉取并核对指定分支及上述提交。
- 比较该提交与 `origin/master`：分支增量集中在 6 个 BFC 文件。
- 检查 Tauri、Rust、浏览器插件、工作区文件安全、进程、终端、Agent、数据库、AI、CI 与打包配置。
- 运行现有聚焦契约测试，结果：
  - `BFC_NATIVE_CONTRACT=PASS`
  - `BROWSER_NATIVE_PROFILE_CONTRACT=PASS`
  - `BFC_NATIVE_LOAD_FAILURE=PASS`

### 2.2 未执行，不能据此宣称通过

- 未在 Windows 原生环境编译或运行。
- 未执行 WebView2 GUI、DPI、多显示器或崩溃恢复测试。
- 未使用真实 B 站账号做 Windows 回读验收。
- 调研环境没有 Rust 工具链，因此本轮没有新增 `cargo check/test` 结果。
- 本文工作量均为估算，不是承诺工期。

后续每个实施阶段必须追加对应平台的真实证据，不能复用本文作为 GUI PASS。

## 3. 当前平台差距清单

| 模块 | Linux 当前实现 | Windows 缺口与影响 |
| --- | --- | --- |
| Tauri/Vue 主界面 | 已有完整实现 | 前端大部分可复用，Rust 后端的 Unix 依赖先阻断完整构建 |
| 基础浏览器页签 | WebKitGTK 子 WebView | 通用 Wry 路径可复用，高级能力需要 WebView2 适配 |
| BFC 脚本执行 | Linux 原生回调返回结果 | 非 Linux 当前返回 `GRID_PLATFORM_UNSUPPORTED`，BFC 无法形成回读 |
| BFC 登录态 | 应用专属 WebKit Profile，权限 `0700` | 需要共享、应用专属、当前用户 ACL 的 WebView2 User Data Folder |
| 页面失败与资源事件 | WebKitGTK 导航、TLS、资源信号 | 需要 WebView2 导航、进程失败和必要响应状态映射 |
| 宫格子进程 | Unix Domain Socket、进程组、`waitpid` | 需要 Named Pipe、Job Object 和进程树回收 |
| 终端与脚本 | Bash/Sh、PTY、进程组和信号 | 需要 ConPTY、PowerShell、解释器支持矩阵与进程树终止 |
| 工作区安全文件 | `openat/openat2/renameat2`、Unix 权限与目录 FSYNC | 安全读取/移动存在非 Linux unsupported；普通 rename 不能满足 Windows CAS 替换语义 |
| Agent/LSP/MCP | `bwrap`、`poll/fcntl`、`rlimit`、Unix FD | 编译、IPC、进程控制和安全沙箱均需独立设计 |
| 数据库 | SQLite/MySQL/PostgreSQL | 代码对非 Linux 显式返回 `DB_NOT_SUPPORTED` |
| AI Provider | `/usr/bin/curl` | Windows 路径不可用；建议复用受控 Rust HTTP 实现 |
| AI 配置导入 | `/usr/bin/python3` | 需要 Rust 原生导入或明确的受控 Python 发现策略 |
| 默认浏览器/应用列表 | `xdg-settings`、`.desktop`、Freedesktop 图标 | 需要 Windows 注册应用、系统设置、开始菜单和图标解析 |
| 凭据 | Linux Secret Service | `KeyringStore` 抽象可复用，但需验证 Windows Credential Manager |
| CI 与安装包 | Ubuntu CI、DEB 构建证据 | 需要 Windows 编译、测试、NSIS、WebView2 检测与签名链 |

### 3.1 重点代码证据

- `tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/commands.rs`
  - 非 Linux `evaluate_script` 明确拒绝。
  - 持久 Profile 仅 Linux 配置。
  - 加载失败与资源扫描使用 Linux 原生信号。
- `src-tauri/src/main.rs`、`src-tauri/src/grid_process.rs`
  - 主进程与宫格子进程使用 Unix Socket 和 Unix 进程语义。
- `src-tauri/src/capabilities/workspace/safe_paths.rs`
  - Linux 使用句柄与 `*at` 系统调用防链接逃逸；非 Linux 部分能力显式 unsupported。
- `src-tauri/src/capabilities/workspace/cas.rs`
  - 非 Linux 路径使用普通 `fs::rename`，不能直接在 Windows 上保证已存在目标的 CAS 替换与目录持久化语义。
- `src-tauri/src/capabilities/agent/coding_lsp.rs`
  - 使用 `/usr/bin/bwrap`、`ldd`、Unix FD、`poll`、资源限制和进程组。
- `src-tauri/src/framework/ai.rs`
  - Provider 请求使用 `/usr/bin/curl`，导入使用 `/usr/bin/python3`。
- `src-tauri/src/capabilities/database/database.rs`
  - 非 Linux 显式平台降级。
- `.github/workflows/`
  - 原生发布与 GUI 证据以 Ubuntu 为主，没有 Windows 等价链。

## 4. 支持范围与交付层级

### 4.1 建议首发平台矩阵

| 平台 | 首发基线 | 暂不默认承诺 |
| --- | --- | --- |
| Windows | Windows 10 22H2、Windows 11，x64，标准用户，WebView2 Evergreen | 老版本 Windows、ARM、无 WebView2 且无法安装 Runtime 的环境 |
| Linux | Ubuntu 22.04/24.04，x64，X11 | Wayland、未验证发行版、无 Secret Service 的完整等价体验 |
| 包格式 | Windows NSIS；Linux DEB，随后 AppImage | 未经过实机验证的 RPM 或其他格式 |

### 4.2 R1：Windows BFC Beta

必须包含：

- 主界面和基础浏览器页签。
- 应用专属 WebView2 持久 Profile。
- BFC 登录探测、分页回读、partial、取消、结果未知和本地导入。
- 会话、书签等不依赖 Unix 高级运行时能力。
- 明确的原生能力协商与未支持功能提示。
- Windows 安装包和基础升级/卸载验证。

可暂不支持，但必须显式降级：

- 宫格、完整资源扫描、终端、脚本、数据库、Agent/LSP 沙箱等未完成能力。

### 4.3 R2：双平台完整能力

在 R1 基础上补齐：

- Windows 工作区安全文件后端。
- ConPTY 终端、受控脚本、Git 和 MCP 进程管理。
- SQLite/MySQL/PostgreSQL 分阶段验证。
- AI Provider 和配置导入。
- 宫格 IPC 与窗口生命周期。
- Agent/LSP 的 Windows 安全方案；未达安全合同前不得静默开放。
- Linux DEB/AppImage 与系统集成的发布证据。

## 5. 目标架构与不可破坏原则

### 5.1 统一平台接口，避免业务代码散落 `cfg`

建议逐步抽离以下接口，Linux 保持现有行为，Windows 增加等价后端：

| 接口 | 责任 | Linux | Windows |
| --- | --- | --- | --- |
| `AppPaths` | 数据、Profile、日志、缓存、临时与图片目录 | XDG 语义 | 应用专属 APPDATA/LOCALAPPDATA |
| `ProcessControl` | 启动、超时、进程树终止、回收 | 进程组、信号、`waitpid` | Job Object、进程树回收 |
| `IpcChannel` | 宫格帧传输与身份边界 | Unix Domain Socket | 当前用户 ACL 的 Named Pipe |
| `SecureFileSystem` | 安全读取、CAS、无覆盖移动、私有文件 | `*at`、Unix 权限、目录 FSYNC | Reparse Point 防护、ACL、Windows 替换语义 |
| `BrowserEngine` | Profile、脚本回执、导航与失败事件 | WebKitGTK | WebView2 |
| `SystemIntegration` | 默认浏览器、应用枚举、外部 URL | XDG/.desktop | 注册表、开始菜单、系统设置 |

实施时不允许把这些接口变成绕过现有 Owner、ACL、授权与能力边界的新旁路。

### 5.2 原生能力协商

增加只读的原生能力查询，至少区分：

- `browser.basic`
- `browser.persistentProfile`
- `bfc.readback`
- `browser.resourceEvents`
- `grid`
- `terminal`
- `scripts`
- `database`
- `agent.coding`
- `agent.lspSandbox`

前端依据结果显示、禁用或解释入口。不得让 Windows 用户点击后才遇到 Unix 路径或 unsupported 错误。

### 5.3 安全不变量

- 不读取或导出 B 站 Cookie，不为 BFC 开启远程调试/CDP 端口。
- BFC begin/status/finish 保持主窗口授权；submit 只来自限定 HTTPS B 站 tab。
- ticket 继续绑定 tab、账号 MID、TTL；导入继续核对 CSRF、回执和幂等语义。
- 不用普通文件读取替代防链接逃逸读取。
- 不用“先删除再写入”伪装原子替换。
- 不用 Windows Job Object 冒充 `bwrap` 等价沙箱。
- 不通过删除断言、跳过测试、复用 Linux 证据制造 Windows PASS。
- WSL/Git Bash 可作为用户可选环境，不能成为 Windows 基础版本的强制依赖。

## 6. 分阶段实施方案

### P0：支持契约与双平台门禁

工作：

- 冻结 R1/R2 支持矩阵、降级规则和测试矩阵。
- 增加 Windows experimental 编译任务；红就是红，不作为通过证据。
- Windows 编译转绿后再升级为 required gate。
- 将平台调研和 backlog 作为后续 PR 的引用基线。

出口：支持范围明确，后续任务可以引用统一合同。

### P1：平台底座与 Windows 编译

工作：

- 实现 `AppPaths`，清理生产代码中的 HOME、XDG、`/tmp` 和 Unix 字符串路径依赖。
- 隔离 Unix-only 模块，定义进程、IPC、文件、日志和系统集成接口。
- 保留 Linux 当前实现和行为。
- Windows 未完成能力显式 unsupported，不以危险 fallback 换取编译成功。

出口：Windows 编译通过，Linux 原有检查无回归；Windows required CI 可以启用。

### P2：WebView2 与 BFC 垂直链路

工作：

1. 用小型技术验证确认脚本执行回执、共享 Profile 和子 WebView 定位。
2. 为不受信任的浏览器子 WebView 配置共享、应用专属、当前用户 ACL 的 User Data Folder，并与可信主 UI 隔离。
3. 封装带结果的脚本执行：成功 JSON、注入失败、超时、导航上下文销毁必须可区分。
4. 映射导航、加载失败、TLS、必要的 HTTP 412、WebView 进程异常和 tab 关闭。
5. 复用现有 `BfcRuntime` 与本地 BFC 导入，不另写 Windows 业务状态机。
6. 验证 DPI、多显示器、窗口焦点、快捷键和崩溃提示。

出口：Windows 完成“登录 → 回读 → 导入 → 重启后登录态仍在”的真实垂直链路，形成 R1 Beta。

### P3A：Windows 工作台运行时

按风险分批：

1. reqwest 替代 curl、Credential Manager、SQLite，然后 MySQL/PostgreSQL。
2. ConPTY、PowerShell、受控 Python/Node 发现、Git/MCP 进程管理。
3. Windows 安全文件后端，覆盖 Junction、Reparse Point、UNC、ADS、长路径、保留名、文件锁与 ACL。
4. 宫格 Named Pipe 和 Job Object。
5. Agent/LSP AppContainer 或受限 Token 沙箱；未完成前返回明确 unavailable。

### P3B：Linux 发布补齐

工作：

- Ubuntu 22.04/24.04 X11 安装、启动、升级与卸载验证。
- DEB 首先正式交付，AppImage 独立验证后加入。
- Secret Service 不可用、锁定、无 D-Bus 会话的可操作错误提示。
- `.desktop`、单实例和外部 URL 路由回归。
- Wayland 单独立项，未解决子 WebView 定位前不取消现有限制并宣称支持。

### P4：安装包与 OS 集成

Windows：

- NSIS 当前用户安装，不默认要求管理员权限。
- WebView2 Runtime 检测与安装引导。
- 开始菜单、图标、卸载、文件关联和代码签名。
- 默认浏览器通过注册应用能力和系统设置引导，不假设可静默修改。
- 卸载默认保留用户数据，删除 Profile 必须显式选择。

Linux：

- 按包格式分别验证依赖、桌面条目、图标、单实例和升级行为。
- 未验证的包格式不列入正式支持。

### P5：双平台发布验收

- 每个 PR 执行前端、BFC 契约、Rust、路径安全、进程树与构建检查。
- 自动测试使用本地模拟 B 站页面和伪 BFC 服务，真实凭据不进入 CI。
- Windows 与 Linux 分别执行 GUI、安装、升级、压力和真实站点人工验收。
- 所有未支持能力有明确 UI 和文档说明。

## 7. BFC Windows 专项验收

至少覆盖：

- 未登录、已登录但 MID 不可识别。
- 单页、多页、部分页失败和分页不前进。
- 重复页、页内重复、跨页重复 BVID。
- 脚本注入失败、执行超时、导航上下文销毁。
- 页面切走、tab 关闭、错误 tab 提交、跨域提交。
- TLS 失败、HTTP 412/访问受限。
- 本地 BFC 未启动、CSRF 失败、导入拒绝、导入超时、结果未知。
- 重复完成、重复取消、导入中取消。
- 应用完整重启后的登录态持久化。
- 日志、BFC payload 和导入结果不包含 Cookie。

脚本执行失败不得误报为未登录；412/TLS/站点加载失败不得错误终止为“用户未登录”。

## 8. 双平台测试矩阵

| 检查 | Ubuntu | Windows |
| --- | --- | --- |
| 前端构建、Vue/TS 检查 | 每 PR 必须 | 每 PR 必须 |
| BFC 纯状态机与契约测试 | 每 PR 必须 | 每 PR 必须 |
| `cargo check/test` | 每 PR 必须 | 每 PR 必须 |
| 文件权限、链接与 CAS 测试 | 每 PR 必须 | 每 PR 必须 |
| 进程树终止与残留检查 | 每 PR 必须 | 每 PR 必须 |
| 安装包构建 | 必须 | 必须 |
| 原生 GUI | Linux 独立证据 | Windows 独立证据 |
| BFC 模拟页面端到端 | 必须 | 必须 |
| 真实 B 站账号 | 发布前人工/受控环境 | 发布前人工/受控环境 |

Windows 额外覆盖：标准用户、中文用户名、长路径、DPI 100/125/150/200、多显示器、WebView2 缺失、杀毒软件文件锁、无开发者模式。

Linux 额外覆盖：Ubuntu 22.04/24.04、X11、Secret Service 正常/锁定/缺失、DEB/AppImage 分别验证。

## 9. 风险与应对

| 风险 | 应对 |
| --- | --- |
| Wry/WebView2 脚本回执接口不完全满足 | P2 最先做 2–3 天技术验证；保持 BFC 安全合同，不用 CDP 绕过 |
| Windows 文件安全语义差异 | 独立后端与攻击测试，不用普通 canonicalize/rename 冒充等价 |
| WebView2 子窗口焦点、Z-order、DPI 差异 | 在 BFC 全量开发前验证基本窗口原型 |
| 进程树与孤儿进程 | 统一 ProcessControl，强制退出和异常退出都测试 |
| AppContainer 与 bwrap 能力不等价 | 未达合同前禁用 LSP 沙箱能力，不静默降级 |
| 真实站点和账号不稳定 | 夹具先行，真实站点只做受控发布验收 |
| 大重构破坏现有治理 | 小 PR；重命名、平台抽象和业务语义变更分开 |

## 10. 粗略工作量

以熟悉 Rust/Tauri 的开发者为前提：

- Windows BFC Beta：约 6–9 人周。
- 双平台完整能力：约 13–20 人周。
- 若 AppContainer 级 LSP 沙箱不进入首个完整版本，可减少约 2–3 人周。
- 两名开发加一名兼职 QA，理想情况下约 8–12 个自然周。

估算是资源规划输入，不是发布承诺；P2 技术验证和 Windows 文件安全是主要不确定点。

## 11. 完成定义

只有同时满足以下条件，才能宣称对应平台支持：

- 目标平台编译、测试、安装包和 GUI 证据齐全。
- Windows 不再存在未处理的 Unix-only 编译阻断。
- Windows BFC 不再返回平台 unsupported，且垂直链路通过。
- Windows 登录态重启保留，失败语义与 Linux 一致。
- 文件链接、权限、CAS 和进程树测试通过。
- 所有未实现能力均显式降级，文档与 UI 一致。
- 没有使用其他平台证据、静态检查或构建成功替代真实功能 PASS。
