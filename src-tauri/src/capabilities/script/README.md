# Capability Native Module: `script` (Rust)

> 迁移自 `src-tauri/src/scripts.rs`（M2-3）与 `src-tauri/src/snippets.rs`（M2-6），合并入 `capabilities/script/`（Native Physical Boundary Matrix Pilot 3）。
> 对照矩阵：`docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §2.3 / §3.2 / §8.3。
> 同级 TS 能力：`src/capabilities/script/`（maturity C1，命令接线仍经 `bridge.ts`）。
> 执行内核（`script_runner.rs`）属同一 `script` 能力，但本目录只承载**纯校验**，见 §7 禁止依赖。

## 1. DDD 职责（Domain Responsibility）

将脚本 / 命令片段的外部定义（JSON）做**静态语义校验与脱敏**，产出可持久化、可审计的 `ScriptMeta` / `CommandSnippet` 领域对象：

- `scripts.rs` — `validate_script_id` / `validate_meta`（定义期 fail-closed）/ `validate_body` / `validate_param_value`（9 条危险字符规则）/ `can_delete` / `is_secret_param` / `redact_param_value` / 持久化协议常量（`MAX_*`）/ 稳定错误码 `ScriptError`（20 个）。
- `snippets.rs` — `validate_snippet`（整元素占位 F2、argv[0] 字面量、占位符必须对应 params）/ `can_delete` / `builtin_snippets` / `merge_builtin_snippets` / 稳定错误码 `SnippetError`。

## 2. 边界（Boundary / Non-Responsibility）

- **不执行**脚本/片段（无 `script_runner` 调用、无子进程、无 PTY、无 socket）。
- **不安装 / 不联网 / 不下载**。
- **不引桥**（`crate::bridge`）；不持有 `AppState`、不注册 Tauri 命令。
- 不持久化；不持有任何会话语义。
- 执行职责归属调用方（经 `script_runner` 委托），本目录只产出"可校验/可审计"的领域对象。

## 3. Commands

**无 Tauri 命令。** 本目录是纯逻辑，全部脚本/片段命令（`script_list/add/update/remove`、`snippet_list/add/update/remove`、`run_command`/`run_script`/`cancel_script` 等）当前仍注册在 `bridge.rs`，由前端 `bridge.ts` 经 `invoke` 调用，并在命令体内部委托本目录纯函数。后续 bridge 拆分时，这些命令可迁至 `capabilities/script/commands.rs`（见矩阵 §3.2）。

## 4. Resources

无运行时资源（无进程 / 线程 / webview / PTY / socket / sqlite / keyring / 文件句柄）。纯 CPU + serde 解析。持久化由 `workspace.rs` 承担（本目录只定义 `MAX_*` 协议常量与调用 `workspace::save_scripts_at` 等，单测用临时目录）。

## 5. 生命周期（Lifecycle）

无状态、无生命周期。函数式纯逻辑，按调用即时执行、即时返回。

## 6. 依赖（Dependencies）

- `crate::domain` — `ScriptMeta` / `ScriptParam` / `ParamType` / `ScriptInterpreter` / `CommandSnippet` / `SNIPPET_MAX_TIMEOUT_SECS` / `SNIPPET_BUILTIN_CATEGORIES`（SHARED_NATIVE_INFRASTRUCTURE）。
- `crate::images` — `validate_id`（scripts.rs 复用文件名主干形态约束，SHARED_NATIVE_INFRASTRUCTURE）。
- `crate::security_policy` — `is_sensitive_query_key`（scripts.rs 脱敏兜底，SHARED_NATIVE_INFRASTRUCTURE）。
- 内部复用：`snippets` 复用 `scripts` 的 `validate_param_name` / `validate_script_id` / `MAX_*`（同一 `script` 能力，sibling 模块）。

## 7. 禁止依赖（Forbidden Dependencies）

- 禁止 `crate::bridge`（含 `AppState` 共享态枢纽）。
- 禁止 `crate::script_runner` / `std::process` / `std::os::unix::net` / 任何网络 / 任何后台 worker。
- 禁止任何 Tauri `WebviewWindow` / `State` 引用。
- 执行内核 `script_runner.rs` 是同一 `script` 能力的**另一模块**，但本目录不得反向依赖它（依赖方向单向：runner → 本目录纯函数）。

## 8. Public / Native Contract

- 对外纯函数契约：`validate_meta(&ScriptMeta)`、`validate_param_value(&str, ParamType)`、`can_delete(&ScriptMeta)`、`is_secret_param` / `redact_param_value`、`validate_snippet(&CommandSnippet)`、`merge_builtin_snippets(Vec<CommandSnippet>)` 等。
- 无 FFI / 无 IPC contract。TS 侧经 `bridge.ts` 命令调用，不直接 `import` 本 crate。
- 类型真源：`crate::domain::ScriptMeta` / `CommandSnippet`（单一，禁止第二份定义）。
- 既有调用点（`bridge.rs` / `script_runner.rs` / `tasks.rs` / `snippets.rs` 内）经 `main.rs` 顶部 `pub use crate::capabilities::script::{scripts, snippets};` re-export shim 解析，**未逐处改写**。

## 9. 测试（Tests）

- `scripts.rs` 内 `#[cfg(test)] mod script_domain_tests`（危险值/正常值/类型/脱敏/定义期校验/错误码稳定）+ `mod script_persistence_tests`（真实磁盘往返 / 原子写 / 路径穿越 / 历史 JSON / 内置不可删，约 22 项）。
- `snippets.rs` 内 `#[cfg(test)] mod tests`（s1–s14 / 整元素占位 / argv[0] 字面量 / 内置种子只读与合并安全，约 20 项）。
- 运行：`cd src-tauri && cargo test capabilities::script`（或 `cargo test` 全量）。

## 10. Source of Truth

- 领域类型：`src-tauri/src/domain.rs`（`ScriptMeta` / `CommandSnippet` / `ScriptParam` / `ParamType` / `ScriptInterpreter`）。
- 安全策略：`src-tauri/src/security_policy.rs`（`is_sensitive_query_key`）。
- 文件形态约束：`src-tauri/src/images.rs`（`validate_id`）。
- 命令接线（暂留）：`src-tauri/src/bridge.rs`（`script_*` / `snippet_*` / `run_command` / `run_script` / `cancel_script` 等）。
- 前端接线：`src/bridge.ts`（`invoke("script_list" | "snippet_add" | ...)`）。
- 门禁真源：`scripts/check-script-domain-policy.py`（扫描 `src-tauri/src/capabilities/script/scripts.rs`）、`scripts/check-command-domain-policy.py`（扫描 `src-tauri/src/capabilities/script/snippets.rs`）。

## 11. Known Debt

- `#![allow(dead_code)]` 部分纯函数（`validate_enum_value` / `check_required` / `is_secret_param` / `redact_param_value`）当前除 `#[cfg(test)]` 外无 native 调用方（审计/执行通道仍内联于 `bridge.rs` / `script_runner.rs`）；bridge 拆分后将解除。
- 命令仍未迁离 `bridge.rs`（LEGACY_MIXED_MODULE 残核的一部分）；属矩阵 §3.2 计划内的后续批。
- 执行内核 `script_runner.rs` 尚未迁至 `capabilities/script/`（同一能力、最高耦合批次，见矩阵 §8.3）。

## 12. Extraction Readiness

- **高。** 纯模块、零命令、零 `AppState` 耦合、跨模块依赖仅 SHARED（`domain` / `images` / `security_policy`）+ 内部复用 `scripts`↔`snippets`。
- 当前已可独立审核 / 独立发展；达到阈值后可提级为 crate `mvp-script-rust`（不改领域语义）。
- 同能力下一个低风险同批候选：`script_runner.rs`（执行内核，需先解除对 `bridge` hub 的耦合，见矩阵 §4）。
