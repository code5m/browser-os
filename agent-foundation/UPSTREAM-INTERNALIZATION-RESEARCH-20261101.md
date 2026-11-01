> **后续实现提醒（2026-11-01）**：本文是最初调研/升级计划的历史快照，其中“未接 SDK”等描述已经过时。当前实际依赖、版本、功能边界与验收以 [开源接入矩阵](OPEN-SOURCE-INTEGRATION-MATRIX-20261101.md) 和 [验证报告](VERIFICATION-FOUNDATION-20261101.md) 为准；固定 Rig 0.43.0、RMCP 3.8.0、ACP 3.6.0，不升级根项目 Rust 工具链。

# Agent Channel 上游项目深度内化研究决策（首轮源码审查）

> 项目：code5m/browser-os。工作分支：feat/agent-channel-rust-foundation-20261101。
> 时间：2026-11-01。只更新此分支 agent-foundation/ 内研究材料；不接入 BrowserOS。
> **研究口径**：已直接检查上游仓库 README、Cargo、部分核心源码；没有完成全调用链审计/真实编译/互操作或压力测试，因此属于 **源码级首轮审查**，不是“深度内化已完成”。
> **当前代码状态**：agent-foundation 是独立的 Rust 原型；Rig/ACP/RMCP/A2A/CloudEvents/Goose 目前全部 **未作为依赖接入**。

## 0. 总决策

采用 **独立核心契约 + 成熟 SDK 可选适配器 + BrowserOS 自有权限/任务/资源治理**。内化不是直接复制项目代码，也不是把六个 SDK 一起塞入 Tauri。必须先比较“上游已实现的东西”与本地原型，删除重复设计并保留唯一 Owner。

- **直接依赖 SDK（未来可选）**：供应商通信 Rig；MCP wire/transport RMCP；对外编辑器兼容 ACP。依赖需要稳定版本、特性裁剪、Cargo 验证。
- **内化设计**：任务与资源生命周期、用户授权、资源快照验证、任务执行租约、事件审计、消息边界。只针对 BrowserOS 独有业务实现，不 fork 原生协议。
- **只借鉴，不接入整体项目**：Goose 源码中的 Agent 运行架构/工具审批/取消/扩展管理。
- **暂不引入**：A2A 外部 Agent 协作、CloudEvents SDK 的完整事件总线。两者有明确外部互通需求后再开特性。

## 1. 六个上游项目的首轮源码事实和处理结论

| 项目 | 已核对的具体实现 | 内化策略 | 不做什么 | 当前 |
|---|---|---|---|---|
| Rig | rig-core 统一消息/模型/工具；rig-agent 的 AgentRun 是可序列化、sans-I/O 的状态机，next_step 会输出 CallModel / CallTools / Done；rig-memory 会话存储；rig-cassette 效果记录/回放 | **优先深研，并做两个受控 PoC：模型适配；AgentRun 与本项目任务状态的责任分离。**如 Rig 状态机满足需要，停止重造模型多轮执行状态机 | 不把 Rig CLI/Bevy Harness、文件/shell 工具直接植入 BrowserOS；不重写 Rig 模型协议 | 首轮源码阅读，未接依赖 |
| ACP | agent-client-protocol Rust SDK 的 Client/Agent/Proxy/Conductor、session.new/load/resume；v1 稳定、v2 明确标记 draft/unstable；RMCP bridge 可选 | **作为 Editor ↔ Agent Channel 的可选外部通信 adapter**，先对照 v1 稳定契约做测试。BrowserOS 自有 Task 状态不等于 ACP 会话 | 不用 ACP 替代产品内部事件系统或 BrowserOS 权限系统；不默认开启 draft v2 | 首轮源码阅读，未接依赖 |
| A2A | a2a-rs 提供 A2A v1 Task、Agent Card、client/server、多传输协议 | **暂不接入**；未来确需跨设备/跨进程/第三方 Agent 互操作时，单独提供 gateway，内部任务映射需显式转换 | 不为了内部 Browser/Code/Git Agent 协同而增加 A2A 网络复杂度 | 首轮源码阅读，延期 |
| RMCP | 官方 Rust SDK，支持 stdio/HTTP、协议版本协商、Tools/Resources/Prompts、2026-07-28 扩展及 MCP TaskManager | **优先做可选 MCP 客户端/服务端传输 adapter**；保留 BrowserOS 已有 capability allowlist 为唯一权限真源 | 不复制实现 MCP wire/jsonrpc，不让 RMCP tool discovery 越过宿主审核；MCP Task 不是 BrowserOS Agent Task | 首轮源码阅读，未接依赖 |
| CloudEvents Rust | Event/EventBuilder、1.0 JSON、HTTP/Kafka/NATS 绑定，README 标记 WIP/API 不稳定 | **先只将 metadata-only AgentEvent 输出映射为 CloudEvents 1.0 schema**；需跨进程分发时才评估可选 SDK | 不把整个事件存储、前端 EventBus、审计日志都换成 CloudEvents SDK | 首轮源码阅读，SDK 延后 |
| Goose | goose-agent StateMachine + EffectHandler + CancellationToken；Goose ToolConfirmationCoordinator 使用每 Session 的 turn_lock、请求去重和等确认；PermissionStore 含 context hash、expiry | **只借鉴业务拆分和安全/生命周期测试**；对照 BrowserOS Coding Agent 逐项提取 invariants（授权/会话恢复/未知结果/多工具） | 不依赖 Goose 应用大仓，不复制其桌面 UI、provider / extension 全体系；不直接复制其授权存储实现 | 首轮源码阅读，无依赖 |

### Source map（首轮直接查阅）

- Rig 模型执行状态机：https://github.com/0xPlaygrounds/rig/blob/main/crates/rig-agent/src/run/mod.rs
- Rig 工具调用策略：https://github.com/0xPlaygrounds/rig/blob/main/crates/rig-agent/src/run/policy.rs
- Rig replay identity：https://github.com/0xPlaygrounds/rig/blob/main/crates/rig-cassette/src/agent/mod.rs
- ACP session lifecycle：https://github.com/agentclientprotocol/rust-sdk/blob/main/src/agent-client-protocol/src/session.rs
- ACP overview：https://github.com/agentclientprotocol/rust-sdk/blob/main/README.md
- A2A Task types：https://github.com/a2aproject/a2a-rs/blob/main/a2a/src/types.rs
- RMCP TaskManager：https://github.com/modelcontextprotocol/rust-sdk/blob/main/crates/rmcp/src/task_manager.rs
- RMCP transport/protocol overview：https://github.com/modelcontextprotocol/rust-sdk/blob/main/README.md
- CloudEvents event data：https://github.com/cloudevents/sdk-rust/blob/main/src/event/mod.rs
- Goose state machine：https://github.com/aaif-goose/goose/blob/main/crates/goose-agent/src/machine.rs
- Goose tool approval：https://github.com/aaif-goose/goose/blob/main/crates/goose/src/agents/tool_confirmation_coordinator.rs
- Goose permission store：https://github.com/aaif-goose/goose/blob/main/crates/goose/src/permission/permission_store.rs

## 2. 重大架构纠偏：不能重复制造 Agent 引擎

Rig 的 AgentRun 目前已能串联 CallModel/CallTools/Done，并有 recording/replay 扩展。
因此暂停扩展当前 `engine.rs` 为大型多轮框架：它只作为独立测试样板，用于对照上游行为，**不是未来必然保留的生产 runtime**。

未来如采用 Rig：
1. BrowserOS Task = 外层“用户目标/授权/工作区/任务状态/事件/持久化/取消”的唯一 Owner。
2. Rig AgentRun = 内层“模型回合、工具响应配对、对话构造”的算法步骤。它不拥有文件、终端、浏览器等真实副作用。
3. 在 Rig 的 CallTools 步骤将工具建议送给 BrowserOS 权限 Broker，用户批准后由被调用能力 Owner 执行；模型不得自行确认。
4. 需要一个**单向状态映射表和单一持久化点**，防止 Task 与 AgentRun 保存互相矛盾的状态。
5. 对比实验失败时，可以保留自有最小 StateMachine，但必须记录上游不满足的具体测试与证据。

RMCP 的 TaskManager 解决的是 MCP Server 长任务生命周期，不能直接替代 BrowserOS Agent 任务。
ACP Session 解决的是编辑器与 Agent 交互会话，不能直接替代本地任务执行状态。
A2A Task 解决外部 Agent 协作状态，也不能不经转换复用同一个 Rust Task enum。

## 3. BrowserOS 原有能力复用（不修改）

- 旧 `src/capabilities/agent/` 是 Agent 定义/只读壳，`agent_chat` 等执行接口尚未落地；不应为它再造一套模型执行器。
- `src-tauri/src/capabilities/agent/coding.rs` 已包含用户审核、草稿、源文件 hash 检查和操作确认流程；后续应做 adapter，而非整体替换。
- `src-tauri/src/mcp.rs` 已有 allowlist/policy；`mcp_server.rs` 是 gated stdio 预备骨架，两个只读 introspection 绑定，其余 fail-closed。RMCP 只负责协议传输，不产生第二份权限。
- `src-tauri/src/capabilities/agent/coding_lsp.rs` 已有缓存/复用 LSP 会话及隔离策略；不要重复做 LSP 管理层。
- `src/capabilities/agent/ui/CodingWorkbench.vue` 同时持有多类状态，未来拆 UI 状态时要保持唯一 Owner 和草稿的一致性。
- Browser / Grid / WebView 的原生 GUI 具有经实测锁定的生命周期限制，本独立实验一律不触碰。

## 4. 兼容性、许可、供应链首轮风险

| 项目 | 上游检索到的版本/要求 | 风险和下一步 |
|---|---|---|
| Rig | README 示例 Rig 0.46.0；CLI 提示 Rust 1.97.1+ | 需要查明用于 SDK 的确切 MSRV、license 和依赖树，再决定具体 crate，而不是直接引入整个 CLI；Rust 编译器版本需在授权机器核对 |
| ACP | Rust SDK workspace Rust 1.88、SDK 3.6.0；v2 draft | 只启用稳定 v1；覆盖重连/恢复/取消和协议兼容 |
| RMCP | workspace SDK 3.8.0、Rust 1.88；协议兼容性快速变化 | 先核对 lockfile、transitive deps、features，注意官方 LICENSE 描述 MIT→Apache-2.0 重许可过渡 |
| A2A | Rust SDK README Rust 1.85+ | 多传输/网络面较大，内部协作暂不需要，延后 |
| CloudEvents | SDK 0.9.0，README 明确 API 尚不稳定 | 仅输出结构化事件/标准 JSON，先不依赖全部 SDK |
| Goose | workspace 1.57.0、Rust 1.94.1；依赖 RMCP 3.4.1、ACP 2.2.0 | 与上游最新 ACP/RMCP 版本不同步，任何设计均必须重新做兼容测试；不整体集成 |

**依赖许可声明**不等于已完成法律审查；任何第三方代码复用和分发都需确认适用 LICENSE、NOTICE、依赖许可和项目根目录 MulanPSL-2.0 兼容性。不得未经审查直接复制外部源码。

## 5. 真正“深度研究”的待办与验收（不能用 README 阅读代替）

### D0 — 先让独立基础项目可编译（阻塞后续依赖接入）
- [ ] 真实 Rust/Cargo toolchain 验证。执行 fmt/check/test/clippy/example。
- [ ] 修复编译/借用/状态机安全问题，新增关键单测和失败注入。
- [ ] 对基础代码标识“协议骨架”；禁止在验证前写 SDK 兼容/完成。
- [ ] 检查除 agent-foundation/ 外 0 文件变动。

### D1 — 对 Rig 做深入双方案实验（最高优先）
- [ ] 绘制 Rig AgentRun next_step→model_response→CallTools→answer→Done 的实际调用链和故障路径。
- [ ] 追踪 rig-memory/cassette 事务边界；分析未知工具返回、流中断、重启、重复工具调用。
- [ ] 独立 feature-gated PoC A：rig-core 模型统一适配；PoC B：rig-agent 内层状态机 + BrowserOS 外层 Task/Approval，双方案对照测试。
- [ ] 记录自研 vs 上游依赖的新增代码、编译时间、包体积、性能、升级风险；决定废弃/保留当前 engine.rs。

### D2 — 研究 Goose 的宿主能力与安全时序
- [ ] 读完整 ToolConfirmationCoordinator、StateMachine、EffectHandler、extension lease、PermissionStore。
- [ ] 画出请求→授权→工具执行→回传→取消/未知结果→恢复的调用链及资源 Owner。
- [ ] 与现有 Coding Agent 对照；列出可直接复用和必须重写的场景。
- [ ] 验证 PermissionStore 参数 hash + expiry 的安全边界，BrowserOS 必须额外绑定 principal、workspace、resource revision 和 effect。

### D3 — RMCP / ACP 分别做协议互操作（不触碰主应用）
- [ ] RMCP 使用官方 SDK 做受限 mock server/client，覆盖 stdio 初始化、版本协商、tools/list/call、cancel、timeout、恶意参数、错误码；保持工具实际执行全为 mock。
- [ ] ACP 使用稳定 v1 构建 fake editor/client + fake agent/server，覆盖 session lifecycle、流式进度、审批/取消、断线恢复，禁止读取真实项目文件。
- [ ] 依赖均可选（feature flags），不改变根目录 Cargo/Tauri 依赖；有测量结果后才考虑接入提案。

### D4 — CloudEvents / A2A 按需内化
- [ ] CloudEvents 先只做内部 EventRecord→CloudEvents 1.0 的规范测试（specversion/id/source/type/subject/data），不把 SDK 引入内部总线。
- [ ] A2A 暂不开发；有跨进程 Agent 对接需求再设计 Agent Card、Task state 和 auth mapping。

### D5 — 编制最终内化实施准入报告
必须回答：独立 crate 编译结果、上游源码路径、对照测试、不能复用的原因、依赖版本/MSRV/许可、代码复用收益、性能差异、唯一 Owner、迁移文件、回滚方案和 GUI 验收计划。没有证据不能写“内化完成”。

## 6. 明确的架构出口（未来，单独授权后）

```text
BrowserOS 外壳 / 工作区 / 编辑器 / 浏览器  [原有代码保持不变]
                        |
             Agent Channel Public API       [未来独立接线]
                        |
               BrowserOS Agent Host
       TaskStore + PermissionBroker + ContextGate
       EventJournal + cancellation + ResourceLease
            /                |               \
   Rig adapter          RMCP adapter        ACP adapter
 (模型/回合)          (MCP 协议传输)     (编辑器互操作)
                 A2A optional later
```

“核心 Rust 能力被 **内化**”的标准是：外部 SDK 可以切换而不修改 BrowserOS 宿主任务/权限/资源语义；绝不是脱离上游自己再造 MCP/ACP 协议标准。

## 7. 进度与当前保护

- 完成：6 个上游项目的首轮核心源码/manifest/README 对照；确认 Rig/AgentRun 重复实现风险；形成明确内化策略。
- 进行中：独立 Rust 原型和 mock 测试代码已在 agent-foundation/，尚待真实 Cargo 验证。
- 尚未完成：上游全调用链审计、实际接入任何 SDK、互操作、安全/性能测试，以及 BrowserOS 生产接线。
- 不变更：master、perf、其他现有分支和所有现有源码。此文档只是新分支的设计资产。
