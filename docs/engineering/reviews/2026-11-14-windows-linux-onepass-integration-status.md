# Windows/Linux V2 单分支代码集成实况

日期：2026-11-14

- 唯一 BrowserOS 集成分支：feature/browseros-windows-linux-v2-onepass-20261114
- 原 BFC 基线：908a506bd1855ab97b4e0446a4c8321bc77ce853
- 决策：先完成代码开发及对应 Windows/Linux 编译、单元测试，再考虑部署/安装包/真实 GUI/账号验收；最后才对 master 做一次性合并
- 严格状态：PARTIAL_CODE_INTEGRATION / NOT FULL V2 PASS，任何有条件编译的后端不能凭设计文档冒充功能完成

## 已提交的真实跨平台代码

| 能力 | 代码实现 | 尚未验证或未完成 |
| --- | --- | --- |
| Windows WebView2 | eval_with_callback、结果上限、browser tab 共享独立 Profile、page load started/finished | 真实 B站登录态/DOM、导航失败与 412 细分 |
| Grid Windows | Windows Grid child 进程、PID 验证、owner-only Named Pipe、IPC seq 与 generation、CreateTab/UpdateRect/Navigate/JS/事件、Job Object 进程树回收和 crash replay | 整条 Windows Grid 实机/运行时验证；阻塞同步管道写取消还需加固 |
| Grid Linux | 保留 UDS/GTK 原有代码，IPC 增加长度上限与不丢帧 BufRead | 发行版 GUI 回归后置 |
| Win32 文件 | 内核句柄校验的 safe read、拒绝 Junction/reparse 和目录逃逸；句柄相对 FileRenameInfo 无覆盖 move；单元/负例测试 | Windows CAS 已实现 pinned-parent NT handle rename/replace，旧提交的原生测试已通过；新提交仍需复核 |
| 终端与脚本 | Windows PowerShell/ConPTY、Job Object 回收；Grid、Node/Python 使用 suspended spawn + Job；ConPTY 已切换 STARTUPINFOEX Job List 原子归属，其他 shell 解释器拒绝 | Job Object 并非受限 Token 沙箱；新增 ConPTY 整树测试须以最新 Windows CI 结果为准 |
| Agent 与 MCP | Windows 限定工作区只读 Agent 文件查看；skills list/read、knowledge_search 实现，敏感行隐去、跨目录拒绝 | Windows Agent 文件事务与 AppContainer MCP/LSP 已有原生实现，当前仍需要验证集成、运行时依赖和权限隔离 |
| AI 与 OS | Credential Manager keyring、Windows 系统目录 curl.exe、可选 Python profile 导入；默认浏览器 ProgId 查看和经用户确认打开 Windows Settings | 真实 AI Provider/凭据生命周期及 OS 功能验收待结果 |
| WebView2 Profile | 独立 persistent user-data folder；首次使用时修复并严格验证 Profile ACL；后续每次创建标签页只读核查根 ACL，并定期只读全树核验，不反复重置活动文件 | ACL 真实 Win11 GUI、WebView2 多进程锁文件及升级后 Profile 恢复仍需实机验收 |
| BFC 本地服务 | Windows loopback health/capabilities、每用户身份令牌、nonce-HMAC 证明、认证 session/import、PowerShell foreground launcher with no-follow owner/SYSTEM-only data ACL, per-object verification, and protected data-directory pointer；外部 bfc-workbench pin 5a37c20c8089533facbd61d23d3a2b67694d043b | 真实 B 站授权会话及打包后置；端口占用负例确保未经证明不发送 token 或快照正文 |

## 证据（按提交 SHA，不跨版本传递）

- 已有双平台 Cargo/check 和 BFC 服务 PASS 的历史 CI：https://github.com/code5m/browser-os/actions/runs/43588080899 ，只对应当时提交。
- 进入新版 Grid/文件/终端单元测试的 CI：https://github.com/code5m/browser-os/actions/runs/43612007737；早期 Windows test target 编译因 Unix-only 测试代码失败，已在后续提交修正，不等于该次运行 PASS。
- 当前分支源码对应 GitHub Actions：https://github.com/code5m/browser-os/actions/workflows/windows-linux-v2-experimental.yml
- Windows tests 包含 IPC framing、私有 Pipe roundtrip、Job Object、handle-read、handle-relative move、只读知识上下文、Agent workspace read 与脚本进程白名单；Linux tests 包含 Cargo check 与 wire framing。
- Windows/Linux 编译、单元测试、安装、GUI、BFC_REAL_LOGIN 是独立结果，不能互相替代。

## 尚未完成的高风险代码

1. Windows CAS 已具备 pinned-parent NT 原子替换及原生测试历史证据；新增代码仍须在同一最终 SHA 上重验，不能将历史 PASS 直接继承给当前提交。
2. Windows MCP/LSP 已基于 AppContainer 做无网络的受限执行；此次移除对宿主运行时 ACL 的永久修改，改为私有运行时副本并限制 DLL 拷贝，增加读写队列超时及 LSP 会话隔离。动态 DLL 依赖和沙箱访问仍需 Windows 原生回归，不能称作通过。
3. Windows WebView2 针对 HTTP 412/TLS/进程崩溃/导航代际的真实回执以及真实 BFC 登录态，仍无 Win11 桌面验收。
4. 全部 R2 OS 集成、完整 AppPaths/历史 Linux 数据迁移及 GUI 功能矩阵尚未关闭；按用户要求安装与发布阶段暂不开展。

## 工程原则

同一 BrowserOS 集成分支持续提交代码和测试，不单独合并每个任务的 master；不禁用已有安全门禁，不绕过 BFC Owner/session/CSRF/tab 绑定，不导出 Cookie，不开启 CDP，也不自动修改用户 B站原收藏夹。未实现的高风险操作原生 fail-closed；未确认的 CI 状态保持 PENDING/FAIL，不虚构全部完成。

## 2026-11-16 批量源码修复（待最终 Windows 验证）

- 源码提交：`7f1798e51784cff91e71a2fb970dd6de38bca507`。集中修改 AppContainer、MCP、LSP、WebView2 和 Windows CI。
- 不再向宿主安装的 `node.exe`、`rust-analyzer.exe` 添加 AppContainer RX ACE：非系统运行时复制到 profile 内的私有 `runtime` 目录，包含受限的邻接 DLL，沙箱获得私有副本只读执行权限。系统运行时保持 Windows 原生 ACL。
- AppContainer 构建 Windows 环境块时按大小写无关规范化键名，拒绝 PATH、SystemRoot、LOCALAPPDATA、HOME、TEMP、USERPROFILE 等保留变量被覆盖。
- MCP/LSP 输出管道发送改为有界等待，拥塞时返回显式错误而非静默丢失；LSP 写入同步队列也设同一请求截止时间，避免队列满时无限阻塞。
- LSP 新建会话先预留容量，释放全局锁再执行初始化；关闭所有会话时使未完成启动失效。每个 LSP 请求更新自己的截止时间，忙碌会话快速返回。
- WebView2 Profile ACL 首次修复、后续根 ACL 每次检查、每 10 分钟只读全树巡检，避免每个标签页递归重置真实登录缓存中的大量活动文件。
- Windows workflow 先编译一次测试可执行文件，再复用它运行完整过滤矩阵；手动 `workflow_dispatch` 额外执行 WebView2 插件 ACL 测试，原正式门禁不撤销。
- 本次代码阶段未执行任何测试、编译或 CI，提交信息含 `[skip ci]`。**结果状态：CODE_PATCHED / NOT VERIFIED；不合并 master、不宣称 R1/V2 PASS。**
