# BrowserOS Agent Channel — 独立 Rust Foundation

> **仅在 `feat/agent-channel-rust-foundation-20261101` 实验分支，尚未接入 BrowserOS 主程序，也未达到生产准入。**
> 当前已在授权开发机真实编译/运行 SDK 模拟测试。所有修改只在 `agent-foundation/`，不修改 `master`、Tauri、WebView、编辑器、Git、数据库业务或其他 Agent 分支。

## 先看真实的开源集成情况

详细阅读：**[开源能力接入与取舍矩阵](OPEN-SOURCE-INTEGRATION-MATRIX-20261101.md)**，逐项目列出“引入了什么 / 为何引入 / 未引入什么 / 为什么”。概要：

- **Rig-core 0.43**：官方模型请求、动态模型、增量模型流、统一工具提议与用量映射。Rig 不掌握 BrowserOS 的 Tool 执行与批准。未接 Rig AgentRun / memory / cassette。
- **官方 RMCP 3.8**：通过 SDK Client/Server 内存对接验证初始化、tools/list/call；宿主白名单决定只读工具，工具元数据没有授权效力。尚未连接真实 MCP 服务及写/执行工具。
- **官方 ACP 3.6 稳定 v1**：Agent 输出投影为官方会话更新与 JSON-RPC frame，SDK Channel 模拟编辑器交互通过。尚未接真实 IDE 生命周期、鉴权与恢复。
- **rusqlite 0.37 / SQLite**：可选事务性 task/event/outbox、执行租约、UNKNOWN 副作用状态以及人工对账。未做 SQLCipher 与 OS 安全存储接线。
- **hmac / sha2 / hex**：宿主签发短期签名审批、精确工具/参数/资源绑定；真正身份验证、资源 Owner 权限及密钥托管仍属于 BrowserOS。
- **A2A、CloudEvents SDK、Goose**：没有作为依赖集成；原因详见矩阵。CloudEvents 风格元数据输出不等于集成 SDK。

## 代码边界

```text
agent-foundation/
  src/approval.rs     HMAC 授权票据/资源+任务签名/一次性消费
  src/durable.rs      可选 SQLite 任务+事件+Outbox CAS/恢复/租约/副作用对账
  src/model.rs        模型中立的输入/事件契约
  src/adapters/rig.rs 官方 Rig 动态模型实时流适配；工具只提议
  src/adapters/rmcp.rs 官方 RMCP SDK 只读工具策略 + mock SDK 客户端/服务端
  src/adapters/acp.rs ACP 稳定 v1 会话消息/SDK Channel 兼容
  src/task.rs         Task 状态、取消、暂停/恢复、版本检查
  src/tool.rs         效果分类、Owner 二次授权、幂等 key、UNKNOWN
  src/engine.rs       有界单模型回合，不自动调用模型建议的工具
  src/event.rs        事件及 metadata-only 回放
  tests/contract.rs   跨模块 API/审批契约
  tests/fault_injection.rs 崩溃、超时、取消、资源变更、重启
```

**唯一安全边界**：模型只能提议工具；宿主用户应从可信 UI 确认工具、参数、资源及影响，宿主密钥才能签发票据；Tool Owner 再次核验实际资源与版本。写/外部操作先记录持久化 UNKNOWN，出现超时、取消或断电不能自动重试，必须查外部证据后人工对账。

## 开发机独立验收

在本仓库根目录：

```sh
cargo fmt --manifest-path agent-foundation/Cargo.toml -- --check
cargo check --manifest-path agent-foundation/Cargo.toml
cargo check --manifest-path agent-foundation/Cargo.toml --all-features
cargo test --manifest-path agent-foundation/Cargo.toml --all-features
cargo clippy --manifest-path agent-foundation/Cargo.toml --all-features --all-targets -- -D warnings
cargo run --manifest-path agent-foundation/Cargo.toml --example dry_run
```

Cargo 默认 feature 不启用 SQLite、Rig、RMCP 或 ACP。可通过 `--features durable-sqlite,adapter-all` 一次性打开独立验证。依赖固定在本目录的 `Cargo.lock`，没有更改根项目 Cargo。

测试日志和限制见 [验证报告](VERIFICATION-FOUNDATION-20261101.md)；独立构建通过**不代表** BrowserOS Push/PR Full Validation、安装版 GUI、性能压测和真实模型服务测试通过。

## 仍禁止生产使用的原因

1. 目前只是 Host 可调用的签名接口，没有 BrowserOS 实际登录态、OS 用户身份或 UI 操作证明；Demo 固定密钥和 principal 不能生产使用。
2. SQLite 任务和批准参数可能包含敏感信息，未接系统密钥/加密、备份、合法保留和数据迁移；同步 DB 调用必须放到非 GUI 调度器。
3. MCP 工具、ACP 生命周期、真实 Model Provider 网络、真实浏览器/Database/Git/Terminal Owner 均没有接线。RMCP 白名单仅代表独立模拟政策，不能替换现有产品业务授权。
4. Task 执行租约与本地 Outbox 是隔离能力；尚未实现多进程 leader 选举、跨机器 fencing、真实 ACK 消费者与长期重试治理。
5. Agent 仍只有有界的单轮引擎；Rig AgentRun 与现有状态机哪个是唯一执行 Owner 尚待双方案实验。切忌把两个 runtime 同时接进应用。
6. 需补齐官方协议完整互操作、真实外部幂等回传测试、GUI 安装版手动验收、性能基线与供应链许可证审查，才能进入主程序集成评审。

## 文件与计划

- [Open Source Integration Matrix](OPEN-SOURCE-INTEGRATION-MATRIX-20261101.md) — 哪些真实集成/没集成/为什么
- [Verification Report](VERIFICATION-FOUNDATION-20261101.md) — 真实测试、故障注入与缺口
- [Implementation Plan](IMPLEMENTATION-PLAN.md) — 后续接线与 Owner 映射
- [Upstream Research](UPSTREAM-INTERNALIZATION-RESEARCH-20261101.md) — 接入前的源码研究历史
- [Dependency Upgrade Guide](DEPENDENCY-UPGRADE-GUIDE-20261101.md) — Cargo/POM 对照与升级安全门禁

此目录代码仍属于独立实验。任何正式合并都必须另经 BrowserOS 业务 Owner 和 GUI/Full Validation 审核。
