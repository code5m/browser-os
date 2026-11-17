# BrowserOS Windows / Linux 真正跨平台审查与源码改动（2026-11-17）

> 审查范围：`code5m/browser-os`，`feature/browseros-windows-linux-v2-onepass-20261114`。只修改源代码与本仓库回归合同；按要求不运行 CI、不编译、不运行测试、不合并 master。macOS 没有可用的 Grid、进程与文件安全适配，不计为“支持”。

## 结论口径

“Windows 能编译”“某些 Windows 原生测试通过”“功能已补齐源代码”“Windows 安装版完整可用”是四种不同证据。最新已运行的 native 验证为 [run #201](https://github.com/code5m/browser-os/actions/runs/43981446235)，其中 23/27 个 Windows 过滤项通过，但 AppContainer、MCP、LSP 仍失败；该 run 不能证明本轮新源码通过。Linux CI 在该历史 run 中通过；本轮源代码也未重新验证。

## 已确认的真实跨平台缺口与处理

1. **应用启动器 Windows 空列表**：`list_apps()` 原来无条件遍历 `/usr/share/applications` 和 `.desktop`，在 Windows 没有价值。本轮将 Linux XDG 实现与 Windows 实现按平台编译分流。Windows 由可信系统 PowerShell 只读枚举 Start Menu .lnk 的直接 exe 目标，结果转换为现有 AppEntry；无法枚举时提供系统 Notepad/Paint 的有限退路。启动目标均复查真实文件与安全策略，不执行 .lnk 原有参数，不开放脚本/命令代理，不把 exe 伪装成图片。**限制：MSIX/UWP AppUserModelID 尚未实现，不是所有安装应用都一定被列出。**
2. **Windows shell 检测绕过/错误拒绝**：旧 `launch_basename()` 只分割 `/`，而 Windows 路径是 `\\`；旧名称规范化没有去掉 `.exe`。已调整原生路径处理、阻止 cmd/powershell 等 Windows 原生 shell 和高风险执行包装器、拒绝相对路径对当前目录的暗中依赖，启动时使用先前验证的 canonical 可执行文件而非再次搜索 PATH；同一路径经主窗口 Tauri 权限守卫。Linux 的 .desktop 解析与直接启动保留。
3. **实际文件管理命令绕过安全 CAS**：`write_file` 原来做路径检查后调用 `std::fs::write`；`create_file` 更是先写入再检查真实落点；`rename_path` 使用路径式重命名。已使读文件走受限的 handle-based 读取，更新文件使用内容比较和 pinned-parent CAS；新增文件先写 stage 再无覆盖重命名；重命名使用 Linux `renameat2(RENAME_NOREPLACE)` 与 Windows `NtSetInformationFile(FileRenameInformation)`，均绑定已授权父目录。普通文件删除亦复用句柄相对删除。递归建目录逐个创建和验证，拒绝 Junction/符号链接跳转。
4. **Tauri IPC 入口缺少主窗口守卫**：文件浏览/读取/预览/写入/新建/重命名等入口以及 `launch_app` 均增加可信主窗口来源核验；不通过时拒绝执行。现有前端直接调用的命令名不变，不额外增加前端参数。
5. **回归范围**：补充 Linux 和 Windows 共用的创建文件→不可覆盖→重命名→冲突拒绝→删除文件测试代码；新增 Windows Shell、路径解析单测与静态合同，不运行测试或 CI。

## 功能支持对照

| 领域 | Linux | Windows | 当前不能签署的范围 |
|---|---|---|---|
| Browser/Tauri 主界面 | WebKitGTK 已实现 | WebView2 已接入 | Windows 安装版会话、B 站真实登录和 GPU/WebView2 异常 |
| Grid 多窗口 | Unix Socket 原生实现 | NamedPipe + Job Object 原生实现 | GUI 交互和设备性能 |
| 终端 | PTY / Bash | ConPTY / PowerShell | 终端生命周期真实 GUI 回归 |
| 文件 CAS | pinned dirfd + renameat2 | pinned NT handle + 原生重命名 | 最新源码 SHA 未跑原生矩阵 |
| 系统应用 | XDG .desktop | Windows Start Menu 直接 exe | UWP/MSIX shortcut launch |
| Agent 文件事务 | Linux 适配 | Windows 原生文件操作已接入 | 与当前 UI 组合后的实机一致性 |
| MCP / LSP | Linux 实现 | AppContainer 代码已接入 | 上一轮仍 Win32 5，必须原生再验 |
| Script Runner | Linux Bash/Node/Python | Windows Node/Python | Windows Bash 不作为受支持能力 |
| BFC | 本地服务 | Windows synthetic loopback PASS | 真实账户授权与浏览器绑定 |
| 浏览器默认设置 | xdg-settings | 手势后打开 Windows 设置 | Windows 不可无授权写 UserChoice |
| macOS | 不适用 | 不适用 | 本轮不支持，不能宣传全三平台 |

## 尚存阻断，不得用“源码完成”代替验收

- **P1：AppContainer Node/MCP 与 rust-analyzer LSP**：run #201 仍出现 `CreateProcessW Access is denied (5)`。后续的 cmd /C、私有 LocalAppData 和运行时 stage 修改均尚未在目标 Windows 下验证。
- **P1：Windows GUI / BFC 登录态 / NSIS**：静态合同及历史原生 PASS 不足以验收安装包，尚未完成 Win11 安装版端到端。
- **P1：目录递归删除**：普通文件删除已采用 pinned handle；目录树删除仍调用标准库 `remove_dir_all`，在用户可写目录被并发重定位的威胁模型下仍需强化根目录绑定与负例。
- **P2：Windows 应用覆盖率**：Start Menu 只收集直指 exe 的 .lnk，不包含所有 UWP、MSIX 与协议快捷方式。
- **P2：启动性能**：Windows 首次加固 app-data ACL 和 WebView2 profile 树的成本、并发锁占用需在真机量化。
- **P2：跨平台 UI 一致性**：Windows 图标、默认应用名称、中文文件名/路径和多屏 DPI 需真机回归。
- **macOS**：主进程 GridStream/原生 Grid 只有 Linux/Windows 实现，未实现 macOS；不得标记支持。

## 统一验收条件（待用户单独批准）

在同一 SHA 上先跑一次 Windows 原生测试（包括 AppContainer 原生隔离、Node MCP stdio/背压、rust-analyzer LSP、CAS/新文件/rename、ConPTY、Grid）；Linux 原生和静态合同；再独立执行 NSIS 安装及 Win11 WebView2、真实会话、文件管理与 App Launcher 人工验收。验证前不修改 master 或降低正式门禁。

机器可读对照：[`windows-linux-parity-matrix.json`](./windows-linux-parity-matrix.json)。

## 第二轮：全仓库 114 个 Rust 后端文件扫描发现的附加缺口

- **Windows 资源面板不是真正的平台实现**：原 `read_proc_table()` 和 `mem_total_mb` 直接读取 Linux `/proc`，Windows 下返回空进程表/总内存 0。现已将 Linux /proc 与 Windows Toolhelp32Snapshot + GetProcessMemoryInfo 分离，Windows 物理内存总量取 GlobalMemoryStatusEx，主进程和 Grid 子树继续共用相同的汇总算法。
- **Windows 已存在配置文件更新失败**：数据库连接元数据 `database-profiles.json`、Agent 实验配置原使用 `fs::rename(temp, target)`，Windows 不能以此可靠覆盖旧目标；现统一到跨平台 `session::atomic_replace_file`，并取消 Windows 上错误的普通目录 `File::open(...).sync_all()`。官方配置路径走 Windows 私有 app-data ACL。
- **Git 冲突解决跨平台不一致**：Studio/Local Workbench 原通过 `std::fs::rename` 覆盖 Git 已跟踪文件；Windows 容易报错，且存在文件重定向风险。现复用授权根目录下的句柄安全读取与内容 CAS；保存冲突失败不覆盖先前内容。
- **图片存储**：原图片路径先 `create_dir_all` 再验证并 `fs::rename`，现先确认 workspace 根授权，再使用 pinned-parent 递归建目录及无覆盖原子新建文件，避免同 ID 图片被静默覆盖。
- **未在本轮消除的历史风险**：Git 其他分支操作与用户工作区写入、业务工作区 JSON、目录递归删除、Windows AppContainer Win32 5、Win11 安装版 GUI、macOS 原生进程/文件适配均需分别验证或继续实现。**新增实现均为源码补丁，本轮未编译/测试/CI。**

## 测试工具平台边界

- 既有 `native_58_gui_review` 和 `workbench_smoke` 的真实执行逻辑依赖 Linux XDG、`xdg-settings` 与 `/tmp`；在 Windows 上明确返回 Linux-only 错误，不再让其被误认为跨平台 Windows GUI 验收。Linux 默认诊断输出仍使用系统 `temp_dir`。
- Windows V2 原生测试/NSIS 安装/真实 WebView2 GUI 具有独立验收入口，但本轮**未执行**；Windows AppContainer MCP/LSP 在上轮实际跑出的 `CreateProcessW Win32 5` 尚未获得最终通过证据。
- 当前源代码已具备 Windows 资源监控、应用列表、CAS/文件管理、Git 冲突写回及数据库配置落盘入口；在同一 HEAD 上完成 CI 和实机 GUI 之前，所有这些新增修改只能标记为 **source implemented / unverified**。

## 第三轮：跨模块架构与稳定性收口（源码修改；无 CI）

本轮以远端基线 `b6027665113ba0a84572a03219a435fa9dc6efb1` 为准，在已有的 cmd.exe 原始命令尾、AppContainer 私有运行时、Windows 资源采集、文件 CAS 与应用发现修复之上收口，避免重写已有逻辑。

1. **LSP 启动容量与工作区作用域**：`SessionCache` 将全局 `epoch`/整数 `starting` 替换为按 UUID 启动令牌绑定的规范工作区根路径；`lsp_close` 无 session_id 时仅撤销当前工作区尚未完成的启动，其他工作区可以正常完成。预留令牌的 Drop 仍确保错误路径释放名额；在取消分支返回前显式释放 cache mutex，避免析构重入同一互斥锁。
2. **LSP 关闭与已有 Arc 竞态**：注册会话增加 `closed` 终止标志，关闭时在子会话锁保护下标记，再从缓存移除；即使并发请求此前已取得 Arc，也不得在已关闭会话上执行。指定 session_id 正忙时明确返回 `LSP_SESSION_BUSY`，不再冒充“关闭零项成功”。
3. **Agent 公共 API 路由一致性**：`dispatch_coding_request` 纳入 `lsp_start` 与 `lsp_diagnostics`，与 Windows native LSP 支持能力对应。Linux 不支持的 Windows 持久化会话动作仍返回显式不支持，不进行不安全模拟；Linux 原有 `lsp_request` 语义不变。
4. **MCP 配置路径**：对固定来源配置调用 Windows 的整链路 Reparse Point 防护，再读受限普通文件；禁止 `.opencode`、`mcp.json` 的符号链接/Junction 重定向后被当成合法配置。原有明确审批、keyring 引用、stdio AppContainer 与结果脱敏不变。
5. **AppContainer 生命周期与可维护性**：`lsp_status` 改为非创建式系统前提检查，不再每次只读状态查询就创建/删除 OS Profile；新增 `sandbox_verified:false`，明确尚未通过实际原生启动。System32 免暂存名单必须是 `System32` 直属目录的 `cmd.exe`/`curl.exe`，不能利用 `starts_with` 将子目录的同名可执行文件绕过私有暂存。
6. **防回归合同**：增加 LSP 工作区关闭代际、已关闭会话访问、公共入口路由、MCP 重解析点和无副作用状态查询的源码断言；增加工作区 A 的关闭不能取消 B 启动的单元测试代码，均未执行。

架构边界：Windows/Linux 共用授权和上层路由语义，平台隔离使用各自成熟原语（Unix FD/Namespace 与 Windows NT Handle/AppContainer）。共享边界集中在`public_api` 和明确的 session owner，不将 Unix-only shell/文件写入 fallback 偷渡到 Windows。

**状态：SOURCE_PATCHED_NOT_NATIVE_VERIFIED。** 按用户指令本轮没有调用编译、Cargo 测试、Node 测试、GitHub Actions 或 workflow_dispatch；不修改 master。此前 run #201 的 Windows native 失败，及当前 Windows MCP/LSP、安装包和 WebView2 GUI 未验，仍为 PENDING/FAIL；不可宣称平台已完全支持。

仍需进一步处理/验证的范围：Windows MCP/LSP AppContainer 实际 Win32 启动、真实依赖 DLL 及 stdio；递归目录删除的原子安全、UWP/MSIX 覆盖；Linux/Windows LSP 请求字段级统一、Windows NSIS 和 GUI、BFC 真实登录态。上述均不因源码收口自动变为 PASS。

## 第四轮：目录句柄安全、文档 LSP、UWP/MSIX 与稳定性代码收口（未跑 CI）

基准分支 HEAD：`72ad4df5d2ccc4a6868a658720338c4abd11f7a0`。用户明确要求不运行 CI、Cargo 编译/测试、NSIS、Windows 实机验收，不合并 master。

1. **SID 有效性与用户身份**：修复 `safe_paths_windows.rs` 中另一处遗漏的 `S-1-` 校验（旧逻辑拒绝字母 S），并修正含逗号 Windows 用户名的 CSV 解析。补充合法 SID/非法 SID 测试代码。
2. **安全递归目录删除**：`bridge/filesystem_commands.rs` 不再对已校验路径调用 `std::fs::remove_dir_all`。新增 `safe_paths::remove_tree`，Linux 采用已授权目录 FD + `fdopendir/readdir/openat/unlinkat(AT_REMOVEDIR)`，Windows 采用已授权 NT 目录句柄 + `NtCreateFile(FILE_OPEN_REPARSE_POINT)` + `SetFileInformationByHandle(FileDispositionInfo)`。列出文件名不等于授权；所有操作均由父目录句柄定位。不跟随 symlink/Junction 或特殊文件，深度与对象数上限 64/50,000。追加正常删除、拒绝删授权根、拒绝目录链接并保留外部文件的测试代码。并发改名仍以原句柄的对象身份为准；原子回滚整个树不在本轮合同内。
3. **Windows UWP/MSIX**：新增 `windows_apps.rs`，仅执行固定的无用户脚本 PowerShell `Get-StartApps` 读入已注册 AppUserModelID，限制输出 192 KiB、8 秒和最多 256 个结果；已注册且符合严格 AUMID 字符集才给 UI 展示 `appsfolder:` 标记。`launch_app` 先校验主窗口来源，再在异步工作线程复核系统登记表，使用受信任的系统 Explorer 打开 `shell:AppsFolder\\<AppID>`；普通 EXE 仍只走现有 `check_launch_target`，从不把 AppID 当 shell 命令执行。该实现不覆盖全部协议快捷方式/损坏的系统登记条目。
4. **LSP 平台 API 收敛**：Windows 保留现有工作区绑定持久 session；当 `lsp_request` 未带 `session_id` 时增加 Linux 风格的、明确审批的一次性文档请求。默认只暂存一个经验证的 `document.rs`；只有 `approved_module_sources=true` 时才允许扫描并暂存限定的模块源文件。与持久 session 共用最多两个分析器进程的容量预留；虚拟文档 didOpen / diagnostics 和 completion、definition、hover、documentSymbol 复用已有 AppContainer LSP IPC，结果只回映获授权文档 URI，其他文件 URI 隐去。返回 `document_hash`、`document_uri`、`scope` 和 `compilation_verified:false`。仍需 Windows 原生验证 rust-analyzer 的单文件附着与诊断时序。
5. **测量报告原子落盘**：M0 `m0_atomic_write` 改用现有 `session::atomic_write` 的唯一临时文件 + Windows replace + fsync；解决固定 `.tmp` 并发竞争与 Windows 目标已存在时 `fs::rename` 失败。
6. **工程防回归**：扩展 Windows/Linux 源码合同和纯模块回归用例。新增代码尚未编译或执行；不能将这些断言和文档标为 PASS。

**现阶段：SOURCE_PATCHED / NATIVE_UNVERIFIED；不合并 master，不触发 CI。** 旧版 run #201 的 MCP/LSP AppContainer Win32 5 和 Windows NSIS、Win11 GUI/BFC 真实授权会话仍无新 SHA 证据；`[skip ci]` 仅避免本批提交重复触发流水线，不取消正式门禁。


## 第五轮：文件新建与目录浏览安全收口（源码未验证）

基线 afbeb7a1。修复 write_file 只能覆盖不能新建的 API 行为差异：存在文件继续使用内容 CAS；不存在且无其他读取错误时使用授权父目录的原子无覆盖创建。新增目录浏览句柄化：Linux 使用已授权 dirfd + fstatat(AT_SYMLINK_NOFOLLOW)，Windows 使用已授权 NT 目录句柄 + 相对子句柄取元数据，并在目录身份被替换时拒绝输出。保留前端 DirEntry 合同并限制枚举最多 50000 项。

新增静态合同与原生回归用例代码，未执行编译、测试、Actions、NSIS 或实机验收。状态 SOURCE_PATCHED / NATIVE_UNVERIFIED；不应将历史 run #201 结果延伸到最新提交。本轮提交采用 [skip ci]；仓库多个 workflow 监听 feature push，不能未经 Actions 核查就断言零被动运行。

## 第六轮：集中收敛（同分支，源码修复，不运行 CI）

基线为 7d6730264afd21e33253c2f08f82cbaee0f3779d，仍使用 feature/browseros-windows-linux-v2-onepass-20261114；不新建分支、不合并 master。

### 本轮已经调整的源码

1. **Linux/Windows LSP 单次诊断**：diagnostics 允许省略行列号，兼容旧的 documentSymbol result；标准 documentSymbol 请求不再传多余 position 字段。completion/hover/definition 继续严格校验 UTF-16 光标坐标。
2. **Windows LSP 结果边界**：持久会话只将快照内部的文件 URI 回映到已授权工作区；外部/私有文件 URI 遮蔽，不区分 file scheme 大小写；单次请求也拒绝异常相对路径段。
3. **LSP 关闭与回收**：Linux 关闭不再等待全局会话中的繁忙 RPC，Linux/Windows 同步标记已关闭的会话并禁止旧 Arc 继续使用；批量关闭明确返回 skipped_busy_candidates 与 close_complete，不能把未关闭对象报告为已全部关闭。Windows 过期回收同样标记 closed。
4. **Linux 删除对象身份复核**：在 unlinkat 前用目录句柄下的 fstatat(AT_SYMLINK_NOFOLLOW) 复核 inode/dev，拒绝名称在预检后被明显替换；注意这不是无竞态原子删除，最后一次校验与 unlinkat 之间仍存在非事务性窗口。
5. **回归合同**：增加 Windows 文件 URI 脱敏单元测试代码、Linux inode 替换回归用例和静态源码断言；根据明确约束本轮均未执行。

### 唯一剩余验收清单（最后统一在同一个 HEAD 执行）

| 优先级 | 验收项目 | 必须提供的真实证据 |
|---|---|---|
| P0 | Win11 AppContainer CreateProcessW error 5；Node/MCP/rust-analyzer LSP 原生启动与沙箱隔离 | 同 SHA 的 Windows 原生运行日志、实际进程与失败码 |
| P0 | MCP stdio 背压/超时；Windows/Linux LSP 单次、持久、多工作区并发及关闭 | 原生用例实际输出，严禁继承旧 run #201 |
| P0 | Windows/Linux 文件创建/CAS、目录枚举、符号链接/Junction、递归删除与攻击性并发交换 | 同 SHA 安全回归的真实执行结果；失败不能报 PASS |
| P1 | Windows NSIS、WebView2 GUI 冷启动和会话持久化、真实 BFC 登录态、UWP/MSIX 启动器 | Windows 安装版人工+自动验收证据 |

本轮状态：SOURCE_PATCHED_ONLY / NATIVE_UNVERIFIED。没有调用 CI、Cargo、Node 测试、workflow_dispatch 或执行 Windows/Linux 原生验收，旧 run #201 不得作为新提交的 PASS。提交包含 [skip ci]，仍需要通过该 HEAD 的实际 Actions 列表确认有无被动运行。
