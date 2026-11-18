# BrowserOS Windows/Linux V2 补充源码审查与修复记录（2026-11-18）

> 基线：`8267dce55a9a0a25c6add523d0cc1fcc71a6933a`
>
> 目标：`feature/browseros-windows-linux-v2-onepass-20261114`
>
> 本轮仅静态源码审查与提交。**不得把源码修复宣称为编译通过、Windows 原生通过或完成交付**；不触发 CI、workflow_dispatch，不合并 master。

## 确定性问题与修改

| 优先级 | 模块 | 原问题 | 修改 | 证据要求 |
|---|---|---|---|---|
| P0 | Windows Rust LSP | 历史 Win Runner 的 `real_rust_analyzer_runs_against_private_snapshot` 失败，真正底层报错是 `failed to open NUL device for child stdin: Access is denied (os error 5)`；旧工程发现依赖 rust-analyzer 在无能力 AppContainer 中发起 Cargo/rustc 子进程 | 新建受限 `rust-project.json`，在 `initialize` 中显式指定 `linkedProjects` 并关闭 Cargo/sysroot 发现、proc-macro 与 flycheck；不再暂存 Cargo/rustc peers，原生进程权限保持不变 | Win11 相同 HEAD 中 rust-analyzer 启动、syntax diagnostic、symbols、hover/definition；确认不再发生 NUL 子进程错误 |
| P1 | Windows LSP 范围 | 不能把完全不执行 Cargo 的 LSP 结果宣称为编译与依赖语义的完整结果 | 明确 `compilation_verified=false`，限制工程根 <=32、扫描项 <=4000、源码由已授权快照形成；非 Cargo 模式会丢失部分 Cargo 依赖/类型语义 | 实机验证语义降级范围，前端需正确提示 |
| P1 | Windows MCP | MCP 输出脱敏只处理 Value::String，敏感 JSON 属性名可能直接返回前端 | 对 JSON Object 的 key/value 同步脱敏，避免秘密出现在对象键中 | 新增 `credential_in_json_key_is_redacted_in_nested_mcp_results` |
| P1 | Windows MCP | MCP 黑名单只拦截 shell 名称，未覆盖 rundll32、regsvr32、mshta 等 Windows 包装器 | 复用 security_policy 的原始名称 + 版本化名称黑名单，并对调用方路径、解析后真实路径两次检查 | 新增 `windows_lolbin_wrappers_are_rejected_without_spawning_sandbox` |
| P1 | Windows 文件系统 | `normalized_path` 对 `\\?\UNC\server\share` 直接去前缀，导致路径语义丢失 | 区分 verbatim UNC、verbatim drive、其他命名空间并保持根路径分隔符 | 新增 `windows_path_normalizer_preserves_drive_and_unc_root_semantics` |
| P1 | 静态回归 | 既有源代码合同比较不到上述边界 | 增加源合同断言；包括不在 LSP Start 链路中继续暂存 Cargo/rustc | 仅为静态断言，不能替代 Rust 编译或 Windows 安全测试 |

## LSP 功能合同（重要）

- 本轮的 Windows LSP 为 AppContainer 内的 **非 Cargo 静态工程分析**，提供 rust-analyzer 的 IDE/语法诊断、符号与部分语义服务；不读取宿主全局 Cargo registry，不在低权限沙箱里执行本机 Cargo/rustc，且不对 Rust 编译成功作任何声明。
- 对受审查工程的源码扫描不能越过已经创建的 private snapshot。碰到 reparse point、过大文件数、异常缺失 crate root，必须拒绝，不允许退回未授权宿主路径。
- 自动选择 `src/lib.rs`、`src/main.rs`、`document.rs` 及嵌套 crate 的同名根；没有它们时有限选取其他 Rust 源文件作为独立分析根。模块和跨 crate 的完整依赖图 **不能** 在这个模式下保证等价 Cargo metadata。
- **不要**为了解决 NUL 设备限制而放宽系统全局 ACL、增加 AppContainer 网络权限、跳出低权限容器，或在主机当前用户权限下直接运行未经审查的 Cargo。
- 快照 ACL 目前仅授权自身 package SID；由于已有历史 Windows ACL 拒绝证据，必须增加当前包 SID 正例和其他包 SID 负例的真实 Windows 测试，不能假定代码就已经安全可读。

## 本轮不能宣称解决的问题

1. **未测试：** Windows Rust 原生编译、Win11 AppContainer LSP 的真实成功路径、快照独立性与 NUL 错误是否消失。
2. **未测试：** Linux/Windows 全量测试；上一轮用户本地 `586 passed / 21 failed / 13 ignored` 的 21 条失败仍缺逐条根因。
3. **未测试：** ConPTY `0x80070006` 本地宿主；不能仅凭无窗口判断为环境问题。GitHub Runner 对指定 ConPTY 原生用例的历史 PASS 不代表本机。
4. **未测试：** Windows NSIS 构建、SHA256SUMS 实际产出、签名、WebView2 交互式桌面、BFC 真实授权与登录。
5. **剩余设计风险：** 句柄授权文件删除的最终 unlinkat 仍为按名称的操作；源码静态检查不能证明攻击性并发重命名完全原子。

## 最终验收原则

只针对最终冻结的同一个 HEAD 统一验证，依次执行 Linux Rust + Git Studio + 路径 ACL 测试、Windows 原生 27 项及新增负例、MCP/LSP 真实隔离测试、NSIS 打包、可交互 Windows GUI/BFC 验收。任何失败保持 FAIL，不使用历史 SHA 的通过结果代替最新提交。CI 由用户另行授权后再统一运行。
