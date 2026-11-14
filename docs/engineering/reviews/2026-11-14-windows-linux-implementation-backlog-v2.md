# BrowserOS Windows / Linux V2 实施 Backlog、任务依赖与 Agent 交接

- 日期：2026-11-14
- 状态：**V2 Proposed；不得把任务卡等同于 Owner 已授权开发**
- 基线：`d80f59b0c04457e743a5a18e6e8acae249ea7c6c`
- 设计：[V2 实施方案](../../design/browseros-windows-linux-support-v2-20261114.md)
- 审核：[V1 只读评审](./2026-11-14-windows-linux-plan-audit.md)
- 保留：[原版 Backlog](./2026-11-14-windows-linux-implementation-backlog.md)
- 原则：**保留 XP-000～XP-020 任务 ID**，通过 XP-005A/B、XP-011R1、XP-018A/B 补充子任务，修正依赖和验收门禁。

## 1. 不可破坏的任务执行规则

1. 领取任务前依次阅读 `AGENTS.md`、`PROJECT-RULES.md`、审核报告、V2 设计与本 Backlog、相关源码和测试；CI/发布另读 `docs/engineering/README.md`、`governance.json`、工作流目录。
2. 先读、再提设计、再获授权、最后写入；文档标记 Proposed 的内容不能被模型默认为批准的需求。首次“设计与只读探查”不代表生产代码写入授权。
3. 每个 XP 子任务独立分支、单一负责人、独立审核人和文件所有权；不得多个 Agent 共同修改相同文件、并行 push 当前 BFC 分支或直接合并 master。
4. 一项任务必须冻结目标、非目标、允许/禁止修改、调用合同、安全不变量、测试步骤、回滚方式、证据链接。未知先标 BLOCKED，不能自行创造成功语义。
5. PR 推荐小而可独立验收，150 行是**默认拆分提醒**而不是硬性工程上限；对 ACL、原子文件操作、IPC 时序等不可切碎的实现，允许经评审保留较大原子 Diff。
6. 不导出或读取 B 站 Cookie，不启用 CDP/远程调试，不绕过 Tauri ACL、主 UI Owner、MID/TTL/tab 绑定，不改 BFC 导入成功/partial/unknown 的现有含义。
7. Windows 不可用能力在 Rust 原生入口 fail-closed；前端隐藏按钮不是授权机制。Linux 已锁定的 WebView 位置、隐藏、GUI/会话关闭行为不得修改。
8. 不能删断言、跳测试、隐藏红色状态或用 Linux PASS 冒充 Windows PASS；静态、编译、安装、GUI、真实 BFC 验收五类独立出证。
9. 添加 GitHub Actions workflow 时同步调整治理注册文件并执行 `npm run check:engineering-governance`；不要修改历史 workflow 或其成功条件来制造绿色。
10. 完成只提交 Diff + 测试/平台证据供审核；未明确授权不合并、不发布、不安装到用户系统。

## 2. V2 任务完整清单

| ID | Owner 建议 | 依赖（必须先完成的任务） | 交付与安全边界 | 最低准入证据 |
| --- | --- | --- | --- | --- |
| XP-000 | 高级模型 + Owner | 无 | 冻结 Windows 11/Linux R1/R2、BFC 服务依赖、能力 manifest、R1 原生拒绝清单、Owner 未决项 | 范围、API/错误码、服务契约和测试责任签署 |
| XP-001 | 普通模型，只读 | XP-000 初步范围 | 扫描 Rust/TS/CI 中 Unix fd、signal、/proc、路径、bwrap、进程/文件 fallback、生产与测试分类 | 可查找清单，文件、行号、调用者、风险、建议 OS 后端 |
| XP-002 | 普通模型，高级复审 | XP-001，XP-000 治理合同 | Windows experimental Rust/构建 CI，产物和失败日志，不影响现有 required PASS | 真实 Windows runner 执行红绿；workflow 注册治理同步 |
| XP-003 | 高级设计 + 普通机械实施 | XP-001，XP-000 数据合同 | AppPaths、Linux 旧数据兼容、Windows appdata/profile/log/notes；不改变现有路径意义 | 路径映射表、旧数据读写/恢复、中文用户名/临时路径测试 |
| XP-004 | 高级模型 | XP-002、XP-003 | Unix-only cfg 隔离、Windows 最小编译与 unsupported 后端；不重写业务状态机 | Windows cargo check/test 真实记录；Linux cargo/test 无回归 |
| XP-005A | 高级模型 + Windows 环境 | XP-000 最小权限/POC 合同 | **独立** WebView2 子视图/Profile/脚本结果及失败语义 POC；不改生产源码 | Windows 11 GUI 录像/截图+环境版本+日志；失败阻塞接入 |
| XP-005B | 高级模型 | XP-004、XP-005A | 把已验证的 WebView2 原生能力集成插件与 BrowserRuntime；严格小范围 | 真实 Windows 与 Linux 子 WebView 回归 |
| XP-006 | 高级模型 | XP-005B | 共享当前用户受限 UDF、主 UI 隔离、环境一致性和重启保留 | 多 tab、两次启动、权限/卸载保留、无 Cookie 导出 |
| XP-007 | 高级模型 | XP-005B | 带结果 JS 通道：success/error/timeout/navigation_destroyed、tab generation、结果大小限制 | 错误不误报登出、旧执行回执隔离、结果限制 |
| XP-008 | 高级模型 | XP-005B、XP-007 | nav requested/committed/completed/failed、TLS/412 可观察性、进程崩溃、tab 关闭 | 事件时序/代际、失败不伪造 412 或 logged_out |
| XP-009 | 普通测试 + 高级威胁复审 | XP-006/007/008，本地 BFC 服务合同 | 双平台 DOM 模拟、伪服务、partial/duplicate/412/CSRF/timeout/unknown | Rust/JS 自动测试+Windows 真实 BFC 手动验证计划 |
| XP-010 | 普通验证 + 高级终审 | XP-009、XP-011R1、XP-018A、XP-000 服务合同完成 | Windows BFC Beta：安装→登录→回读→导入→重启保持登录；失败提示 | 全链 Windows GUI/真实本地服务/安装证据，Owner 决定 R1 |
| XP-011R1 | 高级模型 | XP-000、XP-004 | **前移安全门**：禁用未验证写入/移动/读取；验证 R1 实际使用的会话/书签/配置原子性和升级保留 | Rust 直接调用拒绝测试；Windows 覆盖/锁/崩溃恢复 |
| XP-011 | 高级模型 | XP-003/004，XP-011R1 | Windows 完整 SecureFileSystem：句柄锚定、重解析/ACL/CAS/无覆盖移动 | Junction/ADS/UNC/长路径/并发/目标替换攻击矩阵 |
| XP-012 | 高级模型 | XP-004 | Windows Job Object + Linux 进程组 ProcessControl；不冒充沙箱 | 正常/超时/强杀/主进程死后无孤儿；负例与残留 |
| XP-013 | 高级设计/普通实现 | XP-012 | ConPTY、PowerShell、受控解释器发现；WSL 非必需 | 中文路径/编码/resize/kill/取消/失效 |
| XP-014 | 高级设计/普通实现 | XP-003、XP-012 | AI Provider 使用受控 Rust HTTP，Rust 配置导入或 Python 合同；密钥仅系统库 | HTTPS/streaming/cancel/超时/无 secret 日志 |
| XP-015 | 普通实现 + 高级复审 | XP-003、原生能力合同 | Windows Credential Manager、SQLite 先行，外部数据库分别放行 | 真实驱动、TLS/凭据、权限/超时/行数限制 |
| XP-016 | 高级模型 | XP-005B、XP-012 | Grid Named Pipe ACL、代际、窗口生命周期、进程树 | 子进程身份校验、崩溃/重连、无错发/无孤儿 |
| XP-017 | 高级模型 | XP-011、XP-012 | 受限 Token/AppContainer 等安全模型或明确 unavailable；Job 不代表 sandbox | 隔离/权限逃逸负例、威胁评审通过才开放 |
| XP-018A | 普通实施 + 高级终审 | XP-004，最低运行环境合同 | **早期最小 NSIS 原型**：当前用户、WebView2 检测、release 产物 | Windows 标准用户安装/启动/卸载，用户数据默认保留 |
| XP-018B | 普通实施 + 高级安全审查 | XP-018A、适用的 R2 功能 | 完整 OS 集成、签名/更新/注册应用与默认浏览器系统设置引导 | 不静默改默认浏览器、安装/升级/回退/签名证据 |
| XP-019 | 普通模型 + Linux QA | XP-000（可与 P2 并行） | Ubuntu 22.04/24.04 X11 DEB/AppImage、Secret Service、图标/单实例 | 每包格式和 GUI 独立证据；Wayland 不虚报 |
| XP-020 | 高级终审、QA + Owner | 对应目标能力与 XP-010/018B/019 | Windows/Linux 分平台发布验收与已知限制 | CI、GUI、安装、压力、真实 BFC/OS、来源 SHA 独立签署 |

**XP-011R1 是提前的一道最小安全门，不意味着 XP-011 完整功能提前全部完成。XP-018A 仅证明安装原型，不能替代 XP-018B 正式集成。**

## 3. 依赖与任务冲突（建议真值）

~~~mermaid
flowchart TD
  A["XP-000 范围和服务合同"] --> B["XP-001 源码矩阵"]
  A --> Z["XP-005A Windows POC（独立）"]
  B --> C["XP-002 Windows experimental CI"]
  B --> D["XP-003 AppPaths"]
  C --> E["XP-004 平台编译"]
  D --> E
  E --> F["XP-005B 原生集成"]
  Z --> F
  F --> G["XP-006 Profile"]
  F --> H["XP-007 script result"]
  H --> I["XP-008 navigation/errors"]
  G --> J["XP-009 BFC E2E"]
  I --> J
  E --> K["XP-011R1 file/data safety"]
  E --> L["XP-018A NSIS prototype"]
  J --> M["XP-010 R1"]
  K --> M
  L --> M
  E --> N["XP-012 ProcessControl"]
  K --> O["XP-011 SafeFS R2"]
  N --> P["XP-013/014/016/017"]
  O --> P
  M --> Q["XP-018B Windows integration"]
  A --> R["XP-019 Linux release"]
  P --> S["XP-020 release gates"]
  Q --> S
  R --> S
~~~

注：图显示关键路径而非所有 XP 的细节前置，**任务表依赖列为准**。XP-014/015 可在满足对应自身依赖后独立并行，不要求所有 R2 模块同时完成。XP-020 是对应发布目标的验收门，不应在 R1 发布时错误要求 R2 所有能力提前完成。

## 4. 第一批 Agent 的实际领取方案

| Agent | 先领任务 | 第一轮允许 | 不允许 |
| --- | --- | --- | --- |
| 高级 Agent 0 | XP-000 | 产出目标/非目标、BFC 服务提供方和原生授权合同草案 | 未经 Owner 批准替他决定范围 |
| 普通 Agent A | XP-001 | 只读 grep/ripgrep 与报告、对 Unix API 和非 Linux fallback 按风险分类 | 修改 native |
| 普通 Agent B | XP-002 设计 | 提前设计 Windows runner/check/upload/evidence 工作流，审查治理义务 | XP-001 未完成就提交 workflow |
| 高级 Agent C | XP-003 设计 | 画出 AppPaths 目录所有调用与旧路径迁移合同 | 未审核就批量替换 `HOME`、XDG 等 |
| 高级 Agent D | XP-005A | 独立 Windows POC，验证 WebView2 最重要的三个假设 | Linux/WSL 假冒 Windows；CDP/Cookie 绕过 |

XP-005A 可以单独原型仓库/临时工作区完成，成果再提交为设计与测试证据（仍需依任务权限审批）。不得使用用户的真实 B 站 Cookie 做自动测试，不要求有真实账号才能验证 WebView2 API 可行性。

## 5. 逐 PR 交接模板（直接发给 Agent）

~~~markdown
# BrowserOS Windows/Linux V2 单任务交接
仓库：https://github.com/code5m/browser-os.git
方案分支：fix/bfc-readback-workbench-hardening-20261114
固定 V1 审查基线：d80f59b0c04457e743a5a18e6e8acae249ea7c6c
任务编号：XP-____
阶段：G____
状态：未开始（Proposed，不代表批准）

开始前必读（按顺序）：
1. AGENTS.md
2. PROJECT-RULES.md
3. docs/engineering/reviews/2026-11-14-windows-linux-plan-audit.md
4. docs/design/browseros-windows-linux-support-v2-20261114.md
5. docs/engineering/reviews/2026-11-14-windows-linux-implementation-backlog-v2.md
6. 任务相关源码、测试；涉及 CI/发布再读 docs/engineering/README.md 与 docs/engineering/governance.json

任务目标：
非目标：
允许修改（准确路径）：
禁止修改：
调用/API/错误码合同：
安全不变量：
前置任务及已满足证据：
本任务独立分支：
负责人：
评审人：
测试命令：
实机验证平台/版本：
回滚方式：
禁止合并/发布的条件：

交付前必须报告：
- 文件清单与 Diff（不得顺便重构）
- commit、基线 sha、工作区变化
- 预期与实际、命令退出码、CI 日志/截图
- Windows 与 Linux 各自的 CODE/PACKAGE/GUI 结论
- ACL/原生命令直接调用负面测试（若相关）
- 故障、已知限制、风险与建议下一项任务

不得：读取或导出 B站 Cookie；开启 CDP；降低 ACL；跳过测试；
在无沙箱时开放 Agent/LSP；将 Linux PASS 作为 Windows PASS；
未经审核直接合并 master；未经授权安装或修改用户本地数据。
~~~

## 6. 测试命令与证据模板

最低命令由 Agent 根据 XP 与仓库工具链确定并记录真实结果；以下是**候选命令，不表示已经执行**：

~~~bash
git status --short
git rev-parse HEAD
npm run build
npm run check:engineering-governance
cargo check --manifest-path src-tauri/Cargo.toml
bash scripts/pre-merge.sh
git diff --check
~~~

Windows 需要 Windows runner 或 Windows 本机的原生 Rust/Tauri 构建命令及 GUI 实测；Ubuntu runner 的 Cargo PASS 不能代替。涉及安装版须按 `AGENTS.md` 禁止开发产物冒充 release 的规范验证；涉及 Windows OS 的命令/选项不得直接照搬 Linux Shell。

~~~text
证据编号：
XP 编号 / 提交 SHA / 分支：
平台（Windows 版本/build 或 Ubuntu 版本/X11）：
架构、Rust、Tauri、WebView Runtime 版本：
安装包及哈希：
类型：STATIC | CODE | PACKAGE | GUI | SECURITY | BFC_REAL
复现命令和前置环境：
预期：
实际：
结果：PASS | FAIL | BLOCKED | NOT_RUN
日志/截图/视频/CI 链接：
失败/未覆盖：
评审人 / 复核日期：
~~~

| 情况 | 允许标记 | 禁止标记 |
| --- | --- | --- |
| 只读源码分析 | REVIEWED_STATIC | CODE_PASS / GUI_PASS |
| Windows Rust 编译成功 | CODE_PASS（必须指定 Windows） | GUI_PASS |
| Windows NSIS 构建成功 | PACKAGE_PASS（必须验证二进制及来源） | GUI_PASS |
| Windows GUI 未做 | GUI_PENDING | GUI_PASS |
| Windows GUI 与真实回读已做 | 对应场景 GUI_PASS、BFC_REAL_PASS | R2 全能力 PASS |
| Linux 已通过但 Windows 未测 | Linux 独立 PASS | Windows PASS |
| 真实环境缺失/依赖阻塞 | BLOCKED / NOT_RUN | PASS |

CI 自动变绿时须确认并非因为路径过滤、忽略错误、删除断言或将 workflow 取消而导致。没有 Windows 证据的 XP-005A/010/016/017 不得虚构截图与日志。

## 7. R1 准入条件（7 条默认未完成）

- [ ] G0 已冻结 Windows 11/Linux 范围、能力降级及原生后端拒绝合同
- [ ] 本地 BFC 服务 Windows 提供方、安装/更新、版本/协议/CSRF/回执合同已确认
- [ ] XP-005A WebView2 真实 Windows 子 WebView、共享 Profile、脚本回执、失败有证据
- [ ] XP-004/011R1 未支持的 Rust 命令 fail-closed；没有不安全文件 fallback
- [ ] R1 开放的书签/会话数据在 Windows 写入、崩溃恢复、升级保留已验证
- [ ] XP 依赖/Agent 分支与审核、工程治理规则无冲突
- [ ] Windows 编译、NSIS、GUI、BFC 端到端及 Linux 回归各自独立证据齐全

该清单是**未来的通过标准**，并不反映实际完成率。只有 XP-010 终审与 Owner 接受上述证据才允许发布 R1。

## 8. 分工风险、预算与发布判定

- 高级模型集中处理 XP-000/004/005A/005B/006/007/008/011R1/011/012/016/017 的合同与安全修改。
- 普通模型接 XP-001、XP-002 CI、XP-009 常规夹具、XP-018A 打包、XP-019 Linux 发行与证据归档；危险区需高级审核授权。
- CI 负责静态扫描、构建、单测、包哈希、进程残留检查，不能自动伪造真实 B 站或桌面 PASS。
- 现有分支有大量 BFC 修复；每任务单独分支与文件所有权，集成按 XP 依赖串行审查，不自动把某代理结果合并到 `master`。
- V1 的工作量估算在 XP-005A 及 Windows BFC 服务交付调查之前不可当作硬工期；根据发现重新核算。

**终审：** 所有能力和 OS 必须逐项提供平台证据；不满足的能力维持 `unavailable` 或 `blocked`。R2 可以明示为 Core，不能对未实现的沙箱、Grid 或文件安全承诺“完整 Windows 支持”。

## 9. 变更记录

| 版本 | 日期 | 说明 |
| --- | --- | --- |
| V1 | 2026-11-14 | 初始 XP-000～XP-020 |
| V1 Review | 2026-11-14 | 指出文件安全/BFC 服务/WebView2 验证/依赖/治理缺口 |
| V2 Proposed | 2026-11-14 | 不覆盖 V1，增加 XP-005A/B、XP-011R1、XP-018A/B、原生拒绝/独立证据/Agent 隔离 |

**文档提交仅代表方案写入，不代表 Windows 开发开始、实现完成或用户批准了产品范围。**
