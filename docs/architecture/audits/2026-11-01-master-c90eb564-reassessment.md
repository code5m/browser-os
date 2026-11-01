# BrowserOS 最新 Master 二次架构审视（2026-11-01 · c90eb564）

> **只读审计范围**：依据 GitHub 当前源代码、Actions 状态及已有架构登记；只新增审计文档，不编辑业务代码、脚本、CI、测试或配置。  
> **当前 Master 精确 SHA**：`c90eb564cb69b28da87d07eb95e9e6f78b86a9c6`  
> **首次审计 SHA**：`9607e260f86b15a730219c619ab5a20cc01efe65`（[旧版只读审计](../audits/2026-11-01-master-reconstruction-readonly-audit.md)）  
> **审计分支**：`research/architecture-reconstruction-audit-20261101`  
> **证据边界**：不是本机运行验收；本次未跑 Rust/GUI/真实负载，下面标注代码观察、CI 实证、文档宣称、待证风险，不能用其中一种代替另一种。文件行号以固定 SHA 为准。

## 一、最重要的结论

**当前不能判定“十项全部达到重构完成标准”。** 与上一版相比，确实有三个实质变化：

1. `agent-foundation/` 已作为 **真实 Rust crate** 合入 Master，并由 Tauri `src-tauri/Cargo.toml` 添加 path dependency；拥有 TaskRegistry、Rig AgentRun/Driver、ToolRegistry、审批票据、可选 SQLite/outbox、协议适配器及独立测试。这不是“只有文档的骨架”。但当前主程序仅新增 **默认关闭** 的 Agent Runtime Lab；**旧 Coding Agent/Browser Agent/Git Agent/Database Agent 并没有全部迁移到新任务执行 Runtime**。
2. Native Command owner 登记由 **178→183**，5 个新增 `agent_runtime_*` 命令均登记 Owner=`agent`，指向 `src-tauri/src/framework/agent_channel.rs`；**183/183 已填写 Owner**。但是仍有 **100/183** 条命令登记在 `src-tauri/src/bridge.rs`，**58/183** 条标记 `permission: "-"`；这些数字分别代表“物理集中”和“登记缺具体权限标签”，**不是** 100 个无归属、58 个确定越权。
3. 最新 GitHub Master `c90eb564` 的 **6/6 工作流状态 success**，包括 [Full Validation 41321886258](https://github.com/code5m/browser-os/actions/runs/41321886258)。但最新 CI 把 Rust 编译与测试/安装版 GUI/资源采样改为按需模式 `native_mode=off`，该 SHA 的 [native-regression job](https://github.com/code5m/browser-os/actions/runs/41321886258) **SKIPPED**。绿色 Full Validation 现在只能证明快速门禁、静态/前端/治理检查通过，**不能声称该次 CI 实测 Rust、deb GUI、RSS/PSS 已通过**。这项差别必须进入发布与重构完成 DoD。

**对上次审计的两个口径修正**：
- 首版将 `orchestrator.ts` / `hotplug.ts` 的缺口列为“P0”；它们的代码路径确有潜在缺陷，但本轮 **没有证明默认生产入口会调用它们**。正确结论是 **潜在重构/扩展阻塞**，不是已发生的线上故障。
- Permission Registry 的 `-` 不能直接推断 Rust 缺少校验。现有 Native Owner 命令包含实际 `check_invocation_source` 等保护；应追踪每一条效果流并写拒绝测试，不能用“字段空”或“已有 Owner”代替实际安全评估。

## 二、主干变化与 Agent Foundation 的真实范围

### 2.1 三个 Master 提交后新增的生产接线

证据：GitHub `9607e260...c90eb564` compare、`src-tauri/Cargo.toml:29-34`、`src-tauri/src/main.rs:1675-1679`、`src/bridge.ts:120-167`。

| 层 | 已经落地 | 尚未达到 |
|---|---|---|
| Crate & SDK | `agent-foundation/`，Rig-core 0.43.0、可选 Rig AgentRun 0.43.0、可选 RMCP 3.8、ACP 3.6，Task/Tool/Event/Approval/Experiment 模块和独立单测 | 真实生产模型/工具网络、统一模型 Provider 生命周期、MCP 真正 Host 执行、完整 ACP IDE 对接 |
| Host 接线 | `framework/agent_channel.rs` 5 个 Tauri 命令已注册且进入 main 窗口 ACL；`SettingsPanel.vue` 异步挂载实验面板 | 既有 `coding_service` 等主执行路径仍在，未统一 Agent Runtime Owner/取消/审批/事件 |
| 只读跨能力 Tool | Catalog `tab_list`、`db_list_connections`、`git_status`、`git_diff`、`coding_lsp_status`；Owner 命令二次校验、结果字节数上限、单次 HMAC 票据 | 未开放 SQL/Git 写、进程、Shell、浏览器导航、MCP effectful 操作，也没有通过真实模型自主工具往返 |
| 状态与持久化 | Lab 配置磁盘持久化，CAS 修订；Foundation 的 SQLite Task/Outbox 代码在可选 `durable-sqlite` 特性中 | `src-tauri/Cargo.toml` 实际启用 `adapter-rig`+`experimental-rig-run`，**未启用 `durable-sqlite`**；Lab 任务不能声称真正持久化跨进程恢复 |
| 测评 | Lab 支持离线 Native/Rig/Canary 固定 fixture，模拟 benchmark 指标 | 不是真实 Agent 生产性能、真实模型 Token、实际 p95/99 GUI 延迟或前后资源比较 |

资料自述 `agent-foundation/HOST-INTEGRATION-VERIFY-20261101.md:13-27` 和真实代码一致：**实验室/只读手动调用**，不是“所有 Agent 已接新引擎”。`agent-foundation/README.md:3` 的“只在实验分支，未接主程序”已是历史语境，跟本次 Master 的实际状态冲突，应修文档。

### 2.2 必须决定清楚的唯一执行 Owner

需要写一张**架构裁决表**（本次不代替实际审批）：

- **Agent Task / Session / Approval 的逻辑真源**：若未来选择 Foundation `TaskRegistry`，旧 `codingService` 和各 Agent 的执行历史/取消状态须统一适配，而不是另建平行永久真源；运行中的任务不能静默重派 Rig/Native。`TaskRegistry` 的 Agent 任务与现有 Scheduled Task `useTaskStore` 语义不同，应加领域限定名避免团队误认。
- **Tool Effect 的唯一实际业务 Owner**：Browser、Database、Git、Terminal、Coding 仍对各自读写、凭据、资源版本、外部副作用负责；Foundation 只管提议、任务、授权证据、执行调度，不能拥有业务数据。
- **Agent Channel Host 的归属**：新增 5 命令在登记中是 `agent`，实现位于 `framework/agent_channel.rs`。如果这里是正式公共接线层，须注明这是 **Agent Host adapter**，而不是“Framework 获得 Git/Database 业务内核调用权”。
- **Runtime 归属**：当前旧 Capability Runtime、旧 Coding Agent 执行循环、新 Agent Foundation 引擎是三个不同概念；可同时存在在代码中，但不允许多个组件同时控制同一条正在执行的用户 Agent Task。

### 2.3 只读 Host API 的具体边界缺口

`agent-foundation/src/public_api.rs:17-62` 定义 Owner catalog 和 `PublicApiHost` trait；`src-tauri/src/framework/agent_channel.rs:226-318` 的 NativeOwnerHost 实际在框架层直接调用：
- `crate::bridge::tab_list`
- `crate::capabilities::database::commands::db_list_connections`
- `crate::capabilities::git::commands::{git_status,git_diff}`
- `crate::framework::ai::coding_lsp_status_for_agent`

这些已有命令可以作为授权适配入口，但**本轮没有发现专门列举此 Rust-to-Rust 跨域调用的“Owner 授权调用方/稳定 Rust Public API”登记**；静态 Native 检查器以 TS `bridge.foo(...)` 调用扫描为主（`scripts/check-native-capability-boundaries.mjs:113-140`），不能自动证明这类内部调用经审批。建议每个业务 Owner 提供小范围 `pub(crate) public_api` facade，提供 `CallerContext`、授权范围、资源身份/版本、审计与取消能力，并给异常负例；Host 只调用 facade，不引用私有业务对象。

另有**具体审批语义问题**：`src/bridge.ts:157-163` 在 `agentRuntimeSetConfig` / `agentRuntimeReadTool` 请求体中直接填 `confirmed: true`；Rust `agent_channel.rs:126-130,326-335` 只读取这个布尔值，真正签名票据由 Host 在读请求内自己签发（`agent_channel.rs:362-409`）。设置页有 checkbox（`AgentRuntimeExperimentPanel.vue:243-244`），但这不是不可伪造的**后端用户意图证明**。本轮只读 Tool 没有开放有副作用写工具；因此此问题首先是**正式授权契约/未来扩展风险**，不要误报为已经可任意 Git/SQL 写入。之后若扩展写/执行，必须服务器签发挑战、受信操作确认、绑定真实 principal/workspace/resource revision 并防重放。

## 三、十条完成标准：Master@c90eb564 的验收矩阵

| ID | 目标 | 现状 | 必须新增的完成证据 |
|---|---|---|---|
| A01 | 所有业务模块有唯一 Owner | **PARTIAL** | Semantic Owner Registry 已覆盖大多数域、Agent/Skill 已分开；新增 Agent Foundation 的 Task/Approval/Runtime 与旧 Coding 运行路径仍未形成唯一生产 owner 迁移表。需要实体/状态级唯一 writer、跨 Agent 适配清单和无双写真源测试。 |
| A02 | 所有新功能经正式 Public API | **PARTIAL** | TS facade + Foundation `PublicApiHost` 已有；NativeOwnerHost 直接调用跨域命令，需完成 Rust 侧业务 Owner 的稳定公有 façade/CallContext/审批登记、接口版本与错误/取消契约。 |
| A03 | 所有 Native Command 有明确业务归属 | **PARTIAL（登记 183/183）** | 183/183 登记了非空 Owner；新增 5 条 Agent 命令归属明确；100 条仍在 bridge.rs；`reveal_path`、`move_path` 指向旧 `src-tauri/src/fs_cmds.rs` 路径。完成条件为 Rust 函数、handler、Tauri ACL、TS bridge、Owner、真实文件位置六者零漂移。 |
| A04 | 所有重要资源有明确生命周期 | **GAP** | `src/capability/runtime.ts:185-211` 的同步 `activate/suspend` 没 await 允许返回 Promise 的 hook；生产 bootstrap `src/capability/index.ts:159,197,222` 用同步激活。Async 版本确实存在，不代表所有路径已迁移。需要状态、资源句柄、异步错误与退出一致性负例。 |
| A05 | 无未经批准的跨模块内部引用 | **PARTIAL / NOT PROVED** | TS 边界脚本只解析相对导入，部分 shared 路径被豁免（`scripts/check-capability-boundaries.mjs:27-150`）；Rust Host 对 Git/DB/Coding 内部的直接调用未有完整静态审批证明。需 TS alias/动态导入/Rust 模块图与例外签审。 |
| A06 | 关键副作用有权限和确认控制 | **PARTIAL / NOT PROVED** | 主窗口来源校验、Git 二段确认、Foundation 单次票据是正向基础；原 `side-effects.yaml` 的 40 条声明中 36 条 `requires_declaration=false`、Native Registry 58 条 permission '-'；新 Lab 的 confirmed 布尔仍来源于客户端。必须形成“执行前身份/授权→资源 Owner 验证→确认→实际副作用→审计”的逐命令证明及远程拒绝/过期/重放负例。 |
| A07 | 模块测试可独立运行 | **PARTIAL** | `agent-foundation/Cargo.toml` 可独立 `cargo test --all-features`，Vault/Clipboard 各有 check；但最新 SHA 默认 Full Validation 不跑 Rust，native rust-tests lane 仅 `cargo test --locked --manifest-path src-tauri/Cargo.toml`，**未单独执行 Foundation --all-features / RMCP/ACP/SQLite 测试**。须建立每模块独立 test & negative fixture 证据。 |
| A08 | 安装版 GUI 真实验收 | **NOT VERIFIED** | `native-regression` 在当前 run skipped，`packaged-gui/resource` 可按需运行 Xvfb；现无该 SHA 的安装版人工 GNOME 签认，`FINAL-HUMAN-ACCEPTANCE.md` 仍 PENDING。 |
| A09 | 与重构前基线比较性能 | **NOT VERIFIED** | 旧历史/Monaco 前端体积数据存在，但 Lab 的 mock µs 不是生产 Agent 性能；没有同机同负载 before/after CPU、RSS/PSS、首屏、WebView/Grid/PTY/Agent p50/p95/p99 成对证据。 |
| A10 | 文档与真实代码一致 | **FAIL（有确定漂移）** | `NATIVE_CAPABILITY_BOUNDARY_AUDIT.md` 仍写 148，而本次 registry 有 183；`agent/public.ts:5` 仍写“无独立 Skill store”；Foundation README/HOST 指向“尚未合入 master”，与事实冲突；Native registry 两条路径失效。 |

上表的 `PARTIAL` 是有框架/代码并非无实现；`GAP` 指存在源码逻辑级反例；`NOT VERIFIED` 指尚无足够运行证据，绝不能用自动推断填 PASS。

## 四、代码级风险和正确的优先级（不是未经验证的生产故障断言）

### P0 — 重构验收之前必须明确定义或修复

**P0-1：Runtime 异步契约和状态推进**  
- `src/capability/types.ts:60-70` 的生命周期 callback 允许 Promise；
- `src/capability/runtime.ts:185-211` 同步入口不等待 Promise 而写 ACTIVE/SUSPENDED；`activateAsync/suspendAsync` 虽存在，默认 bootstrap 尚使用同步入口；
- 要求：定一个 canonical async transition（串行锁、取消、错误回滚、重复调用幂等），明确旧同步 caller 要么只可传同步 Hook，要么迁移异步；用延迟失败的 Hook 负例证实 **未完成不能报告 ACTIVE**。

**P0-2：Native 认证/副作用验收口径**  
- 现有命令的 owner 登记并不等同“权限完整”。对文件写入/删除、Git push/discard、DB 写、Terminal/Script 进程、Agent 变更应用、插件安装以及页面跨源调用，逐命令列出信任域、权限/资源 revision、二次确认/撤销/审计/回滚；
- `src-tauri/permissions/default-commands.toml` 给主窗口较广 ACL，不意味着后端没有 guard；将授权单元下沉到业务 Owner，补负例而非靠 registry 文本；
- Lab 的 `confirmed:true` 只能做演示确认，未来真实写操作不可凭客户端布尔。

**P0-3：重构发布认证不能只看快版 Full Validation**  
- 当前 `native_mode=off` 和 `PREMERGE_SKIP_NATIVE_COMPILE=1` 完全符合最近优化 CI 的要求，不建议恢复每次 Push 都重复 Release 编译；
- **但在宣称“重构完成/可发布”前**，固定同一 SHA 额外获得 `rust-tests` + `packaged-resource` 两条真实运行结果、Foundation 全特性测试、手工安装 GUI 和配对性能基线；用 Release/DoD checklist 将其与常规快速 CI 区分。不要把默认绿色换成发行级全部 PASS。

### P1 — 模块化落地之前必须收敛的明确契约缺口

**P1-1：Hot-Plug Orchestrator 的原子性风险，需先确认它是否为线上路径**  
- `src/capability/platform/hotplug.ts:114-115` 在 `await req.load` 之后执行 `target.addContributions`；`src/capability/platform/orchestrator.ts:108-113` 的 `addContributions` **再次调用 loader 并丢弃可能存在的 Promise**，若走此路径可能双初始化/晚到注册；
- `requestAdd` 权限回调为可选（`hotplug.ts:60-61,101`），`createOrchestrator.add` 没有注入 permit（`orchestrator.ts:128-133`）。如果实际生产引用这个装配入口，必须 fail closed；若目前不使用则作为 **禁止推广 HP2 前的阻塞**，不能写成已发生远程绕过。
- `src/capability/resourceGovernor.ts:61-63` 的 destroy 只有 `runtime.disable`，而 `runtime.ts:250-255` 禁止 ACTIVE 直接 disable；必须统一 destroy/close/deactivate 的可见语义。
- `hotplug.ts:229-231` 在 disable 时 fire-and-forget `onDeactivate`；也应统一等待或限制同步。

**P1-2：Rust Public API 与跨域审批表**  
新增 `framework/agent_channel.rs` 的 Host adapter 采用 `git::commands`、`database::commands` 等公开的命令函数做业务调用，有实际来源校验且限制只读，这是正向结果；但尚不满足“所有 Rust 跨能力调用必须经 Owner 显式批准的正式 API”强约束。建议 Owner facade + 全仓静态图，在 **Rust native-to-native** 调用上记录 allowlist、原因、责任人和禁止过期。

**P1-3：唯一 Agent Runtime Owner 的迁移计划**  
以生产 `coding_service` 为现状；Foundation 的 Task、Event、Approval、Model driver 先作为可选实现，通过 Adapter/Host ports 接入，不要在 Browser/Git/DB 中复制 AI Runtime。每条 Agent 会话只能有一个任务状态机和一个 effect commit owner。冻结任务归属后，再将工具执行移到 Owner 的标准 Public API。

**P1-4：文档自动漂移检测**  
生成 Native Command 统计（183）、签准路径、API 版本和“环境/CI 实测”的证据链接；刷新 `agent-foundation/README.md`、`HOST-INTEGRATION-VERIFY` 的“没进 master”历史语句、`agent/public.ts` 的过时 Skill 注释、Native 148 旧表。避免把本次最新审计混入历史报告：**旧审计保留原 SHA，新审计使用本文件**。

### P2 — 独立测试、GUI 和前后性能证明

1. **每能力独立 test**：至少静态契约、状态与生命周期、权限否决、资源释放、拒绝循环依赖/越界。对 Foundation 要求 `cargo fmt/check/test/clippy --manifest-path agent-foundation/Cargo.toml --all-features` 的同 SHA 证据；Root native test 不能代替独立可选 SDK/SQLite feature。
2. **安装版 GUI**：至少 Browser/WebView/Grid、文件、Monaco、终端、数据库、Git、Coding Agent，新增 Runtime Lab 默认 OFF/开启/只读/失败/取消/关闭；逐项记录包 SHA、Ubuntu/GNOME、人工操作、截图、进程泄漏/启动日志。
3. **性能配对**：指定 `before_sha`、`after_sha`、同机器/内核/依赖/场景/数据集，测 5min/30min、WebView/Grid PID、PSS/RSS、CPU、启动、交互 p50/p95/p99；空白指标标 `NOT_MEASURED`。当前 mock Rig/Native 每轮微秒数只可作为编排单元测试，不能当重构性能改善结论。

## 五、最新版 CI 的真实覆盖证据

`master@c90eb564` 六个工作流全部 success：
- [Engineering Governance](https://github.com/code5m/browser-os/actions/runs/41321886172)
- [Developer Feedback](https://github.com/code5m/browser-os/actions/runs/41321886347)
- [UI Safety](https://github.com/code5m/browser-os/actions/runs/41321886190)
- [Hot-Plug Acceptance](https://github.com/code5m/browser-os/actions/runs/41321886180)
- [Supply Chain Assurance](https://github.com/code5m/browser-os/actions/runs/41321886123)
- [Full Validation](https://github.com/code5m/browser-os/actions/runs/41321886258)

Full Validation job 快照：`impact=success`、`frontend-build=success`、`validate=success`、`native-regression=skipped`、`source-evidence=skipped`。validate 日志确认 `PRE_MERGE_COVERAGE=FAST_NATIVE_COMPILE_SKIPPED`；`cargo check/test` skipped；`PRE_MERGE_RESULT=ALL_PASS` 仅在此“快速预合并”范围内成立；doctor 仍报告 `GUI_PENDING`。这与 `.github/workflows/capability-v2-validation.yml:19-30,308-316,325-450` 和 [CI-OPTIONAL-NATIVE-20261101.md](../../engineering/CI-OPTIONAL-NATIVE-20261101.md) 一致。

配置了四档 `off/rust-tests/packaged-gui/packaged-resource`。当 `rust-tests` 按需执行时，workflow 执行一次 `cargo test --locked --manifest-path src-tauri/Cargo.toml`；当 `packaged-resource` 执行时重用相同 SHA frontend，单次 Release build + Xvfb GUI + 5min 采样。需要说明：`agent-foundation` 的 `durable-sqlite/adapter-rmcp/adapter-acp` 并不属于 root 默认依赖特性，不能让别的构建结果替代 Foundation `--all-features` 证明。

## 六、最终重构 DoD（任何缺项不标 ALL PASS）

- [ ] `OWNER_UNIQUE=PASS`：所有业务实体/可写状态/Agent Task 唯一 canonical Owner，有跨域例外审批与静态负例。
- [ ] `PUBLIC_API=PASS`：所有跨域 Rust/TS 调用经过正式稳定 facade，签准 Consumer/Scope/Permission/契约版本。
- [ ] `NATIVE_INVENTORY=PASS`：183/183 owner + 实际 Rust 函数/handler/ACL/TS bridge/file locator 严格零漂移，新增命令自动更新总数。
- [ ] `LIFECYCLE=PASS`：资源完整 birth/active/suspend/cancel/close/crash/restart，异步失败及回滚准确、不遗留 PID/连接/worker。
- [ ] `INTERNAL_BOUNDARY=PASS`：全仓 TS alias/dynamic/re-export 和 Rust 模块依赖检查，无未经批准越界（例外带人/理由/到期日）。
- [ ] `EFFECT_GUARD=PASS`：每条关键副作用均通过 permission+二段确认/拒绝/过期/重放/审计与 resource-version/身份验证。
- [ ] `MODULE_TEST=PASS`：每模块独立运行；Foundation 所有生产/可选 feature 有专项测试，不用统一全仓命令冒充独立验收。
- [ ] `GUI_INSTALLED=PASS`：当前 SHA 真实 deb、自动 Xvfb 回归及 Ubuntu GNOME 人工 GUI 都通过，证据与包 SHA 一致。
- [ ] `PERF_BEFORE_AFTER=PASS`：真实硬件成对指标与重构前基线对比，带原始 artifact/统计。
- [ ] `DOC_CODE_PARITY=PASS`：Native Registry、Owner、API、测试口径、CI/GUI 状态与真实代码一致，文档机审零漂移。

**合并策略**：保持默认快速 CI 以免阻塞日常开发；此 DoD 用于“重构完成/发行级认证”，不是要求每次普通 push 都重复原生编译或安装 GUI。审计分支不修改任何业务源码；待业务改造需独立实施分支、独立验收和批准。
