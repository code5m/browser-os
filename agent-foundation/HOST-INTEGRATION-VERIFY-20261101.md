> **2026-11-01 后续同步与 CI 治理补齐**：本文件下方列出的 Grid 重复测试、格式和历史 Pre-merge 失败是初版在旧 HEAD `fae385d9` 上的真实记录，并非最新状态。已将 Grid 修复后的 `master`（`8a5e8380`）无冲突合入 Agent Channel（`918bebc7`）；同步后本机 `cargo fmt` 三项 PASS、Tauri `cargo check` PASS、完整 Rust `cargo test` 583 PASS/17 ignored、Foundation 默认 24 PASS/全特性 43 PASS、Clippy PASS、前端生产构建 PASS。新增的 5 个 Tauri Host 命令已同步登记在 `docs/architecture/native-boundary/native-commands.yaml`，工程治理 `expectedRegisteredCommands=183`，本机 `npm run check:native-semantics` 183/183 PASS。**GitHub PR #48 的最终 CI / Pre-merge / 安装版 GUI 仍须依据最新提交的 CI 单独判定；不能从旧版结论推断最新通过。**

# BrowserOS Agent Channel — Tauri Host 接线与双 Runtime 实测记录（2026-11-01）

## 变更范围与事实

在隔离工作目录 `/home/ainfinit/PersonalProjects/browser-os-agent-foundation-20261101`，分支 `feat/agent-channel-rust-foundation-20261101`，保留已有业务 Runtime，新增 **默认关闭的实验室**。本次未安装客户端、未合并 `master`，且未获单独 commit/push 授权，所有代码保留在本地工作区。原有 `bridge.rs` 和 GTK/Grid 文件均未修改。

### 已实现（并区分实际效果）

| 请求 | 实现位置 | 真实能力和边界 |
|---|---|---|
| 正式 Tauri Host 接线 | `src-tauri/src/framework/agent_channel.rs`、`main.rs`、`src-tauri/permissions/default-commands.toml`、`src/bridge.ts` | 5 个主窗口限制的原生命令已注册：get/set config、离线 A/B benchmark、按配置路由单一离线任务、单次授权真实只读 Public API。格子/外部 Webview 不开放这些命令；不修改旧 Coding Agent 命令。 |
| Rig 多轮 Driver | `agent-foundation/src/rig_driver.rs`、`rig_run.rs` | `rig-agent 0.43.0` 真正依赖的官方无 I/O `AgentRun`，对 SDK `ModelTurn`／`CallTools`／`tool_results` 的状态转换，签名审批后回填工具结果并驱动第二轮；独立 2 回合 mock 测试通过。**尚未接真实生产模型供应商和已有 Coding Agent 的多轮会话**。 |
| 设置持久化与 UI | `agent_channel.rs`、`src/components/system/AgentRuntimeExperimentPanel.vue`、`SettingsPanel.vue` | 持久化 `app_data_dir/mvp-browser-os/agent-runtime-experiment.json`；版本 CAS，临时文件 0600、fsync、原子 rename；默认关闭，Native/Rig/Canary、比例、最大轮次、基准开关可调。设置页带原生状态和错误显示。 |
| 真实 Public API 执行 | `agent_channel.rs`、`agent-foundation/src/public_api.rs` 和 `tool.rs` | 有人工确认的主窗口请求经 HMAC 单次票据与来源检查，调用**现有** `bridge::tab_list`、`database::commands::db_list_connections`、`git::commands::git_status/git_diff`、Coding LSP Status 的原业务 Owner；只返回有界/删减元数据，写入 SQL/Git、浏览器导航、Shell、MCP 写工具均禁止。若真实 Owner 判定无权限或配置缺失，原样失败关闭。**目前没有完整 Agent 模型自行调用生产工具的自动往返流程**，工具仅在设置页人工触发。 |
| 同任务集对照测评 | `agent-foundation/src/benchmark.rs`、`experiment.rs`、设置页 | `tab_summary/git_review/database_report` 三种**固定 mock 内容**，分别跑 Native 与 Rig，记录成败、耗时（含微秒精度）、模拟 token；单任务也可按当前 Native/Rig/Canary 动态路由。**无真实模型网络/审批等待/GUI FPS/生产响应时间对比**；使用结果只能比较固定样例的编排开销。 |

### 安全和正式验收问题

1. **未升级旧生产 Agent 路径**：所有新增配置/路由只针对 Agent Runtime Lab 的新任务。旧 Coding Agent、Browser Agent、Database Agent、Git Agent 尚未统一迁移到新 Runtime。需业务 Owner 逐一批准 Public API 迁移、实际身份体系和权限。
2. **只读调用不能替换业务全功能**：数据库 SQL 具有潜在副作用，始终未接为 Agent 自动执行；Git 写/浏览器导航/Shell 同样没有开放。
3. **签名证明不是完整 OS 身份**：实验主窗口在有人工确认的调用内生成一次性主体与签名密钥。产品化需要系统级用户身份、工作区授权、密钥托管和持久化审计。
4. **持久化配置不等于持久化任务运行态**：配置 JSON 落盘，新 Task 分配与响应为实验请求即时创建，当前未提供跨进程的生产任务 Resume 和跨 Runtime 已运行任务迁移。
5. `emergency_fallback_to_native` 当前有配置字段，但没有开启新的实际**运行中回滚**，避免未知副作用重新执行。应通过单独回滚策略和幂等验证后启用。
6. SQLite/Outbox Foundation 在原独立 crate 可选 feature 中，**根 Tauri app 当前未启用持久化 Store/Outbox**；不能说生产中的任务状态与真实工具已持久化。
7. **GUI 安装包手动验收尚未执行**，不应继承独立 crate / Vite 构建的 PASS。

### 已识别的旧仓库阻塞（必须独立处理）

`src-tauri/src/grid_process.rs` 中两处 `#[cfg(test)] mod grid_connection_generation_tests` 分别位于 1229 和 1265 行。已通过 `git show HEAD:src-tauri/src/grid_process.rs` 证实在**本次修改前的 HEAD 提交**即同时存在。运行 `cargo test --manifest-path src-tauri/Cargo.toml agent_channel::tests` 因 `E0428` 失败，属于其他功能的原有测试模块冲突，**严格遵守仓库 AGENTS.md 停止条件，没有擅自修改 Grid/GTK/既有其他模块**。因此 Root 原生测试及 Full Validation 不能标记为 PASS。独立 Foundation 的全特性测试可独立完成。

### 验证范围

- `cargo check --locked --manifest-path src-tauri/Cargo.toml` — PASS（历史 `coding_turn_cancelable` dead_code warning）。
- `cargo test --locked --manifest-path agent-foundation/Cargo.toml --all-features` — PASS，35 库内 + 4 契约 + 4 故障注入，共 **43/43**。
- `cargo clippy --locked --manifest-path agent-foundation/Cargo.toml --all-features --all-targets -- -D warnings` — PASS。
- `npm run build` — 前端 Vite 构建通过；每次新增 UI 后需重新检查。
- `npm run generate:rust-contracts && npm run check:rust-contracts` — PASS（183 命令 / 19 事件，unverified 9 为既有验证指标，需逐项审核）。
- `npm run check:engineering-governance` — PASS（9 workflows、3 hooks、22 gates）。
- `cargo test --manifest-path src-tauri/Cargo.toml agent_channel::tests` — **FAIL**，以上原有重复 Grid test module 编译错误。
- `bash scripts/pre-merge.sh` — 已运行，发现当前 HEAD 的 `src-tauri/src/grid_process.rs` 格式不符合 rustfmt，报告 `FAIL: cargo fmt main`。按照仓库 AGENTS.md 的超出范围失败停止条件中止后续耗时检查，不将预合并门禁标记为 PASS。
- 安装版 GUI 人工验收：未执行，不得写 PASS。

### 开源依赖取舍

- **Rig AgentRun 0.43.0** 已新增 optional Cargo feature `experimental-rig-run` 并有真实独立状态机与工具回填测试；不是仅参考。
- **Rig core 0.43.0** 同步作为模型 SDK 接口，Tauri 的演示仅离线 mock，真实模型 SDK 网络暂未产品联调。
- **RMCP 3.8.0 / ACP 3.6.0** 仍保留独立 Foundation 可选 SDK 和模拟测试；未改原生生产 MCP/编辑器 wire。
- **A2A、CloudEvents SDK、Goose、ACP v2、rig-memory、rig-cassette** 不在当前生产依赖，未为了“全部集成”在业务基础中增加多余 Owner 或未批准的持久化敏感历史。

最后验收时只能把「独立能力实现」「Host 主窗口接入和只读人工验证入口」「离线双引擎可调测评」标为已开发，不能把「生产四类 Agent 统一 Runtime」「真实模型协议联调」「安装包 GUI 手工验收」标为完成。

### 本机离线实测样本

执行 `cargo run --locked --manifest-path agent-foundation/Cargo.toml --all-features --example benchmark`，每个运行时各 15 个样本，同任务集各 5 次：Native 15/15 PASS，中位数 25 μs、P95 样本 28 μs；Rig 15/15 PASS，中位数 27 μs、P95 样本 30 μs。输入/输出 token 都是编排测试中固定的模拟值，不是真实模型计费与运行时性能对比。原始输出保存在授权开发机 `/tmp/browseros-agent-benchmark-20261101.json`。
