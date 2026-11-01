# Agent Channel Rust Foundation — execution plan

> **2026-11-01 implementation update**: The historical work-breakdown table below
> records the original planning state, not the latest SDK and test results.
> See [current integration matrix](OPEN-SOURCE-INTEGRATION-MATRIX-20261101.md)
> and [current verification](VERIFICATION-FOUNDATION-20261101.md)
> for confirmed work and outstanding production gates.

Status as of 2026-11-01: **isolated development only**. Production integration is explicitly out of scope.
Repository: code5m/browser-os. Branch: feat/agent-channel-rust-foundation-20261101.
Baseline: master 9607e260f86b15a730219c619ab5a20cc01efe65.

## Product decisions

- A single Agent Channel for the browser, editor, terminal, database and Git; specialized Agents contribute only strategy/context/tools.
- BrowserOS browser-style UI stays the product interface. This crate supplies no UI and owns no WebView.
- Rust implements the bounded orchestration and policy boundary; official Rust SDKs are external adapters rather than source-code forks.
- Keep exactly one owner of each tool side effect. Reuse existing BrowserOS Public APIs after separate approval, never reach into feature internals.
- Model output and MCP results are untrusted. Models may **propose**, not authorize or silently execute.

## 2026-11-01 Research gate: prioritize internalization, not bulk SDK adoption

See [UPSTREAM-INTERNALIZATION-RESEARCH-20261101.md](UPSTREAM-INTERNALIZATION-RESEARCH-20261101.md) for the six-project source-map and adoption decisions.

**Corrected dependency strategy**: keep BrowserOS Task/Permission/Context/Resource ownership independent; study Rig `AgentRun` (serialized I/O-free state machine), `rig-memory` and `rig-cassette` **before extending our own multi-turn engine**. New `engine.rs` is an experiment to compare against Rig, not automatically our production execution engine. Separate ACP editor-to-agent transport from internal task orchestration; A2A is deferred; RMCP owns wire protocol, not authorization; CloudEvents is an outbound event format, not our internal event bus; Goose is a whole-application architectural reference, not a dependency.

**Blocking gates before any external SDK feature**: (1) independent Cargo fmt/check/test/clippy evidence, (2) upstream call-chain + error/recovery analysis, (3) license/MSRV/feature/size review, (4) contract tests with mock providers/tools, (5) branch path-isolation check. No connection to BrowserOS main app until separately authorized.

## Work breakdown and acceptance gates

| Priority | Deliverable | Acceptance requirement | Status |
|---|---|---|---|
| P0 | Isolated crate, typed model/task/context/tool/event contracts | Cargo fmt/check/test/clippy succeed independently; no pre-existing files modified | CODE_ADDED / VERIFICATION_PENDING |
| P1a | Single model turn with bounded stream, timeout, explicit proposal | Mock provider -> task -> one pending approval; no tool executes before host approval | CODE_ADDED / VERIFICATION_PENDING |
| P1b | Trusted approval boundary and cancellation | Unknown tools denied, approval one-use and task-scoped, cancellation rejects stale replies | CODE_ADDED / VERIFICATION_PENDING |
| P1c | Recovery and event consistency | Snapshot restore drops pending side effects; monotonic sequence; replay gap signal | PROTOTYPE / VERIFICATION_PENDING |
| P2a | Rig adapter | Adapter tests for provider errors, streaming, tool calling, usage accounting | NOT_STARTED |
| P2b | MCP official Rust SDK adapter | Tool discovery/allowlist, stdio process isolation, argument/output validation | NOT_STARTED |
| P2c | ACP Rust adapter | Editor-to-agent RPC compatibility, cancellation/approval boundary; no Tauri integration | NOT_STARTED |
| P2d | A2A + CloudEvents (optional) | Inter-agent tasks and external event bridge only if needed; v1 envelope tests | NOT_STARTED |
| P3a | Durable task/outbox storage | Atomic task+event commits, recovery with schema upgrades, explicit user resume | NOT_STARTED |
| P3b | Security hardening | Authenticated principal-bound permits, scoped resources, secret filtering, injection tests | NOT_STARTED |
| P3c | Resource governance | Per-task lease, bounded queues, parallel work limits, external effect reconciliation | NOT_STARTED |
| P3d | System testing | Benchmark, fault injection and real external-provider tests; baseline comparisons | NOT_STARTED |
| P4 | Integration RFC only | Reuse existing agent/coding/MCP APIs, document exact file ownership and migration | RESEARCH_ONLY |

## Definition of done

1. No edits outside agent-foundation/; compare against master to confirm all existing paths untouched.
2. Independent crate passes cargo fmt/check/test/clippy on authorized developer machine or independent CI.
3. No new Tauri command, no browser/editor/terminal filesystem access, no app dependencies or modifications.
4. No model-originated approval or process execution; destructive effects require authenticated explicit user grant.
5. Cancellation, timeout, crash recovery and unknown-outcome behavior independently tested.
6. No claim of real SDK compatibility before protocol-interoperability tests.
7. Real GUI, full validation and installed package checks remain separate gates and cannot inherit this crate's result.

## Key dependencies to evaluate, not yet vendored

- Rig: https://github.com/0xPlaygrounds/rig (MIT; check versions/MSRV and dependencies before enabling).
- Official Rust MCP SDK: https://github.com/modelcontextprotocol/rust-sdk (verify exact crate license, features and standards compatibility).
- ACP SDK: https://github.com/agentclientprotocol/rust-sdk (Apache-2.0).
- A2A Rust SDK: https://github.com/a2aproject/a2a-rs (Apache-2.0; only for external agent cooperation).
- CloudEvents Rust SDK: https://github.com/cloudevents/sdk-rust (Apache-2.0; optional transport bridge).

## Risks and mitigations

- Existing BrowserOS has both legacy Agent shell and working Coding Agent. Do not create competing owner; integration only through a later explicit migration.
- GitHub connector writes files but cannot run cargo. Checks remain NOT_RUN until real compiler evidence exists.
- A tool future may be cancelled while a remote side effect still completes: require idempotency keys and explicit reconciliation before production.
- Snapshot serializes user goal and opaque resource identifiers: do not persist without host security controls.
- GUI and Grid/WebView behavior are unrelated to this independent crate and must not be inferred from unit tests.

## Next work order

1. Run real independent crate build/test and fix any compiler or logic findings in this folder only.
2. Add principal+resource+effect-bound host authorization and a task execution lease.
3. Add durable snapshot/event outbox as an opt-in storage adapter (not automatically active).
4. Evaluate real Rig and RMCP adapters behind optional cargo features, write contract tests before adopting.
5. Re-read existing Coding Agent and MCP implementations and refine the integration RFC; do not wire them up.

## Actual independent verification — 2026-11-01 follow-up

On the connected developer machine, source formatting was corrected; `cargo check` passed, `cargo test` passed (13 unit + 4 contract tests) and strict `cargo clippy --all-targets -- -D warnings` passed. See [VERIFICATION-20261101.md](VERIFICATION-20261101.md). This lifts only the standalone prototype's initial compilation gate; authenticated authorization, durable outbox, official SDK interoperability and GUI acceptance remain open.


## 2026-11-01 第二阶段落实情况（以真实代码为准）

| 原计划能力 | 本次变更 | 结果/限制 |
|---|---|---|
| P0 独立编译 | Cargo fmt/check/test/clippy、sdk 可选构建 | 独立 crate PASS，不代表 BrowserOS 主程序 |
| P1c 审批可信度 | HMAC-SHA256 签发/验签，绑定 principal+workspace+session+task revision+call+args+effect+resource revision+TTL | 本地伪造、篡改、重放测试 PASS；真实登录/密钥 Owner 未接 |
| P3a 事务任务与事件 | SQLite WAL + FULL sync + revision CAS + 原子 Outbox + 单进程任务恢复 | 测试 PASS，敏感数据加密与迁移未做 |
| P3c 工具副作用 | 工具 Owner 二次验证 + 单次授权 + 执行前持久 UNKNOWN + idempotency key + 对账 | mock PASS；真实服务幂等/退款/回滚未接 |
| P2a Rig | feature \`adapter-rig\`，官方 0.43 SDK DynModel、Completion、实时增量流、工具提议、usage | 编译/映射测试 PASS；真实模型未联调；rig-agent AgentRun 未引入 |
| P2b RMCP | feature \`adapter-rmcp\`，官方 3.8 SDK client/server 通过内存协议初始化与 tools/list/call | SDK 模拟互操作 PASS；真实 MCP / stdio / HTTP 未测，只读注册 |
| P2c ACP | feature \`adapter-acp\`，官方 3.6 SDK 稳定 v1 SessionNotification/JSON-RPC frame/Channel | SDK 内存通知 PASS；真实 IDE handshake/load/resume 未测 |
| P3d 故障注入 | SQLite Outbox 写入失败整体回滚、活动租约挡恢复、外部执行后超时、取消、资源失效、文件重启 | mock PASS；没有长时压力/网络故障/GUI 实验 |
| P2d 可选 A2A / CloudEvents | 未引入相应 SDK；CloudEvents 仅沿用原有元数据格式 | 意图保留，按外部集成需求另决策 |
| P4 BrowserOS 集成 | 未修改任何现有 app/Tauri/业务代码 | 仍待单独 RFC、Owner 与人工 GUI 验收 |

**下一阶段唯一 Owner 原则**：BrowserOS Host 实名身份/密钥/权限，TaskRegistry 任务生命周期，Rig 只做模型适配，RMCP 只做工具 wire，ACP 只做编辑器 wire，Tool Owner 负责真实资源版本和远端幂等。不得把 \`rig-agent\` AgentRun 与本项目 Task 状态机同时作为任务的最终 Owner，也不得把 MCP 工具 annotations 作为授权依据。

**仍需要完成**：真实 Host 鉴权、资源 Owner 的 Public API 接线、数据加密、持久化进程治理和日志保留、外部 Provider/ACP/MCP 全互操作、真实取消/崩溃对账、性能基线及 BrowserOS 安装版 GUI/Full Validation。最终上线仍是 **NOT READY**。
