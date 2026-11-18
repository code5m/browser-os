# BrowserOS Windows/Linux V2 安全收敛和真实验收清单（2026-11-18）

> Branch: feature/browseros-windows-linux-v2-onepass-20261114
> Baseline: 5d259c5cb4b8a60f9929ce665e05c5bcd89bed34
> 本轮仅提交源码、测试用例与 CI 打包合同；不执行测试、构建或任何 Actions dispatch，不合并 master。
> 已发现的宿主环境限制、尚未通过的运行测试及真正的产品缺陷必须分开列示。

## 1. 已提交的源码变化（SOURCE_PATCHED，非原生验收）

| 风险 | 位置 | 本轮修复 | 应由哪个验收证明 |
|---|---|---|---|
| P0 Linux 大小写兄弟目录误认为受信根 | security_policy.rs | OS 分流：Linux OsStr 精确比较，Windows 路径分量受控折叠，UNC/Verbatim 处理；缺失根不再回退字符串授权 | Linux 大小写兄弟目录、无效根、非 UTF-8 名称及根目录删除负例 |
| P0 Git Studio 越界读取/检查后替换 | git/studio.rs | 根目录归一化错误不吞掉；file 工作树与 snapshot 指纹读取走 safe_paths 已授权目录句柄，而不是路径式 fs::read | Linux 符号链接、大小写目录越界；Windows Junction/竞争测试 |
| P0 Windows LSP 源码快照被其他 AppContainer 读取 | coding_lsp_windows.rs | 不再往源码和祖先目录主动添加 ALL APPLICATION PACKAGES ACE；保持单一包 SID 授权 | Windows 不同 SID 交叉读取负例，自己 SID 正例 |
| P0 Snapshot “只读”虚假安全声明 | coding_lsp_windows.rs | 改为 private-source-copy 描述：Cargo 需要生成 Cargo.lock，目录具有 sandbox 可写授权，主机源目录没有挂载 | Windows Snapshot 内写入允许、宿主源码无更改、不同 SID 被拒绝 |
| P0 DLL 暂存碰撞 | windows_sandbox.rs | 不能以 DLL 长度相同就直接复用；现校验完整内容；whoami 仅在 test 编译下使用可信 System32 入口 | Windows 多工具链/同名异内容 DLL 测试，身份组探针 |
| P0 AppContainer 内 Rust NUL 设备不可访问 | coding_lsp_windows.rs | 不越权给全局 NUL 设备授权；测试环境识别真实子进程错误 LSP_CHILD_NULL_DEVICE_DENIED 避免 300 秒空等 | 需要更换为已验证无 NUL 依赖的 rust-analyzer 子进程启动方案，不能以本轮源码断言代替 |
| P1 NSIS 缺少 SHA-256 清单 | .github/workflows/windows-linux-v2-experimental.yml | 手动 workflow_dispatch 成功构建后生成 SHA256SUMS.txt 并核对哈希 | 真正的 workflow_dispatch 打包记录、安装包、hash 文件及系统 GUI |

## 2. 关键限制：Windows rust-analyzer 子进程仍存在 P0 阻断

最新基线的 GitHub Windows V2 工作流：
- Run 44245478085: windows-cargo FAIL，27 项中 26 项 PASS。
- 失败项：capabilities::agent::coding_lsp::tests::real_rust_analyzer_runs_against_private_snapshot。
- runner 实际 stderr：failed to open NUL device for child stdin: Access is denied. (os error 5)。
- Cargo 独立 direct-spawn locate-project / version / rustc version 成功，不证明 rust-analyzer 内部通过 std::process::Stdio::null() 生成的子进程能成功。
- 错误对应 Rust std Windows 在 Stdio::Null 情况下打开 Windows NUL 设备；这是 AppContainer 设备/进程模型问题，不是 Cargo.toml 解析问题。
- 这轮**没有**修改全局设备 DACL、没有把 LSP 迁出 AppContainer、没有为测试绕开沙箱。此问题需要经过受控 Windows 实机方案和同一 HEAD 的 native regression 验证，绝不能标记为已修复。

本轮将 whoami test-only 可信 System32 入口从 private runtime DLL 扫描中剥离，能提高 token 诊断能力；它不解决 Cargo 的 NUL 权限。

## 3. 正确的验收边界

### 之前的有效证据
- 基线 5d259c5: 5/6 GitHub Actions workflows PASS；Windows V2 专项 FAIL。
- Windows V2 4 jobs: static-contract PASS、linux-native PASS、windows-bfc-loopback PASS、windows-cargo FAIL。
- Windows 原生 27 项指定矩阵：26 PASS，rust-analyzer private snapshot FAIL。
- Windows 真实 ConPTY/Job Object/MCP stdio 的指定用例在 GitHub runner 曾经通过，不能断言所有 Win11 宿主都通过。
- 用户另提供全量 586 PASS/21 FAIL/13 IGNORED 与 security_policy 85 PASS、Git 63 PASS；21 项归因为环境仍需逐项证据。
- “Windows Runner 能启动部分 AppContainer 工具”不能外推 LSP/Cargo 子进程。
- 用户 Windows 磁盘余量：C: 16.83 GB、D: 40.41 GB（一次性快照，未测打包峰值）。

### 后续验收门禁（必须同一个新 HEAD）
1. Linux security_policy: case-sensitive sibling and non-UTF8; Git Studio 受控读取；目录循环/Junction 等回归。
2. Windows security_policy: verbatim disk、UNC、大/小写、根自身删除保护与 LOLBin (rundll32/regsvr32)；native file/CAS 竞争。
3. Windows AppContainer: 同 SID 私有源读取、不同 SID 拒绝、快照内写与宿主不可写、stderr 保密、DLL 冲突检测。
4. Windows LSP: rust-analyzer 真正能拉起 Cargo/rustc 并完成诊断与符号请求；明确 NUL 错误必须归零，不能跳过此门禁。
5. Windows ConPTY: 在出现 ERROR_INVALID_HANDLE 的宿主查 Pipe 创建句柄、OS build、输入/输出有效性，与 runner 结果对照；21 项逐条溯源。
6. NSIS: 按 workflow_dispatch 在相同 HEAD 生成 Installer + SHA256SUMS；签名状态单独核查，不能把 SHA-256 等同于代码签名。
7. Windows 真实桌面: WebView2 安装/冷启动/状态复用、BFC 真实登录、浏览器权限与 loopback、TLS/412/崩溃回执、卸载清理。

**状态:** 仅 SOURCE_PATCHED / NATIVE_UNVERIFIED。不得称完整修复、不得继承旧提交的 PASS、不得提前合并 master。
