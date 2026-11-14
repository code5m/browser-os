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
| Win32 文件 | 内核句柄校验的 safe read、拒绝 Junction/reparse 和目录逃逸；句柄相对 FileRenameInfo 无覆盖 move；单元/负例测试 | Windows 安全 CAS 保存仍 fail-closed，不提供不安全路径 fallback |
| 终端与脚本 | Windows PowerShell/ConPTY、Job Object 回收；审核后的 Node/Python 脚本允许 Job Object 调度、取消/超时，其他 shell 解释器拒绝 | Job Object 并非受限 Token 沙箱；Windows 长时间与多子进程压力测试待结果 |
| Agent 与 MCP | Windows 限定工作区只读 Agent 文件查看；skills list/read、knowledge_search 实现，敏感行隐去、跨目录拒绝 | Windows Agent 写文件/MCP 子进程/LSP 沙箱尚未安全实现，仍拒绝危险命令 |
| AI 与 OS | Credential Manager keyring、Windows 系统目录 curl.exe、可选 Python profile 导入；默认浏览器 ProgId 查看和经用户确认打开 Windows Settings | 真实 AI Provider/凭据生命周期及 OS 功能验收待结果 |
| BFC 本地服务 | Windows loopback health/session/CSRF、模拟 readback import receipt + 文件落盘、PowerShell foreground launcher；外部 bfc-workbench pin f98f280e8194e40df106d7176c92c9402215e330 | 真实 B 站授权会话及打包后置；曾发生仅 200ms 轮询导致偶发 async 测试超时，保留断言改为最多 5s 等实际完成 |

## 证据（按提交 SHA，不跨版本传递）

- 已有双平台 Cargo/check 和 BFC 服务 PASS 的历史 CI：https://github.com/code5m/browser-os/actions/runs/43588080899 ，只对应当时提交。
- 进入新版 Grid/文件/终端单元测试的 CI：https://github.com/code5m/browser-os/actions/runs/43612007737；早期 Windows test target 编译因 Unix-only 测试代码失败，已在后续提交修正，不等于该次运行 PASS。
- 当前分支源码对应 GitHub Actions：https://github.com/code5m/browser-os/actions/workflows/windows-linux-v2-experimental.yml
- Windows tests 包含 IPC framing、私有 Pipe roundtrip、Job Object、handle-read、handle-relative move、只读知识上下文、Agent workspace read 与脚本进程白名单；Linux tests 包含 Cargo check 与 wire framing。
- Windows/Linux 编译、单元测试、安装、GUI、BFC_REAL_LOGIN 是独立结果，不能互相替代。

## 尚未完成的高风险代码

1. Windows 文件 CAS 原子安全保存：须防止路径重定向/NTFS reparse/同名竞争；未有安全实现前 write_file_cas、save_if_unchanged 继续拒绝。
2. Windows Agent / LSP / MCP 的受限 Token / AppContainer 沙箱和相关进程/文件/网络权限负例：Job Object 只保证进程树回收，不代表沙箱。
3. Windows WebView2 针对 HTTP 412/TLS/进程崩溃/导航代际的真实回执以及真实 BFC 登录态，仍无 Win11 桌面验收。
4. 全部 R2 OS 集成、完整 AppPaths/历史 Linux 数据迁移及 GUI 功能矩阵尚未关闭；按用户要求安装与发布阶段暂不开展。

## 工程原则

同一 BrowserOS 集成分支持续提交代码和测试，不单独合并每个任务的 master；不禁用已有安全门禁，不绕过 BFC Owner/session/CSRF/tab 绑定，不导出 Cookie，不开启 CDP，也不自动修改用户 B站原收藏夹。未实现的高风险操作原生 fail-closed；未确认的 CI 状态保持 PENDING/FAIL，不虚构全部完成。
