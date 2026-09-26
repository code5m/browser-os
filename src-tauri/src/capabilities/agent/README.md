# Capability Native Module: `agent` (Rust)

> 迁移自 `src-tauri/src/agent.rs` / `src-tauri/src/agent_memory.rs`（Native Physical Boundary Matrix Pilot）。
> 对照矩阵：`docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §2.3 / §3.2 / §6.1。
> 同级 TS 能力：`src/capabilities/agent/`（maturity C1，只读壳，无执行后端）。

本目录承载两个纯模块（均无 Tauri 命令，不持有 `AppState`）：

## 1. DDD 职责（Domain Responsibility）

### 1.1 `agent.rs` — Agent 解析与校验
- `AgentDef::parse(&str) -> Result<AgentDef, String>`：外部 JSON → `AgentDef`（serde 保证类型合法，补语义校验）。
- `AgentDef::validate(&self) -> Result<(), PolicyError>`：id/version 非空、能力单源、`system_prompt`/`description` 不含凭据（K3 + 隐私）。
- `AgentDef::permission_preview(&self) -> PermissionPreview`：闸门档 + 所需能力列表（供 UI 权限预览）。

### 1.2 `agent_memory.rs` — Agent 记忆 KV 契约层
- 纯存储 + 纯策略：per-agent 软配额（字节，1 MiB）、agent 数上限（32）、总上限、隐私三重闸（字段名 + 字符串值双扫）、审计条目绝不含 `value`。
- 经 `mvp_core::core::seam::PathResolver::base_dir` 解析持久化目录；时钟以参数化 `now_secs` 传入，不依赖 `Clock` seam。

## 2. 边界（Boundary / Non-Responsibility）

- **不执行** Agent（无 `script_runner` 调用、无子进程、无 PTY、无 socket）。
- **不联网 / 不监听 / 不引入 rmcp / 后台 runtime**。
- **不引桥**（`crate::bridge`）；不持有 `AppState`、不注册 Tauri 命令。
- 不持久化 Agent 定义本身；`agent_memory.rs` 仅持久化记忆 KV（JSON）。
- 命令职责暂留 `bridge.rs`（LEGACY_MIXED_MODULE 残核），后续 bridge 拆分时迁出。

## 3. Commands

**无 Tauri 命令。** `agent_parse` / `agent_validate` / `agent_permission_preview` 当前注册在 `bridge.rs`，由前端 `bridge.ts` 经 `invoke` 调用，命令体内部委托本目录纯函数。

## 4. Resources

- `agent.rs`：无运行时资源（纯 CPU + serde 解析）。
- `agent_memory.rs`：文件系统（记忆 KV JSON 持久化，经 `PathResolver::base_dir`）；无进程 / 线程 / webview / PTY / socket / sqlite / keyring。

## 5. 生命周期（Lifecycle）

无状态、无生命周期。函数式纯逻辑，按调用即时执行、即时返回。`agent_memory.rs` 的 KV 为落盘 JSON，按 key 读写。

## 6. 依赖（Dependencies）

- `crate::domain` — `AgentDef` / `PermissionPreview` / `AclLevel` / `CapabilityRef`（SHARED_NATIVE_INFRASTRUCTURE）。
- `crate::security_policy` — `PolicyError` / `check_skill_capabilities` / `contains_credential_leak`（SHARED_NATIVE_INFRASTRUCTURE）。
- `mvp_core::core::seam::PathResolver`（SHARED_NATIVE_INFRASTRUCTURE，仅 `agent_memory.rs`）。

## 7. 禁止依赖（Forbidden Dependencies）

- 禁止 `crate::bridge`（含 `AppState` 共享态枢纽）。
- 禁止 `crate::script_runner` / `std::process` / `std::os::unix::net` / 任何网络 / 任何后台 worker。
- 禁止任何 Tauri `WebviewWindow` / `State` 引用。

## 8. Public / Native Contract

- 对外纯函数契约：`AgentDef::parse` / `validate` / `permission_preview`；`agent_memory` 的 store API（首期切片，仅被 `#[cfg(test)]` 消费，待命令层 M5-3.b 接入）。
- 无 FFI / 无 IPC contract。TS 侧经 `bridge.ts` 命令调用，不直接 `import` 本 crate。
- 类型真源：`crate::domain::AgentDef`（单一，禁止第二份 `AgentDef` 定义）。

## 9. 测试（Tests）

- `agent.rs` 内 `#[cfg(test)] mod tests`（解析 / 校验 / 权限预览）。
- `agent_memory.rs` 内 `#[cfg(test)] mod tests`（容量 / 隐私双扫 / 审计）。
- 专用静态 gate：`scripts/check-agent-memory-policy.py`（AGENT_KV_* 码位钉死容量 / 隐私 / 审计不变量）、`scripts/check-agent-skill-policy.py`（AGSK_*，含 `agent.rs` 的第二执行路径 / 内联 shell 守卫）。
- 运行：`cd src-tauri && cargo test agent`（或 `cargo test`）；双 checker 的 `--self-test` 须全绿。

## 10. Source of Truth

- 领域类型：`src-tauri/src/domain.rs`（`AgentDef` / `PermissionPreview`）。
- 安全策略：`src-tauri/src/security_policy.rs`（`PolicyError` / `contains_credential_leak`）。
- 命令接线（暂留）：`src-tauri/src/bridge.rs`（`agent_parse` / `agent_validate` / `agent_permission_preview`）。
- 前端接线：`src/bridge.ts`（`invoke("agent_parse" | "agent_validate" | "agent_permission_preview")`）。

## 11. Known Debt

- 两模块均 `#![allow(dead_code)]`：当前除 `#[cfg(test)]` 外无 native 调用方（命令体仍内联于 `bridge.rs`）；bridge 拆分后解除。
- `agent_memory.rs` 的 store API 尚未被命令层消费（M5-3 首期切片）；命令层 M5-3.b 接入时补 source check / ACL / bridge / 审计。
- 命令仍未迁离 `bridge.rs`（LEGACY_MIXED_MODULE 残核）；属矩阵 §3.2 计划内的后续批。

## 12. Extraction Readiness

- **高。** 纯模块、零命令、零 `AppState` 耦合、跨模块依赖仅 SHARED（`domain` / `security_policy` / `core::seam`）。
- 当前已可独立审核 / 独立发展；达到阈值后可提级为 crate `mvp-agent-rust`（不改领域语义）。
- 同批低风险候选完成（与 `skill` 并列首批）；下一批：`scripts.rs` + `snippets.rs`（bridge.rs 有 `crate::scripts/snippets::` 调用，需 re-export shim）。
