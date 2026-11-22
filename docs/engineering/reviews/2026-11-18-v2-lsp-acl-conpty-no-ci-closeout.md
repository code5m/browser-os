# BrowserOS Windows/Linux V2 — LSP、AppContainer、ConPTY 源码收敛记录

基线：`bd7cf45a0697192a2883d646d78897f0fc70104a`
目标分支：`feature/browseros-windows-linux-v2-onepass-20261114`
限制：不新建分支、不合并 master、**不运行或主动触发 CI/Actions**。本轮仅通过 GitHub 审查和提交源码，远程 Windows 设备离线。下述测试为**新增测试代码，未运行**。

## 一、确认的设计缺陷与修复

| 范围 | 源码位置 | 根因/风险 | 已提交源码修复 |
|---|---|---|---|
| P0 Snapshot 目录仍可写 | `src-tauri/src/capabilities/agent/windows_sandbox.rs` | 迁移非 Cargo `rust-project.json` 后，无需 Cargo.lock，但 `protect_snapshot_files` 仍给低权限 SID 的目录 `(OI)(CI)M`；目录可新建/移动/删除条目，与只读源码快照不一致 | 改为 `(OI)(CI)RX`；将可写数据限定在单独 `state` 目录 |
| P0 跨 SID 负例不具备阳性对照 | `coding_lsp_windows.rs` | 原测试只断言另一 SID 读取失败，假如该 SID 的进程本身未正常工作，也可能误判安全 | 新测试要求另一 SID 在同一个进程中先读取其**自己的快照**，再尝试读取首个 SID 的快照；只有“自己的可读、别人的不可读”才算通过 |
| P0 不能证明快照不可修改 | `coding_lsp_windows.rs` | 只测读，不测自身包低权限 SID 修改 | 增加同 SID 尝试修改源码文件的负例，核对退出码、快照字节和宿主工作区字节 |
| P0 Windows LSP 验收缺少 hover/definition | `coding_lsp_windows.rs` | 先前只有空数组诊断和 documentSymbol 判断，且 Cargo 工具探针与主用例混在一起 | 在同一原生测试中覆盖 `DIAGNOSTICS`、`SYMBOLS`、`HOVER`、`DEFINITION`、`SYNTAX_DIAGNOSTICS`、`CLOSE`；断言私有路径不会暴露。旧 Cargo 子进程探针独立为显式 ignored 用例，不再污染非 Cargo 模式验收 |
| P1 ConPTY Pipe 生命周期 | `src-tauri/vendor/portable-pty/src/win/psuedocon.rs` | CreatePseudoConsole 的两个宿主 Pipe 句柄在函数返回时就被释放，但微软文档要求等客户端 CreateProcess 创建完成后再释放，过早关闭可能影响 I/O 生命周期 | 新增 `creation_pipes` 句柄所有权，首个客户端成功创建后释放；失败保留供重试，最终对象释放时自动关闭 |
| P1 ConPTY 失败可诊断性 | 同上 | E_HANDLE 无法区分传入句柄已经失效、尺寸非法或底层宿主故障；误归因“父 Shell 无标准句柄” | CreatePseudoConsole 前校验独立管道句柄，拒绝非正尺寸；失败时报告 HRESULT、两个 Pipe 句柄是否仍有效、尺寸（不输出实际句柄值） |
| P1 ConPTY 尺寸与加载路径 | `conpty.rs`、`psuedocon.rs` | `u16 as i16` 对大尺寸会溢出；相对路径加载 `conpty.dll` 可能从不可信工作区搜索 | 严格限制 rows/cols 为 1–32767；仅尝试加载当前可执行文件同目录下的非链接 conpty.dll，否则使用已知 Windows kernel32 导出，拒绝工作区控制 DLL 搜索 |
| P1 回归门禁 | `src-tauri/src/capabilities/terminal/terminal.rs`、`tests/check-windows-linux-v2-source-contract.mjs`、Windows V2 workflow | 缺少上述生命周期、安全合同和失效路径的回归监测 | 新增终端尺寸负例、源合同断言和 Windows 原生矩阵过滤项（未来运行时才执行） |

## 二、原生 Windows LSP 的逐条验收要求

原生测试必须由一个冻结 SHA 的 Windows 主机执行：`real_rust_analyzer_runs_against_private_snapshot` 应确认：
1. AppContainer 创建成功且无网络权限，不通过降低隔离级别或主机进程替代。
2. rust-analyzer 读取由授权代码生成的 `rust-project.json`，不再因内部 Cargo/rustc 打开 `NUL` 设备而超时。
3. `diagnostics` 至少确认结构正确，对故意无效语法的 Rust 模块报告非空解析诊断。
4. `documentSymbol` 发现两个不同函数。
5. 对本地函数调用的 `hover` 返回函数符号、`definition` 返回工作区内目标 URI，不能显示私有快照绝对路径。
6. `lsp_close` 清理会话，防止已关闭会话继续使用。
7. 显式返回 `analysis_mode=rust-project-no-cargo` 和 `compilation_verified=false`，不把此模式误称为 Cargo 编译验证。

旧证据：GitHub Windows V2 run #44245478085 发生在 **5d259c5**，`windows-cargo` 的 27 项矩阵中 26 PASS / 1 FAIL，失败项为 LSP 私有快照。该历史结果不适用于本轮新 SHA。

## 三、AppContainer 跨 SID 验收

`private_snapshot_requires_own_package_sid_and_rejects_other_profiles` 必须得到同一测试中的三个控制结果：
- SID A 读取自己的快照成功；SID B 读取自己的快照成功、读取 A 的快照失败；
- SID C（自身 Package SID）试图修改快照内源码文件失败，字节未变化；
- 宿主工作区未发生读写副作用。

**源码权限更改不是原生执行证据。** 所有测试失败必须计入 FAIL，不允许将进程创建失败误判为跨 SID 隔离成功。

## 四、ConPTY 与“剩余 21 项”的证据边界

本地已知 `CreatePseudoConsole` 返回 HRESULT `-2147024890` = `0x80070006` = `ERROR_INVALID_HANDLE`。源码使用的是两条独立 `Pipe::new()` 创建的句柄，而不是直接继承父 Shell 标准句柄；故“无交互控制台会话必然失败”不构成最终根因。

本轮修复了**可以从源码明确确认**的 Pipe 句柄过早释放、尺寸溢出和工作目录 DLL 搜索风险，并增加更精确的错误信息。至于本地这一次 `E_HANDLE` 是否被这些修复消除，必须在出现问题的宿主上复现并以新日志确认，不能保证其根因相同。

用户先前提供全量 `586 passed; 21 failed; 13 ignored`，但当前聊天记录、GitHub 分支和已连接远程设备均没有完整 21 条失败**名称、断言、stderr**。目前只有 ConPTY 错误以及 `framework::ai`、`database`、`draft_journal`、`workbench` 等模糊模块范围。没有这些证据，不能把其他模块全部标为“环境问题”，也不能准确逐项下手。需要用相同 Windows 主机、相同测试命令与同一 SHA 的逐项输出分类（代码缺陷、宿主限制、全局并发/端口冲突、测试代码问题）。

## 五、当前不做的操作

- 不运行 Cargo/PowerShell/ConPTY 本机测试、不触发 CI/Actions、不执行 NSIS/GUI 测试；
- 不合并 master、不更换 branch、不使用没有真实来源的 “21/21 已修复” 结论；
- 不扩大 AppContainer 权限，不修改全局 Windows NUL 设备 ACL，不把 LSP 移出沙箱。

**状态：SOURCE_PATCHED / NATIVE_UNVERIFIED**。任何 PASS 需要来自将来的同一目标 SHA 原生执行证据。
