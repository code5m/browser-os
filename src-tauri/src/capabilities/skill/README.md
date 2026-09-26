# Capability Native Module: `skill` (Rust)

> 迁移自 `src-tauri/src/skills.rs`（Native Physical Boundary Matrix Pilot）。
> 对照矩阵：`docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §2.3 / §3.2 / §6。
> 同级 TS 能力：`src/capabilities/skill/`（maturity C1，只读壳，无执行后端）。

## 1. DDD 职责（Domain Responsibility）

将外部 Skill 定义（JSON）解析为 `SkillDef` 领域对象，并做**静态语义校验**：

- `SkillDef::parse` — 外部 JSON → `SkillDef`（serde 保证类型合法，此处补语义校验）。
- `SkillDef::validate` — id/version 非空、能力单源（`check_skill_capabilities`）、描述/输入不含凭据泄露。
- `SkillDef::permission_preview` — 闸门档（`acl`）+ 所需能力列表（供 M5-6 UI 权限预览）。
- `SkillExec::validate` — 递归校验执行体不含内联 shell（K6 fail-closed 守卫）。

## 2. 边界（Boundary / Non-Responsibility）

- **不执行** Skill（无 `script_runner` 调用、无子进程、无 PTY、无 socket）。
- **不安装 / 不联网 / 不下载**。
- **不引桥**（`crate::bridge`）；不持有 `AppState`、不注册 Tauri 命令。
- 不持久化；不持有任何会话语义。
- 执行职责归属调用方（经 `script_runner` 委托），本模块只产出“可执行的 SkillDef”。

## 3. Commands

**无 Tauri 命令。** 本模块是纯逻辑，全部命令（`skill_parse` / `skill_validate` / `skill_permission_preview`）当前仍注册在 `bridge.rs`，由前端 `bridge.ts` 经 `invoke` 调用，并在命令体内部委托本模块纯函数。后续 bridge 拆分时，这些命令可迁至 `capabilities/skill/commands.rs`（见矩阵 §3.2）。

## 4. Resources

无运行时资源（无进程 / 线程 / webview / PTY / socket / sqlite / keyring / 文件句柄）。纯 CPU + serde 解析。

## 5. 生命周期（Lifecycle）

无状态、无生命周期。函数式纯逻辑，按调用即时执行、即时返回。

## 6. 依赖（Dependencies）

- `crate::domain` — `SkillDef` / `SkillExec` / `PermissionPreview` / `AclLevel` / `CapabilityRef` 领域类型（SHARED_NATIVE_INFRASTRUCTURE）。
- `crate::security_policy` — `PolicyError` / `check_skill_capabilities` / `contains_credential_leak`（SHARED_NATIVE_INFRASTRUCTURE）。

## 7. 禁止依赖（Forbidden Dependencies）

- 禁止 `crate::bridge`（含 `AppState` 共享态枢纽）。
- 禁止 `crate::script_runner` / `std::process` / `std::os::unix::net` / 任何网络 / 任何后台 worker。
- 禁止任何 Tauri `WebviewWindow` / `State` 引用。

## 8. Public / Native Contract

- 对外纯函数契约：`SkillDef::parse(&str) -> Result<SkillDef, String>`、`SkillDef::validate(&self) -> Result<(), PolicyError>`、`SkillDef::permission_preview(&self) -> PermissionPreview`、`SkillExec::validate(&self) -> Result<(), PolicyError>`。
- 无 FFI / 无 IPC contract。TS 侧经 `bridge.ts` 命令调用，不直接 `import` 本 crate。
- 类型真源：`crate::domain::SkillDef`（单一，禁止第二份 `SkillDef` 定义）。

## 9. 测试（Tests）

- `skills.rs` 内 `#[cfg(test)] mod tests`：safe_skill_validates / unknown_capability_rejected / credential_in_description_rejected / empty_id_rejected / sequence_exec_validates / parse_json_roundtrip（6 项）。
- 运行：`cd src-tauri && cargo test skills`（或 `cargo test`）。

## 10. Source of Truth

- 领域类型：`src-tauri/src/domain.rs`（`SkillDef` / `SkillExec`）。
- 安全策略：`src-tauri/src/security_policy.rs`（`check_skill_capabilities` / `contains_credential_leak` / `PolicyError`）。
- 命令接线（暂留）：`src-tauri/src/bridge.rs`（`skill_parse` / `skill_validate` / `skill_permission_preview`）。
- 前端接线：`src/bridge.ts`（`invoke("skill_parse" | "skill_validate" | "skill_permission_preview")`）。

## 11. Known Debt

- `SkillDef::parse` 仅支持 JSON（YAML 未引入 `serde_yaml` 依赖，W4 决策）。YAML 支持待 M5-4.b 视需补，复用同一 `SkillDef` 类型。
- `#![allow(dead_code)]`：本模块当前除 `#[cfg(test)]` 外无 native 调用方（命令体仍内联于 `bridge.rs`）；bridge 拆分后将解除。
- 命令仍未迁离 `bridge.rs`（LEGACY_MIXED_MODULE 残核的一部分）；属矩阵 §3.2 计划内的后续批。

## 12. Extraction Readiness

- **高。** 纯模块、零命令、零 `AppState` 耦合、跨模块依赖仅 SHARED（`domain` / `security_policy`）。
- 当前已可独立审核 / 独立发展；达到阈值后可提级为 crate `mvp-skill-rust`（不改领域语义）。
- 下一个低风险同批候选：`agent.rs` + `agent_memory.rs`（同样纯 / 低耦合）。
