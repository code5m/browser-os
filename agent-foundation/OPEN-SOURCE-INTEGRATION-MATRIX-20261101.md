# Agent Channel Foundation — 开源能力接入与取舍（2026-11-01）

分支：`feat/agent-channel-rust-foundation-20261101`。唯一修改范围：`agent-foundation/`。本文件是**本次实现后**的状态说明；早期 `UPSTREAM-INTERNALIZATION-RESEARCH-20261101.md` 是接入前历史调研，不再代表当前依赖状态。

## 一、什么叫“已集成”

- **A：真正依赖上游 Rust crate + 编译 + 模拟交互测试**：可以确认本分支的 SDK 能编译并完成声明的本地测试；不代表与所有真实服务互通。
- **B：采用开源生态基础库**：利用成熟持久化、并发或密码学实现；这些不等于外部 Agent 协议接线。
- **C：只借鉴设计/兼容格式**：没有把该项目作为 crate 引入，更没有宣称运行了它。
- **D：明确不集成/延期**：当前 BrowserOS 实际需求或安全准入条件不支持引入。

### A. 实际接入的三个 Agent 相关官方 SDK

| 开源项目与版本（固定） | 已经用到的具体能力，代码证据 | 为什么使用 | 哪些能力**没有**集成及原因 |
|---|---|---|---|
| **Rig / rig-core 0.43.0**，原项目 [0xPlaygrounds/rig](https://github.com/0xPlaygrounds/rig) | `src/adapters/rig.rs` 实际使用 `DynModel<Completion>`、`CompletionRequest`、`Message`、`CompletionResponse`、`Streamed` 的真实增量流、工具提议和 token usage；将 SDK 的返回映射到内部 `ModelEvent`；模型提出工具时交给宿主审批，不让 SDK 执行 | 多模型接线只维持一个供应商无关入口；保留官方模型协议/流式解码，不在 BrowserOS 自造各家请求和工具回包协议 | **未引入 rig-agent 的 AgentRun**（会与 BrowserOS Task/授权状态机产生双 Owner，需先做状态映射及双方案对照）；**未引入 rig-memory/rig-cassette、向量存储、CLI/Bevy、Rig 自带 shell/file 工具**（安全/存储/资源治理未过审）；**未测试真实远端模型/密钥**，只能称 SDK 模型传输可注入、流式包装已实现 |
| **RMCP / rmcp 3.8.0**，官方 [MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk) | `src/adapters/rmcp.rs` 用 `CallToolRequestParams`、`CallToolResult`、`Tool`、`RunningService<RoleClient,()>`；在官方 SDK **client/server + tokio::io::duplex** 上真实完成初始化、`tools/list`、`tools/call` 和返回；宿主 allowlist 决定只读工具，输出限 64 KiB，错误明确拒绝 | 不重新实现 MCP wire、协商和传输；Browser/DB/Git/Code 能复用 MCP 工具生态，而不产生第二个安全 Owner | **没有接入远端/stdio 实际 MCP server、OAuth/凭证、Resources/Prompts、MCP Tasks、写入或进程工具**；SDK 的 `readOnlyHint` 是不可信提示，不能成为授权依据。写入/远端副作用必须先通过宿主幂等、审批、资源归属及未知结果对账 |
| **ACP / agent-client-protocol 3.6.0**，官方 [ACP Rust SDK](https://github.com/agentclientprotocol/rust-sdk) | `src/adapters/acp.rs` 用稳定 **v1** 的 `SessionId`、`SessionNotification`、`SessionUpdate`、`ContentChunk`、`TextContent` 和官方 `RawJsonRpcMessage`／`TransportFrame`；在 SDK `Channel::duplex` 上传递会话消息给模拟编辑器并反序列化验证。模型工具建议不会被转为自动审批 | 与 IDE/编辑器 Agent 生态形成标准会话/流式 UI 交互能力，避免把内部 Task 状态等同 ACP Session | **未实现完整 Agent/Client 的 initialize/new/load/resume、断线重连、权限协商和真实编辑器接线**；**不启用 unstable v2**。批准工具仍属于 BrowserOS 宿主，不接受 ACP 客户端或模型自行授权；SDK Channel 是无界内存通道，生产需宿主背压 |

Rig 版本选择：开发机 Rust `1.96.0`；Rig `0.46.0` 的声明 MSRV 是 `1.97.1`，故固定兼容的 `0.43.0`，**没有为此升级 BrowserOS 整体工具链**。三个 SDK 通过 `adapter-rig`、`adapter-rmcp`、`adapter-acp` 可选 Cargo feature 控制；默认不加载。针对上游版本升级必须重新验证 API、MSRV、依赖许可证和协议互操作。

### B. 实际采用的基础设施开源库

| 项目 | 实用能力，代码证据 | 为什么用 | 仍不解决什么 |
|---|---|---|---|
| **SQLite / rusqlite 0.37 + bundled SQLite** | `src/durable.rs`：独立数据库，WAL/FULL sync、任务 revision CAS、**任务/事件/Outbox 同事务**、按任务顺序重放、明确 ACK、跨进程执行租约、进程重启后 Paused、`UNKNOWN` 副作用日志与人工对账 | 资源轻、嵌入式、无需再部署服务；用成熟事务保证一致性，不在代码里自造存储引擎 | 尚无 SQLCipher/系统密钥绑定、备份与数据保留政策、迁移策略、跨进程全局 leader 与分布式 fencing；同步 SQLite API 需由宿主调度至非 GUI 线程；**禁止保存真实敏感数据用于生产** |
| **Tokio** | 原有异步引擎、`watch` 取消、有限 `mpsc`、超时、真实 RMCP 测试内存传输、Rig 实时事件转发 | 不自己做 async runtime、线程池、信号和取消 | 无完整 BrowserOS 业务优先级调度/资源配额，不等于“全系统无阻塞” |
| **Serde / UUID** | 版本化 envelope、协议投影、快照和事务记录、事件 ID/Task ID | 标准化序列化和稳定关联键 | 不等于身份认证或访问授权 |
| **hmac / sha2 / hex** | `src/approval.rs`：HMAC-SHA256 凭证签发和常数时间核验，绑定 principal/workspace/session/task+revision/call/tool/args SHA256/effect/resource+revision/TTL/decision | 外部模型只能提议；用户批准结果不能简单伪造或跨任务重放 | **签发端的用户身份认证和密钥存储仍由可信 OS/Host/UI 提供**；资源 Owner 在真正执行前仍须核验权限和资源版本；不能宣称实现了系统级身份认证 |

### C/D. 特别说明没有集成的项目/能力

| 项目/能力 | 本分支状态 | 选择理由 | 什么条件下考虑 |
|---|---|---|---|
| **A2A / a2a-rs** | **未依赖、未接 A2A Task/AgentCard 网络** | Browser/Database/Git/Coding 四类 Agent 是一个产品的内部分工，不需要将内部任务转成对外可信 Agent 网络或建立新公网攻击面 | 有第三方/跨设备 Agent 合作需求及服务身份、网络边界、任务映射后单独做 gateway |
| **CloudEvents Rust SDK** | **未依赖 SDK**；现有 `event.rs` 仅手工生成 metadata-only CloudEvents 风格 JSON | 内部 Outbox/审计先使用自己的 Task/Event 事务与顺序；没有跨组织事件 Broker/Kafka/NATS 的刚需，整包 SDK 会增加升级、传输与存储复杂度 | 明确接外部事件消费者，做 spec 1.0 一致性、Schema/订阅、数据脱敏与 ACK 验证后再引入 |
| **Goose** | **仅源码/架构参考**；未链接 Goose crates，未复制代码 | Goose 是完整 Agent 应用/扩展体系；整体引入将重复已有 BrowserOS 宿主、UI、安全和资源 Owner | 可继续对照其 `ToolConfirmationCoordinator`、状态机和恢复测试，不引入 UI/命令执行 |
| **Rig AgentRun / rig-agent** | **明确尚未整合** | Rig 的多步执行状态机与 BrowserOS 任务状态机可能重复；先验证“唯一 Task Owner”映射与故障补偿，避免双状态机、双持久化 | 先完成 Rig AgentRun ⇄ 本地审批/任务状态的冻结契约和回放/取消对照测试，再选择替换或保留现有最小 engine |
| **MCP 写/执行和真实浏览器/数据库/Git/终端 Owner** | **未接入**；本分支所有执行均模拟 | 当前只是独立 foundation。实际能力需要复用现有 Public APIs，不应绕过业务 Owner，也不能隐式授予 shell 或文件写入 | 宿主按 principal/resource/version/effect 再授权、幂等对账、真实回滚、审批 UI 与 GUI 验收通过后单独集成 |
| **ACP v2、MCP 跨设备服务、第三方 Provider auth** | **未启用** | v2 尚为实验协议，真实服务凭证/远端网络会产生额外权限与供应链风险 | 稳定协议版本和独立验收后启用 |

## 二、已实现的安全链路（由谁负责）

```text
模型/Rig 请求 -> ModelEvent::ToolSuggestion (不能执行)
  -> 注册的 ToolRegistry 决定 effect (忽略模型和 MCP hint)
  -> TaskRegistry AwaitingApproval (SQLite 同事务写出事件)
  -> 可信 Host UI 验证用户与资源 -> HostApprovalIssuer HMAC 签发
  -> ApprovalBroker 验签、工作区/会话/任务版本/参数/资源/过期校验
  -> Tool Owner validate_binding() 再验证实时资源
  -> [写/执行/远端] 仅 durable-sqlite + supports_idempotency=true
  -> 先写持久化 UNKNOWN (唯一 call_id + 幂等 key)
  -> call_authorized 传递幂等 key 至真实服务
  -> 收到明确成功 -> Confirmed
  -> 取消/断电/超时/响应未知 -> 保留 UNKNOWN，不自动重试
  -> 宿主使用外部事实证据 -> ReconciledSucceeded / ReconciledNotApplied
```

**重要未闭环**：这里提供的是“可信宿主可调用的签名/检查接口”，不是对真实人或真实 OS 会话的鉴权方案。不能把 SDK 的 tool call/ACP 通知当作用户批准。外部提供商能否实际去重必须由每个业务 Owner 承诺，不能由一个字符串 `idempotency_key` 自动推导。

## 三、实际测试覆盖与测试之外的范围

- 基础状态机与拒绝默认工具、Rig 返回映射、ACP v1 类型与 SDK Channel、RMCP SDK 初始化与 tools/list/call、SQLite/Outbox、HMAC 篡改/过期/重放：均有独立测试。
- 故障注入：签名篡改/错误 key/重放、资源版本变化拒绝、外部工具执行后永不响应导致超时、取消后仍可能发生副作用、未知结果只允许显式对账、SQLite 文件重启、事务 Outbox 写入触发器失败整体回滚、活动租约拒绝并发恢复。
- **没有**跑真实模型凭证、真实远程 MCP 服务、ACP 真实编辑器、SQLCipher、网络丢包与供应商故障、性能基线、BrowserOS GUI 安装版冷启动、Root Full Validation；不得继承独立 crate 的 PASS。
- Cargo 的默认 feature 关闭外部 SDK 与数据库。**仅编译全特性并通过模拟测试**不等于整个产品生产部署成熟。

## 四、编译、验证和安全准入命令

```sh
cargo fmt --manifest-path agent-foundation/Cargo.toml -- --check
cargo check --manifest-path agent-foundation/Cargo.toml
cargo check --manifest-path agent-foundation/Cargo.toml --all-features
cargo test --manifest-path agent-foundation/Cargo.toml --all-features
cargo clippy --manifest-path agent-foundation/Cargo.toml --all-features --all-targets -- -D warnings
cargo run --manifest-path agent-foundation/Cargo.toml --example dry_run
```

**上线前仍必须**：BrowserOS 主应用 Owner / Public API 接线与签名认证、独立密钥管理、持久化加密和迁移、模型供应商/API真实联调、MCP 协议错误/取消/背压安全审查、ACP 初始化/恢复、故障恢复人工验收、性能对照和安装包 GUI 完整验收。原有 master/perf/其他分支此处不改。

资料：`README.md`、`IMPLEMENTATION-PLAN.md`、`VERIFICATION-FOUNDATION-20261101.md`、`UPSTREAM-INTERNALIZATION-RESEARCH-20261101.md`。本项目根许可证为 MulanPSL-2.0，SDK 依赖许可证/NOTICE 需在分发前由供应链策略复核。
