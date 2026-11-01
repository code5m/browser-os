> **更新说明**：本文件为初版动态开关规划；之后已新增 Tauri Host 接线、离线 Rig Driver 和 UI。最新真实实现/验收/未完成项以 [HOST-INTEGRATION-VERIFY-20261101.md](HOST-INTEGRATION-VERIFY-20261101.md) 为准。

# Agent Runtime 实验开关与 Public API 迁移说明（2026-11-01）

分支：`feat/agent-channel-rust-foundation-20261101`。**本次仅在隔离的 `agent-foundation/` 添加选型、动态实验配置及 Public API 宿主契约。BrowserOS 主 Tauri 生产路径尚未启用，不能宣称四类 Agent 已经换成新 Runtime。**

## 真实发现的 Owner / 正式入口

| 能力 | 现有生产实现/接口（只读核实） | 接线边界 |
| --- | --- | --- |
| Browser | `src-tauri/src/capabilities/browser/commands.rs`、`src/bridge.ts` 的 `tab_go_back` / `tab_reload` | 当前 Browser WebView 有敏感生命周期/GTK 保护，Agent 不能直接抓内部 manager |
| Coding Agent | `src-tauri/src/framework/ai.rs::coding_service` | 原入口验证 `webview.label()=="main"` + `check_invocation_source`；不可绕过此验证，也不可直接复用无鉴权 request |
| Database | `src-tauri/src/capabilities/database/commands.rs::db_query`、`studio.rs::db_studio_call` | 业务 Owner 负责连接 ID、Keyring 凭据、SQL 风险分级、取消/超时和所有 SQL 写入判定 |
| Git Agent | `src-tauri/src/capabilities/git/commands.rs::git_status/git_diff` 与 `studio.rs::git_studio_confirm` | Git Owner 继续负责 repo 路径、审批、写入结果、rollback；不在 Foundation 重新造 Git 客户端 |
| 原 Agent 管理 | `src-tauri/src/capabilities/agent/README.md` | 当前仍是定义/ACL/Memory 管理，不具执行后端；**不要直接替换旧 `agent_chat`** |

## 新增的代码是什么，不是什么

- `src/public_api.rs`：已有公开命令名称与 Owner 的静态 Catalog，`PublicApiHost` trait，严格只读的 host-backed AgentTool，宿主明确安装前默认拒绝。**这是可编译、可测试的接线接口，不是 Tauri 已注册的新命令，也没有真正调用生产 DB/Git/Browser**。
- `src/experiment.rs`：`ExperimentConfig`、`ExperimentRouter`、`RuntimeAssignment`、`BenchmarkSample`。支持 default-off、Native、Rig、Canary、配置 CAS 修订、逐任务粘滞锁定、紧急关闭新任务的 Rig 路由。老任务分配永远不随配置更新被静默修改。
- `src/rig_run.rs`：可选 `rig-agent 0.43.0` 的真实 `AgentRun` 无 I/O 状态机实例、`CallModel/CallTools/Done` 步骤投影、JSON snapshot/restore；`CallTools` **只能交回既有 Task+Permission Owner**。**尚未给 Rig AgentRun 写真实多轮模型驱动，不得宣称它已完成可切换的生产执行路径**。

### 配置范例（JSON，供后续宿主 Settings/Feature Flag 投影）

```json
{
  "revision": 1,
  "enabled": true,
  "mode": "canary",
  "canary_percent": 10,
  "emergency_fallback_to_native": true,
  "benchmark_enabled": true,
  "max_turns": 3
}
```

**配置安全规则**：`enabled=false` 时必须 `mode=native`；当没有编译/加载 Rig AgentRun feature 时，Rig 路由直接拒绝；更新必须用期望 revision 做 CAS；每任务创建时分配并序列化固定 assignment，运行中的任务不能热切换。若发生副作用 Unknown，不允许所谓紧急回退自动重新执行一次。同一业务资源的两种模式也不能并行执行写入，否则测评会污染真实数据。

**下一阶段真实产品接线**：经过工程门禁批准后，由原生能力 Owner 提供 `PublicApiHost` 实现和同等权限校验；把只读功能在控制面板 opt-in 开启；为 Rig Driver 补多轮结果映射和审批 gate；通过正式 Tauri 命令或共享 runtime service 公开配置 API；settings 的存储路径和 UI 按 BrowserOS 的 Settings Owner 确定。不修改已有 `bridge.rs` 契约或 GTK/WebView 生命周期。

### 可比测评指标

统一 `BenchmarkSample`：成功率、总耗时、模型耗时、首 token 延迟、input/output token、工具审批等待时长、工具调用数、机器可识别错误码。后续产品测评需进一步加入冷启动、CPU、RSS、P95/99、重启恢复、稳定性、真实工具 outcome 和 GUI 响应时间；必须固定同一 prompt/tool catalog/model/provider、上下文、输入集、硬件环境。异步模型的真实网络返回不是确定性的，应多次重复并报告分布，而不是拿单次结果作为优劣证明。

### 暂不集成项目与原因

- **Rig AgentRun**：本轮已经把官方 `rig-agent 0.43.0` 以**可选 crate** 实际引入并测试步骤/快照；完整 Driver/业务工具链还没接。
- **A2A**：先留显式外部 Agent Gateway 接口；内部四类能力没必要转成四个远程微服务。
- **CloudEvents SDK**：保留原内部事件 + Outbox；对外事件消费者明确时再从现有事件序列映射，不让 CloudEvents 成为本地 Task 状态 Owner。
- **Goose**：完整 Agent app 有自己的交互、工具、模型和状态所有权；采用整体 crate 会复制现有 Runtime，保持只参考具体模式。
- **ACP v2**：不进入正式兼容层；维持 v1，与稳定 SDK 测试隔离。
- **Rig-memory / rig-cassette**：先做隐私/加密/存储归属和 replay 脱敏评审，再选择单独 feature，避免与现有 KV/Task store 重复、持久化敏感模型历史。

## 仓库保护条件与剩余事项

`AGENTS.md` 禁止未经批准修改 `bridge.rs`、GTK、已冻结 Native IPC 契约或自行认领 PENDING 需求；所以本轮的实现限定为独立模块。要称“真正接通”还必须：Root Cargo 正式依赖 feature/应用装配、可信用户授权、Public API Owner 实现、持久化 assignment 和设置存储、终端/DB/Git/Browser 的读/写边界、真实 Rig Driver 串行结果、GUI 安装版测评和 CI PASS。未经以上步骤任何一项只能标为未完成。
