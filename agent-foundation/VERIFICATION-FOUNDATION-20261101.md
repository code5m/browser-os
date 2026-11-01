# Agent Foundation 验证报告（2026-11-01，第二阶段）

范围：`code5m/browser-os` 的分支 `feat/agent-channel-rust-foundation-20261101`，**仅 `agent-foundation/`**。这份报告是当前构建结果，替代旧 `VERIFICATION-20261101.md` 中 SDK 尚未接入的历史状态。开发机：`rustc 1.96.0`、`cargo 1.96.0`、Ubuntu/Linux，使用独立 Cargo manifest。未使用 Codex 沙箱。

## 1. 真实命令与状态

| 验证项 | 命令（仓库根目录执行） | 结果 |
|---|---|---|
| Rust 格式 | `cargo fmt --manifest-path agent-foundation/Cargo.toml -- --check` | PASS |
| 默认 crate 构建 | `cargo check --locked --manifest-path agent-foundation/Cargo.toml` | PASS |
| 可选数据库和三 SDK 构建 | `cargo check --locked --manifest-path agent-foundation/Cargo.toml --all-features` | PASS |
| 默认特性测试 | `cargo test --locked --manifest-path agent-foundation/Cargo.toml -q` | PASS，15 单测 + 4 契约测试 |
| 全特性测试 | `cargo test --locked --manifest-path agent-foundation/Cargo.toml --all-features -q` | **PASS，28 单测 + 4 契约 + 4 故障注入 = 36/36** |
| 默认 Clippy 严格门禁 | `cargo clippy --locked --manifest-path agent-foundation/Cargo.toml --all-targets -- -D warnings` | PASS |
| 全特性 Clippy 严格门禁 | `cargo clippy --locked --manifest-path agent-foundation/Cargo.toml --all-features --all-targets -- -D warnings` | PASS |
| 模拟演示 | `cargo run --locked --manifest-path agent-foundation/Cargo.toml --example dry_run` | PASS：模型建议、签名审批、mock 工具执行、Task Completed |
| 变更格式 | `git diff --check` | PASS |
| 影响范围 | `git diff --name-only` 和 staged path 审查 | 仅 `agent-foundation/`；未改 root/ Tauri/ UI |

执行过程中发现并修复：最初默认特性编译有未使用的仅数据库功能方法；加上 Cargo feature gating，默认和全特性 Clippy 都通过。SDK 可编译不代表产品主程序可集成。

## 2. 实际验证哪些业务链

- **HMAC 可信审批的边界**：签名中校验 principal/workspace/session/task revision/tool+call/参数 SHA-256/effect/resource revision/有效期；不匹配 key、篡改和重复提交立即拒绝。Host 身份鉴别和密钥托管仍是部署前置条件。
- **事务状态与 Outbox**：同一次 SQLite 事务持久化 Task CAS、顺序事件、事件待投递记录；进程重启把 Running/等待审批转为 Paused；待审批不恢复为可执行凭证。
- **故障注入**：1) 外部 side effect mock 已启动但永不回应，超时后持久化 Unknown 且禁止再次使用同一键；2) 外部操作运行期间取消，Unknown 不变；3) 用户批准后资源版本漂移，执行前被 Owner 验证拒绝，未产生持久 side effect intent；4) 真实 SQLite 文件关闭再打开后恢复；5) SQLite trigger 强制破坏 Outbox INSERT，任务变更/事件一并回滚；6) 活跃执行租约阻止另一恢复流程。
- **Rig-core 0.43.0**：实际 SDK 类型请求、`DynModel<Completion>` 实时流桥、最终 tool suggestion/usage 映射；多工具提议/不支持内容拒绝。**未使用真实外部模型密钥或 API**。
- **RMCP 3.8.0**：官方 SDK client/server 使用 `tokio::io::duplex` 实际完成 initialize、list tools、call tool、返回。只读工具注册由 Host allowlist 决定，MCP hints 不能绕过；**没有对外 MCP 服务进程、HTTP、凭证或真实业务工具**。
- **ACP 3.6.0**：官方稳定 v1 SessionNotification/JSON-RPC typed payload，SDK `Channel::duplex` 传输给模拟编辑器并还原；工具建议不会被转成自动审批。**不是完整 ACP 生产生命周期/实际 IDE 联调**。

## 3. 仍不能合并成生产 Runtime 的原因

1. **实名认证、可信 UI 操作证明、密钥管理尚未接入**；签名只是宿主可用的最小安全接口。
2. **存储没有加密**。SQLite 里可能包含目标/资源/工具参数；需要 OS keychain、SQLCipher 或等价的加密方案、访问控制、备份、迁移和保留期限。
3. **外部副作用真实对账仍需服务支持**。UNKNOWN 保证不自动重试，但对账证据目前是由宿主提交的字符串；必须接真实业务 Owner 的幂等凭据与订单/文件/远端状态事实。
4. **并发/性能**：SQLite API 为同步访问；不得在 GUI 主线程阻塞调用；没有压测、跨设备 fencing、全局 leader 选举、统一调度与资源限额。
5. **Rig AgentRun 未内化**：当前 crate 只做一个 bounded turn；Rig AgentRun 与 BrowserOS Task 唯一 Owner 的双方案对照尚未完成。
6. **上游外部接口未完整联调**：真实模型网络、MCP stdio/HTTP+错误取消、ACP initialize/new/resume、A2A、CloudEvents SDK、真人审批 UI 未验收。
7. **产品级测试未跑**：BrowserOS 根 Rust/前端完整编译、Push/PR 流水线、Full Validation、安装版 GUI 冷启动、性能前后基线都没有从本次 crate PASS 获得任何结论。

## 4. 隔离与下一阶段

本次只是独立能力落地和 mock 安全链路测试；`master`、`perf`、`feature/perf-agent-channel-isolated-20261031` 等既有开发分支与 BrowserOS 原生运行时均未修改。后续真正集成须单独批准：先复用现有 Coding Agent/Browser/DB/Git/Terminal Public APIs，校验资源 Owner、身份与真实外部效果，并完成安装版人工验收和产品级 CI。期间保留 feature-gated 回滚方式。

逐项开源接入与延期原因：[OPEN-SOURCE-INTEGRATION-MATRIX-20261101.md](OPEN-SOURCE-INTEGRATION-MATRIX-20261101.md)。
