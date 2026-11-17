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
