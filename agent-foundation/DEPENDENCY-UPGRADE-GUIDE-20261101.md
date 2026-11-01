> **后续实现提醒（2026-11-01）**：本文是最初调研/升级计划的历史快照，其中“未接 SDK”等描述已经过时。当前实际依赖、版本、功能边界与验收以 [开源接入矩阵](OPEN-SOURCE-INTEGRATION-MATRIX-20261101.md) 和 [验证报告](VERIFICATION-FOUNDATION-20261101.md) 为准；固定 Rig 0.43.0、RMCP 3.8.0、ACP 3.6.0，不升级根项目 Rust 工具链。

# BrowserOS Agent Channel：Cargo 依赖接入与深度内化升级说明

> 日期：2026-11-01
>
> 仓库：`code5m/browser-os`
>
> 工作分支：`feat/agent-channel-rust-foundation-20261101`
>
> 变更范围：**仅新增本研究文档；不修改 Rust、Tauri、Vue、CI、master、perf 或其他分支代码。**
>
> 重要状态：**以下版本、功能开关和源码事实来自上游 GitHub Manifest/README 及 Docs.rs；尚未在 BrowserOS 上运行 Cargo 构建、互操作、性能测试或安装版 GUI 验收。** 如上游发布版本和仓库 HEAD 不一致，以正式发布 crate 的 `cargo info` / Cargo.lock 解析为最终准入依据。

## 0. 执行摘要：Cargo 就是 Rust 的 Maven/POM

| Maven / Java | Cargo / Rust | BrowserOS 的落地位置 |
|---|---|---|
| `pom.xml` | `Cargo.toml` | `agent-foundation/Cargo.toml` |
| Maven Central | crates.io | 官方 Rust SDK 包 |
| `mvn dependency:tree` | `cargo tree` | 查传递依赖、重复大版本 |
| `mvn test` | `cargo test` | 仅验证独立 Agent Crate |
| Maven 多模块 BOM / parent POM | Cargo Workspace 的 `[workspace.dependencies]` | 未来多 Crate 才考虑 |
| Maven 依赖锁/可复现解析 | `Cargo.lock` + 显式版本范围 | 内部可执行原型建议提交 lockfile |
| Maven profile | Cargo `[features]` + `--features` | SDK 按需启用，默认不开启 |
| 依赖覆盖 | `[patch.crates-io]` / 精确版本约束 | 特殊补丁方案，不常态化 |
| Maven 仓库镜像/离线 | Cargo registry + `cargo vendor` | 有明确离线交付需求时才评估 |

**架构结论**：把成熟 SDK 作为依赖直接引入 **独立的 Rust 适配器 Crate**，而非直接接入现有 BrowserOS 应用、更不复制六套协议实现。

- **应优先复用**：Rig（模型/模型回合）、RMCP（MCP 协议/传输）、ACP（编辑器与 Agent 通信）。
- **按需启用**：CloudEvents 的规范化事件导出、A2A 外部 Agent 间通信。
- **只借鉴架构**：Goose 的 Agent 状态机、审批与扩展机制；不直接依赖 Goose 主程序。
- **BrowserOS 自行持有**：任务归属、用户身份/权限、工作区范围、资源生命周期、审批记录、事件持久化及执行副作用的一致性。

## 1. 当前项目基线和升级约束

### 1.1 已检查到的文件

| 位置 | 已存在 | 影响 |
|---|---|---|
| `agent-foundation/Cargo.toml` | 独立 package `browseros-agent-foundation`，edition 2021，依赖 serde/serde_json/tokio/uuid | **首个依赖试验入口**；不影响主 Tauri 构建 |
| `src-tauri/Cargo.toml` | 独立 Tauri package `mvp-browser-os`，edition 2021，`mcp` feature | **本轮不得修改**，SDK 不可直接加到主程序 |
| `src-tauri/src/mcp.rs` | MCP capability 白名单/安全策略单一真源；源码说明阶段禁引 rmcp/tokio/server runtime | 升级需独立架构审核，不能绕过 Hard Stop |
| `src-tauri/src/mcp_server.rs` | feature-gated stdio-prep 服务，只有两个只读 introspection 工具绑定，其余 fail-closed | RMCP 可作为未来**传输层**；不能替换既有权限政策 |
| `src-tauri/src/capabilities/agent/coding.rs` | 当前 Coding Agent 已有会话、文件变更提议、用户审批与工作区锁 | 内化时做 adapter；避免平行重复的 Agent 执行器 |
| `src-tauri/src/capabilities/agent/coding_lsp.rs` | LSP session cache 已存在（最多 2 个，空闲 TTL 300s） | 不再单独造 LSP 持久会话 |
| `src/capabilities/agent/ui/CodingWorkbench.vue` | 当前模型流、编辑器、审批、MCP 与项目状态集中在一处 | 未来拆模块须保持唯一状态 Owner |
| `PROJECT-RULES.md` | 原生子 WebView 有已验证的生命周期/布局红线 | Cargo 实验绝不涉及 WebView GUI 改动 |

GitHub `master` 基线在本次核对时为 `9607e260f86b15a730219c619ab5a20cc01efe65`；独立分支在本次调研开始时为 `fc2b9cf04c32dc00dfde03b10828ff1395023ccf`。**文档之外的应用逻辑和历史分支保持原状**。

### 1.2 工具链门禁：MSRV 与 edition 是两件事

- Rust `edition = "2021"` 的 BrowserOS Crate **可以**依赖 Rust 2024 edition 的库，但编译器必须达到依赖的 MSRV。
- 项目根目录未发现统一 `rust-toolchain.toml`；`src-tauri/Cargo.toml` 也没有明确指定 Rust 版本。因此**实际 CI/开发机 rustc 版本尚未验证**。
- Rig 0.46.0 上游 workspace 明确要求 **Rust 1.97.1**；这比 ACP/RMCP 的 1.88 更高。如果现有工具链低于此版本，不能指望 `default-features = false` 绕开 Rust 版本要求。
- 不允许为引入 Rig 而偷偷修改主应用工具链、主 Cargo.toml 或 CI。先独立验收，后发专项升级提案。

## 2. 六个项目的 Cargo 包与依赖选择矩阵

> 下面的版本数字来自 2026-11-01 核对到的**上游源码 Manifest / 发布文档**。A2A 分包有不同发布节奏；Goose 的主分支版本与公开 crate 发布版本已有差异。最终采用前必须验证 crates.io 可解析性。不应直接认为每个 GitHub HEAD 都已发布。

| 项目 / 作用 | 真实 package 名称（Rust import） | 上游核对版本 | 上游 MSRV | 许可证 / 使用判断 | 采用决策 |
|---|---|---|---|---|---|
| Rig 模型协议 | `rig-core` (`rig_core`) | 0.46.0 | 1.97.1 | MIT | 优先研究；可选依赖 |
| Rig Agent 多轮执行 | `rig-agent` (`rig_agent`) | 0.46.0 | 1.97.1 | MIT | 优先研究 `AgentRun`，不要重复自研轮转引擎 |
| Rig 会话/回放 | `rig-memory`、`rig-cassette` | 0.46.0 | 1.97.1 | MIT | 仅读取需求对应功能，不默认引入录制功能 |
| ACP 稳定 v1 | `agent-client-protocol` (`agent_client_protocol`) | 3.6.0 | 1.88.0 | Apache-2.0 | 可选编辑器适配器；v2 仍属实验 |
| RMCP | `rmcp` (`rmcp`) | 3.8.0 | 1.88 | Apache-2.0（仓库 LICENSE 备注重许可过渡） | 优先做可选 client adapter；运行权限由宿主控制 |
| A2A 核心类型 | `a2a-lf`（上游 lib/import 为 `a2a`） | 0.4.2（源码 Manifest） | 1.85 | Apache-2.0 | 当前不需要，暂缓 |
| A2A 客户端 | `a2a-client-lf`（`a2a_client`） | 0.2.9（源码 Manifest） | 1.85 workspace | Apache-2.0 | 仅外部 Agent 互操作时考虑 |
| CloudEvents | `cloudevents-sdk`（`cloudevents`） | 0.9.0 | 包 Manifest 未声明；**待实际编译确认** | Apache-2.0；README 标记 API WIP/不稳定 | 先保持轻量 JSON 投影 |
| Goose 宿主参考 | `goose-agent` 等（Goose workspace 1.57.0；goose-agent 主分支 0.1.0-alpha.13） | 不作为整包升级目标；Docs.rs 最近检索到发布包 0.1.0-alpha.12 | Goose workspace 1.94.1 | Apache-2.0 | **只借鉴架构，不直接接入** |

### 2.1 Rig：优先选择部件，不要安装整个编码智能体

已审查：
- `rig-core`：统一模型消息、providers、tool contracts 和 streaming；
- `rig-agent/src/run/mod.rs`：可序列化、无 I/O 的 `AgentRun`，以 `CallModel → CallTools → Done` 驱动状态转换；
- `rig-memory`：存储/压缩/恢复会话；
- `rig-cassette`：效果记录与回放，并校验运行身份；
- Rig `rig` facade 默认打开 agent/reqwest/rustls 等功能，CLI/Harness 又含 Bevy 插件运行环境。

因此初期推荐 **`rig-core` 和 `rig-agent` 分包引入**，不通过 `cargo install rig` 把完整 CLI 当作 BrowserOS 内核。若编译器不足 1.97.1，继续维持自有 `ModelProvider` 抽象并延期实际依赖，**不自行重写成熟的多轮算法**。

参考：[Rig Cargo](https://github.com/0xPlaygrounds/rig/blob/main/Cargo.toml) · [Rig AgentRun](https://github.com/0xPlaygrounds/rig/blob/main/crates/rig-agent/src/run/mod.rs) · [Rig 0.46 迁移](https://github.com/0xPlaygrounds/rig/blob/main/MIGRATING.md)。

### 2.2 RMCP：直接复用协议，别造新的 MCP wire

上游 `rmcp` 默认 feature 含 `server` 与 `macros`；纯客户端应选择 `default-features = false` 并显式开启 `client` 和所需的 transport（`transport-io` / `transport-child-process` / HTTP client）。不同传输有不同的进程、网络与安全成本。

RMCP 3.8.0 还包含 MCP Tasks 扩展 `TaskManager`，但它管理的是**某一次 MCP 工具服务端的长任务**，不是 BrowserOS 的用户级 Agent Task。必须用适配器显式转换。

BrowserOS 已有 MCP capability allowlist 和 fail-closed 规则。RMCP **不能自行变成第二个权限真源**。在批准生产接入前只能使用 mock tools/禁止 spawn 与网络副作用的试验。

参考：[RMCP Cargo](https://github.com/modelcontextprotocol/rust-sdk/blob/main/crates/rmcp/Cargo.toml) · [RMCP TaskManager](https://github.com/modelcontextprotocol/rust-sdk/blob/main/crates/rmcp/src/task_manager.rs)。

### 2.3 ACP：只负责编辑器与智能体的通信

`agent-client-protocol` 3.6.0 默认 features 为空。可以根据部署角色选择：
- SDK 核心类型：无传输 feature；
- 本地端点（Agent stdio）：`stdio`；
- 编辑器负责启动本地进程：`process`；
- HTTP/WebSocket 是独立包 `agent-client-protocol-http`，不应默认引入；
- 稳定 v1 是第一阶段互操作目标；`unstable_protocol_v2`、`unstable_mcp_over_acp` 等不要默认开启。

**重要：ACP Session、RMCP Task、Rig AgentRun、BrowserOS Agent Task 是四种不同层级对象。** 不可直接混为一个枚举或一个数据库表。

参考：[ACP Cargo](https://github.com/agentclientprotocol/rust-sdk/blob/main/src/agent-client-protocol/Cargo.toml) · [ACP Session](https://github.com/agentclientprotocol/rust-sdk/blob/main/src/agent-client-protocol/src/session.rs)。

### 2.4 A2A、CloudEvents、Goose：不要为了“集齐六个”而带上整套依赖

- A2A 的核心 `a2a-lf` 是外部 Agent Card/Task/Artifact 的互操作数据模型。内部 Browser/Editor/Git Agent 之间以 BrowserOS API 调度更简单，暂不加 HTTP/gRPC 服务器。
- `cloudevents-sdk` 封装规范事件和 Kafka/NATS/HTTP bindings；BrowserOS 目前只需要有界、无敏感文本的事件记录和订阅。先保留现有 JSON 的 CloudEvents-compatible 输出，真正跨系统投递时才加依赖。
- Goose 是完整桌面/CLI/扩展产品，其核心的 `StateMachine`、`ToolConfirmationCoordinator` 和 `PermissionStore` 更适合作源码研究材料；它当前引用 RMCP 3.4.1、ACP 2.2.0，与新的 SDK 版本不完全一致。如果把 Goose 也依赖进来，会无谓增大升级与兼容面。

参考：[Goose StateMachine](https://github.com/aaif-goose/goose/blob/main/crates/goose-agent/src/machine.rs) · [Goose 审批](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/agents/tool_confirmation_coordinator.rs) · [CloudEvents Cargo](https://github.com/cloudevents/sdk-rust/blob/main/Cargo.toml)。

## 3. 可复制的 Cargo.toml 升级模板：**示例，不是已经验证的配置**

下面是**待批准后**可放入 `agent-foundation/Cargo.toml` 的独立试验模板。保留原来的 serde/serde_json/tokio/uuid 依赖；仅展示**拟新增部分**。每次只启用一个 adapter，不一次编译所有依赖。

~~~toml
# 新增到已有 agent-foundation/Cargo.toml（示例，尚未 cargo check）
[features]
default = []
adapter-rig-model = ["dep:rig-core"]
adapter-rig-run = ["adapter-rig-model", "dep:rig-agent"]
adapter-rmcp-client = ["dep:rmcp"]
adapter-acp-v1 = ["dep:agent-client-protocol"]
adapter-cloudevents = ["dep:cloudevents-sdk"]
adapter-a2a = ["dep:a2a"]

[dependencies]
# 原有：serde、serde_json、tokio、uuid，保留不变

# Rig: 部件式依赖。Rig 0.46.0 需要 rustc >= 1.97.1
rig-core = { version = "=0.46.0", default-features = false, optional = true }
rig-agent = { version = "=0.46.0", default-features = false, optional = true }

# RMCP: 仅客户端和 stdio/异步 IO 传输；不默认开启 server/macros
rmcp = { version = "=3.8.0", default-features = false, optional = true, features = ["client", "transport-io"] }

# ACP: 先只启用 stable v1 的核心类型；需要 stdio 时单独开启
agent-client-protocol = { version = "=3.6.0", default-features = false, optional = true }

# CloudEvents: package 与 Rust import 名不同
cloudevents-sdk = { version = "=0.9.0", default-features = false, optional = true }

# A2A: package=a2a-lf, Rust import=a2a
a2a = { package = "a2a-lf", version = "=0.4.2", default-features = false, optional = true }
~~~

**说明**：
1. 这是 `Cargo.toml` 依赖声明**草案**，未经 Cargo 实际解析与编译；尤其 A2A 需通过 `cargo info a2a-lf` 验证发布版本。
2. 这里 `adapter-rig-model` 只验证模型类型与接口；`rig-core` 禁用默认功能后不会自动具有可用的 HTTP/TLS 传输。真实调用模型时须再评估 `rig-reqwest` 或 facade 的 `reqwest/rustls` 组合。
3. ACP 需本地 stdio 时，为其加 `features = ["stdio"]`；编辑器发起进程连接时才加 `process`；跨网络需要独立 `agent-client-protocol-http`。
4. RMCP 改成进程启动器需要 `transport-child-process`；不应在未审查允许命令/路径之前开启。
5. `=0.46.0` 是试验用的**精确锁定**；普通 `"0.46.0"` 是兼容版本范围，不等同精确锁定。未来依赖审查后可恢复合理范围并依赖 `Cargo.lock` 保证构建复现。
6. Features 是**加法合并**的：同一版本包如果由其他路径启用了默认 features，当前依赖的 `default-features = false` 未必能将其彻底关闭。用 `cargo tree -e features` 实查。
7. 在 `agent-foundation` 仍是独立 package 时不必新增仓库根 `Cargo.toml`、`[workspace]` 或 `src-tauri` 依赖。以后多 crate 时才考虑 workspace 统一管理版本。

### 3.1 对接代码的分层方向（设计，不代表已实现）

~~~text
BrowserOS 原有产品与能力（不修改）
  Browser / Editor / Terminal / Git / Database / Coding Agent
                               |
                      [未来 Public API]
                               |
                    BrowserOS Agent Host
       TaskOwner | Authorization | Context | ResourceLease
       Audit/Events | DurableStore | Cancel/Recover
            /               |                 \\
      Rig Adapter        RMCP Adapter       ACP Adapter
  模型通信+Run步骤       工具协议传输       编辑器会话
            \\               |                 /
                 可选: CloudEvents / A2A
    Goose: 只作为已有设计、边界与故障测试的参考
~~~

最重要的边界：**模型只能提议工具调用，工具是否执行由 BrowserOS 的已认证用户授权与能力 Owner 决定**。即使 Rig/RMCP/ACP 内部也有自己的运行时和权限概念，也不能直接覆盖宿主规则。

## 4. 构建、性能、供应链的兼容风险

| 风险 | 为什么会出现 | 检查/治理方法 | 决策 |
|---|---|---|---|
| 编译器最低版本不一致 | Rig 1.97.1、ACP/RMCP 1.88、Goose 1.94.1 | `rustc --version`、独立 `cargo check`，查看 Cargo 报错 | CI/本机未核实前不启用 Rig |
| “一键加依赖”带入不需要的代码 | Facade 默认特性可能启用 Agent、网络/TLS、tools | `default-features = false` + 显式 feature，检查 `cargo tree -e features` | 分包优于全家桶 |
| 同一依赖被不同版本引用 | Rig 上游声明 rmcp 3.5，Goose 引用 rmcp 3.4.1/ACP 2.2.0，ACP upstream 包也引用 rmcp 3.4 | `cargo tree -d`、`cargo tree -i rmcp`，看**实际解析结果** | 不凭版本字符串断言一定有重复 |
| SemVer 自动升级 | `3.8.0` 通常允许 3.x 兼容升级；pre-1.0 规则更细 | PoC 使用 `=` 精确版本 + 提交 `Cargo.lock` | 升级走专项评审 |
| 功能特性叠加 | feature unification 使同一版本依赖的特性合并 | `cargo tree -e features`，记录启用来源 | 逐功能开启 |
| 网络/进程权限扩大 | RMCP client 可连接外部 MCP、spawn 进程；ACP process 可启动可执行程序 | 默认只 mock；主项目必须经过授权、命令允许列表、超时与结果脱敏 | 未验收前不给能力 |
| 发布许可要求 | MulanPSL-2.0 主仓库 + MIT/Apache-2.0 依赖；RMCP 仓库正经历 MIT→Apache 重许可 | 审核具体发布 crate LICENSE/NOTICE、依赖许可、`cargo deny` | 不复制未经审计源码 |
| 会话/事件内容泄露 | Rig Memory/Cassette、Goose PermissionStore 可能持有 prompt、tool args、上下文 | 单一敏感数据策略、最小保存、可禁用、加密、严格日志脱敏 | 禁止默认流量录制 |
| 不可逆外部副作用重复执行 | 取消/超时不一定能撤销外部工具调用 | 幂等键、执行前确认、未知结果对账、恢复后不自动重试 | P0 安全红线 |
| 冷启动/包体积变化 | 多 SDK 带来依赖、TLS、序列化、进程管理代码 | `cargo bloat`（工具可用时）、二进制大小、clean/warm build、RSS、冷启动基线 | 只在独立 crate 做初始对比 |

**不能直接从 `cargo tree -d` 的输出断言所有重复都是错误。** 例如两个引用范围均允许 RMCP 3.8.0 时 Cargo 可能合并为一个版本；但 Goose 使用 ACP 2.x 而本项目引 ACP 3.x 时则可能引入不同大版本。这取决于真实解析后的 Cargo.lock。

## 5. 实施步骤（严格从独立 Crate 到适配器）

### Gate 0 — 当前基础框架先真实验收

当前 `agent-foundation` 包含 model/task/tool/event/engine/contract-test 等代码，但没有可证明的成功构建记录。执行：

~~~bash
rustc --version
cargo --version
cargo fmt --manifest-path agent-foundation/Cargo.toml --all -- --check
cargo check --manifest-path agent-foundation/Cargo.toml --all-targets
cargo test --manifest-path agent-foundation/Cargo.toml --all-targets
cargo clippy --manifest-path agent-foundation/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path agent-foundation/Cargo.toml --example dry_run
~~~

要留存：完整命令、rustc/cargo 版本、测试数量、PASS/FAIL 原始日志和提交号。**任何失败先在独立 Crate 修复，不继续叠加 SDK。**

### Gate 1 — SDK 元数据和版本兼容性（只读）

~~~bash
cargo info rig-core@0.46.0
cargo info rig-agent@0.46.0
cargo info rmcp@3.8.0
cargo info agent-client-protocol@3.6.0
cargo info a2a-lf@0.4.2
cargo info cloudevents-sdk@0.9.0
cargo info goose-agent@0.1.0-alpha.13
~~~

如 `cargo info` 无法找到版本，不得臆造已发布的依赖声明；以正式发布的可解析版本替代并记录差异。

### Gate 2 — 只批准增加 Cargo.toml feature 开关

先试 RMCP 或 ACP 的**纯协议编译**，再试 Rig（可能触发 Rust 工具链升级）。不运行实际 MCP 子进程、不读取真实编辑器内容、不访问真实供应商模型。

~~~bash
cargo check --manifest-path agent-foundation/Cargo.toml --no-default-features
cargo check --manifest-path agent-foundation/Cargo.toml --no-default-features --features adapter-rmcp-client
cargo check --manifest-path agent-foundation/Cargo.toml --no-default-features --features adapter-acp-v1
cargo check --manifest-path agent-foundation/Cargo.toml --no-default-features --features adapter-rig-model
~~~

**仅当新增 feature 和依赖已经通过单独授权、提交后这些命令才有意义；本次只写文档，尚未添加 feature。**

### Gate 3 — 研究后写真正的 Adapter，而不是第二套 Runtime

- Rig adapter：统一 provider request/stream/usage/error；对照 `AgentRun` 的模型回合、工具结果配对、重试与恢复。失败原因必须记录。
- RMCP adapter：`initialize/list_tools/call_tool/cancel`，所有工具先过 BrowserOS capability allowlist 和审批；测试全用假服务器。
- ACP adapter：稳定 v1 的 `session/new`、`prompt`、通知/流式、取消/断线恢复；fake editor + fake agent 双向契约测试。
- CloudEvents adapter：内部 event metadata → v1.0 JSON；不要迁移事件存储/订阅所有权。
- A2A adapter：**暂停**，除非真的存在跨进程/跨设备 Agent 对接需求。
- Goose：完成状态机、审批、权限策略的对照审计，将有价值的**测试不变量**迁移到 BrowserOS，不复制整套实现。

### Gate 4 — 真实依赖树与安全审计

~~~bash
cargo metadata --manifest-path agent-foundation/Cargo.toml --format-version 1
cargo tree --manifest-path agent-foundation/Cargo.toml -d
cargo tree --manifest-path agent-foundation/Cargo.toml -e features
cargo tree --manifest-path agent-foundation/Cargo.toml -i rmcp
cargo test --manifest-path agent-foundation/Cargo.toml --all-targets
# 如已安装下面工具且已建立锁文件：
cargo audit
cargo deny check
~~~

提示：`cargo audit` / `cargo deny` 不是 Cargo 内置子命令，需要单独安装，且应在受控环境运行；执行位置、配置和是否检测到 `Cargo.lock` 都要确认。尽量避免在主仓库之外错误扫描其他 lockfile。

### Gate 5 — 仅在另一个正式评审阶段考虑 BrowserOS 接线

- 明确单一 Owner、Public API、权限校验、并发租约、事件/任务持久化与错误映射。
- 现有 Coding Agent 保留可用；新 Agent Channel 先作为 adapter 对现有执行入口做旁路验证。
- 现有 MCP hard-stop 必须走工程治理审批，绝不从独立 PoC 直接向 `src-tauri` 注入运行时。
- Push 和 PR 流水线均需 Rust 编译测试、前端体积、工程治理/Full Validation；安装版 GUI 冷启动和人工验收仍是额外独立关口。
- 在没有单独批准前，不提 PR 合并，不更新 master，不宣称应用已接入。

## 6. 如何升级与如何回滚

### 6.1 安全升级流程

1. 在独立工作分支上修改**单个** SDK 的精确版本；保留上一版 `Cargo.toml` / `Cargo.lock`。
2. 记录上游 CHANGELOG/MIGRATING/SECURITY 链接及兼容性变化。
3. 检查 Rust MSRV、默认 features、TLS stack、许可与漏洞告警。
4. 更新锁文件并校验：`cargo update --manifest-path agent-foundation/Cargo.toml -p rmcp --precise 3.8.0`（示例包，须在依赖已存在时运行）。
5. 运行格式、静态检查、各 adapter 契约测试与失败注入；覆盖取消、超时、双重审批、任务恢复、未知结果不自动重试。
6. 对比编译耗时、构建依赖数、包体积、运行内存与启动时延；没有测试数据不得宣称“优化成功”。
7. 单批提交，保留 Lockfile 与测试证据，严格审核后才考虑下一模块。

### 6.2 回滚流程

- 独立 Crate 升级失败：回滚**本分支**该批 Cargo.toml、Cargo.lock、适配器代码；不碰主项目和其他分支。
- 升级后进入主应用仍发生编译/权限/GUI 退化：通过 feature 关闭新 adapter，恢复以前可验收的 Cargo.lock、接口实现和主程序提交；运行同口径 Push/PR 和安装版 GUI 验收。
- 如发现安全问题：关闭执行型工具 feature，保留只读事件与诊断，禁止自动重试具有外部副作用的请求。
- 不以“还能启动”代替回滚成功；必须复验任务取消、审批和工作区文件安全。

## 7. 可验收的交付清单与停止条件

| 项目 | 完成标准 | 2026-11-01 状态 |
|---|---|---|
| 六项目 Cargo 包名、主干版本/MSRV、依赖关系 | Manifest/README/Docs.rs 交叉核对 | **首轮已核对**；实际 registry 解析待验证 |
| Maven→Cargo 对照与独立 Crate 设计 | 给出可复制 Manifest 与 feature 拆分 | **已编写草案** |
| 版本、许可、工具链风险评估 | 有风险矩阵与升级/回滚规则 | **已编写** |
| 当前基础 Crate 编译 / 测试 | rustc 真实日志 + cargo fmt/check/test/clippy | **未验证** |
| 真实 SDK 的 Cargo 依赖接入 | 锁文件、单功能依赖树、正式版本 | **未实施** |
| Rig 模型调用和 AgentRun 复用 | 可运行 mock + provider 合约/故障测试 | **未实施** |
| RMCP、ACP 双向互操作 | 独立客户端/服务端真实协议测试 | **未实施** |
| 外部副作用安全 | 权限、身份、scope、资源修订、审批、未知结果对账 | **未完成** |
| 应用接线/主分支合并 | 经明确批准，Push/PR+安装版 GUI 验收 | **禁止在当前阶段实施** |

**准入结论**：Cargo 可直接引入成熟开源项目，但现在只批准**依赖可行性研究与升级说明**；下一次代码阶段的正确顺序是：基础框架编译 → Rig 复用决策 → RMCP/ACP 受控适配器 → 依赖/安全测试 → 另行评审主程序接线。

## 8. 官方源码与资料索引（本报告核对来源）

1. Rust Cargo 官方手册：
   - https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html
   - https://doc.rust-lang.org/cargo/reference/features.html
   - https://doc.rust-lang.org/cargo/reference/workspaces.html
   - https://doc.rust-lang.org/cargo/commands/cargo-tree.html
2. Rig：
   - https://github.com/0xPlaygrounds/rig/blob/main/Cargo.toml
   - https://github.com/0xPlaygrounds/rig/blob/main/crates/rig-core/Cargo.toml
   - https://github.com/0xPlaygrounds/rig/blob/main/crates/rig-agent/src/run/mod.rs
   - https://github.com/0xPlaygrounds/rig/blob/main/MIGRATING.md
3. MCP：
   - https://github.com/modelcontextprotocol/rust-sdk/blob/main/Cargo.toml
   - https://github.com/modelcontextprotocol/rust-sdk/blob/main/crates/rmcp/Cargo.toml
   - https://github.com/modelcontextprotocol/rust-sdk/blob/main/LICENSE
4. ACP：
   - https://github.com/agentclientprotocol/rust-sdk/blob/main/Cargo.toml
   - https://github.com/agentclientprotocol/rust-sdk/blob/main/src/agent-client-protocol/Cargo.toml
5. A2A：
   - https://github.com/a2aproject/a2a-rs/blob/main/Cargo.toml
   - https://github.com/a2aproject/a2a-rs/blob/main/a2a/Cargo.toml
   - https://github.com/a2aproject/a2a-rs/blob/main/a2a-client/Cargo.toml
6. CloudEvents：
   - https://github.com/cloudevents/sdk-rust/blob/main/Cargo.toml
   - https://github.com/cloudevents/sdk-rust/blob/main/README.md
7. Goose：
   - https://github.com/aaif-goose/goose/blob/main/Cargo.toml
   - https://github.com/aaif-goose/goose/blob/main/crates/goose-agent/Cargo.toml
   - https://github.com/aaif-goose/goose/blob/main/crates/goose/src/agents/tool_confirmation_coordinator.rs
8. BrowserOS 相关文档：
   - [内化源码研究](UPSTREAM-INTERNALIZATION-RESEARCH-20261101.md)
   - [实施计划](IMPLEMENTATION-PLAN.md)
   - [逐文件优化审计](OPTIMIZATION-AUDIT.md)
   - [独立 Crate README](README.md)
