# Windows / Linux V2 单分支集成记录（2026-11-14）

- 集成分支：`feature/browseros-windows-linux-v2-onepass-20261114`
- 唯一上游基线：`fix/bfc-readback-workbench-hardening-20261114` 的 `908a506bd1855ab97b4e0446a4c8321bc77ce853`
- 目标：所有 XP 任务可在同一集成分支持续提交，最后才评审合并 `master`；**不为每项 XP 任务单独合并 master**。
- 状态：**PARTIAL IMPLEMENTATION / NOT VALIDATED**。不可宣称 Windows R1、R2 或跨平台构建已完成。
- V2 方案：[设计](../../design/browseros-windows-linux-support-v2-20261114.md) · [Backlog](./2026-11-14-windows-linux-implementation-backlog-v2.md)

## 已写入源码的部分（本次集中提交）

| 关联任务 | 改动 | 意义 | 验证现状 |
| --- | --- | --- | --- |
| XP-011R1 | `workspace/cas.rs` 删除非 Linux 的路径式写入回退、在 `write_file_cas` Rust 命令入口拒绝，补负例测试 | 未实现 handle-based CAS 的 Windows 不能通过原生 API 写入，避免 TOCTOU 风险 | 静态复核；Windows Rust/GUI 未运行 |
| XP-011R1 | `workspace/fs_cmds.rs` 在非 Linux 的 `move_path` 原生命令入口明确拒绝 | 不依赖前端隐藏功能，未实现 Windows 安全移动之前不产生文件写入 | 静态复核；Windows Rust 未运行 |
| XP-006（部分） | 浏览器插件为 Windows 新增应用专属稳定 User Data Folder，按 WebView2 目录语义复用；Linux 原 Profile 保留 | 为 B 站网页重启后登录态提供持久化基础，浏览器页与主可信 UI 分离 | Windows ACL、WebView2 选项冲突、重启真实验证待完成 |
| XP-008（部分） | 非 Linux 导航事件改由 Tauri `on_page_load` 发出 Started/Finished，不再用 `on_navigation` 伪装 Finished | 消除“导航已批准=页面加载完成”的错误上报 | Windows 原生导航、TLS、412 及崩溃事件仍待验证 |
| XP-011R1（部分） | `session.rs` 将固定 `json.tmp` 改为 `create_new` 唯一同目录 tmp + `sync_all`，rename 失败清理 tmp；沿用 `.json.tmp` 后缀兼容旧恢复逻辑 | 避免多个落盘调用共享同一暂存文件；减少崩溃时丢失数据风险 | 文件替换/目录持久化/Windows 真实锁冲突未验收 |

## 尚未完成（必须如实标记）

- **XP-000/001/002/003/004：** 尚未完成最终范围批准、完整平台清单、Windows experimental CI、全仓 AppPaths 及 Unix 专用模块编译隔离。当前仓库其他模块仍有 Unix-only API，Windows 构建尚未证明可通过。
- **XP-005A/005B/007：** 尚无独立真实 Windows WebView2 原型、原生带结果的 JavaScript 回执通道。现有插件非 Linux `eval_result` 仍返回 `GRID_PLATFORM_UNSUPPORTED`，BFC Windows 全链无法完成。
- **XP-008/009/010：** 尚无 Windows 完整导航失败语义、BFC 本地服务的 Windows 部署合同、B站真实站点导入/重启验收。
- **XP-011（完整）与 XP-012～020：** Windows handle-safe 文件系统、Job/Named Pipe、ConPTY、凭据/数据库、Agent sandbox、NSIS 安装、Windows 实机验收、Linux 发行矩阵与最终发布准入均未完成。
- **未执行：** Linux Rust/Cargo 编译与单元测试、Windows Cargo 编译、安装版 GUI、真实用户登录、BFC 本地服务 Windows 验证、B 站写回操作。

## 为什么不跳过门禁

“只合并 master 一次”是减少 Git 集成成本，不意味着跳过安全验证。禁止把 Linux 编译当 Windows 编译、把源码改动当 Windows GUI PASS。GitHub API 可以直接提交源码，但**不能凭此证明 Windows 本机 WebView2、安装版、登录态和进程隔离正常**。

此分支用作集成，不调整 master 现有规则，也不删除已有检查、降低 BFC Tauri ACL、增加 CDP/Cookie 导出或 B 站写回权限。提交后如发现 Windows build/GUI 风险，保持能力 unavailable/blocked，不能静默放开。

## 合并条件

只有 Windows 11 x64 与 Ubuntu X11 的各目标能力经过真实机器验证，Windows BFC 服务及 Profile/JS 回执形成闭环，安全和安装/升级回归可核验时，才对 `master` 做**一次性集成评审**。如用户决定缩减为 R1 或 R2 Core，先明确支持范围与未交付能力，再进行一次性合并。
