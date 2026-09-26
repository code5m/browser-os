# Capability Native Module: `plugin` (Rust)

> 迁移自 `src-tauri/src/plugin.rs`（Native Physical Boundary Matrix Pilot 5）。
> 分类：**CAPABILITY_NATIVE(plugin)**（矩阵 §2.3；target `src-tauri/src/capabilities/plugin/`）。
> 对照矩阵：`docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §2.3 / §8.7。
> 同级 TS 能力：`src/capabilities/plugin/`（成熟度 C1，命令接线仍经 `bridge.ts`）。
> 注意：本模块是 **纯策略切片**（M5-10 / M5-11，Lane A9），**无 Tauri 命令、无 AppState、无运行时**；W13 接命令层后生效。

## 1. DDD 职责（Domain Responsibility）

插件 manifest 与生命周期的**纯策略判定**（fail-closed），不触碰磁盘/网络/执行：

- `validate_plugin_manifest`：schema 边界 + 凭据泄露（`security_policy::contains_credential_leak`）+ 形态③入口 + 能力白名单 + 哈希/签名结构（不做文件 I/O、不做真验签）。
- `verify_plugin_signature_structure`：仅签名**结构**校验（真 Ed25519 验签由运行时 lane 完成，本模块不引加密 crate、不联网）。
- `can_transition` / `transition`：生命周期状态机（13 边，纯，无文件变更）。
- `permission_preview_for_plugin`：派生于 A5 `PermissionPreview`，供 UI 渲染。
- 9 个稳定错误码（`PLUGIN_*`）。

## 2. 边界（Boundary / Non-Responsibility）

- **不安装 / 不卸载 / 不执行 / 不下载 / 不联网 / 不验签真值**（仅结构校验）。
- **不引桥**（`crate::bridge`）；不持有 `AppState`、不注册 Tauri 命令。
- 不读写磁盘清单（`plugins_dir`、manifest 文件）；不跨 store 读 agent memory。
- 形态③唯一合法：入口 URL 同源 webview；排除独立 webview / stdio 进程。

## 3. Commands

**无 Tauri 命令。** 插件生命周期命令（`plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_list` / `plugin_get` / `plugin_keys_add/list/remove` 等）当前仍注册在 `bridge.rs`，由 `bridge.ts` 经 `invoke` 调用，并在命令体内部委托本模块纯函数（`crate::plugin::*` 共 20 处调用）。后续 bridge 拆分时，这些命令可迁至 `capabilities/plugin/commands.rs`（见矩阵 §3.2）。

## 4. Resources

无运行时资源（无进程 / 线程 / webview / PTY / socket / sqlite / keyring / 文件写句柄）。纯 CPU + serde 解析 + `sha2` 哈希。

## 5. 生命周期（Lifecycle）

无状态、无生命周期（模块自身）。`transition()` 是纯状态机函数，按调用即时返回新状态。

## 6. 依赖（Dependencies）

- `crate::domain` — `PluginManifest` / `PluginRecord` / `PluginState` / `PluginSignature` / `TrustedKeyRecord` 等共享类型（SHARED_NATIVE_INFRASTRUCTURE，类型真源）。
- `crate::security_policy` — `PLUGIN_CAPABILITY_V1`（能力白名单单一真源，fail-closed 空集合）/ `contains_credential_leak` / `PolicyError`（SHARED_NATIVE_INFRASTRUCTURE）。
- 外部 crate：`sha2`（仅哈希结构校验）。

## 7. 禁止依赖（Forbidden Dependencies）

- 禁止 `crate::bridge`（`AppState` 共享态枢纽）。
- 禁止 `std::process` / `Command::new` / 网络 / 后台 worker / 任何安装或文件写。
- 禁止任何 `WebviewWindow` / `State` 引用。
- 禁止 `crate::agent_memory`（不跨 store 读 agent memory）。

## 8. Public / Native Contract

- 对外纯函数契约：`validate_plugin_manifest` / `verify_plugin_signature_structure` / `can_transition` / `transition` / `permission_preview_for_plugin` / 稳定错误码 `error_code`。
- 无 FFI / 无 IPC contract。TS 侧经 `bridge.ts` 命令调用，不直接 `import` 本 crate。
- 类型真源：`crate::domain` 中的插件类型（单一，禁止第二份定义）。
- 既有 20 处 `crate::plugin::` 调用点（bridge.rs 插件命令体）经 `main.rs` 顶部 `pub use crate::capabilities::plugin::plugin;` re-export shim 解析，**未逐处改写**。

## 9. 测试（Tests）

- `plugin.rs` 内 `#[cfg(test)] mod`（line ~547 起，11 例）：manifest 校验 / 签名结构 / 状态机转移 / 能力白名单 / 凭据泄露。
- 运行：`cd src-tauri && cargo test capabilities::plugin`（或 `cargo test` 全量）。

## 10. Source of Truth

- 领域类型：`src-tauri/src/domain.rs`（插件 DTO）。
- 安全策略：`src-tauri/src/security_policy.rs`（`PLUGIN_CAPABILITY_V1` / `contains_credential_leak`）。
- 命令接线（暂留）：`src-tauri/src/bridge.rs`（`plugin_*` 命令体调用 `crate::plugin::*`）。
- 前端接线：`src/bridge.ts`（invoke）+ `src/components/workspace/PluginPanel.vue`。
- 门禁真源：`scripts/check-plugin-policy.py`（扫 `src-tauri/src/capabilities/plugin/plugin.rs`）、`scripts/check-plugin-privacy.py`、`scripts/check-agent-skill-policy.py`（`AGSK_PLUGIN_*`）。

## 11. Known Debt

- `#![allow(dead_code)]`：纯策略切片仅被单测与未来运行时 lane 消费，无当前 native 调用方（W13 前）；bridge 插件命令体委托后解除。
- 插件命令仍未迁离 `bridge.rs`（LEGACY_MIXED_MODULE 残核的一部分）；属矩阵 §3.2 计划内的后续批。
- W13 命令层接入后，本目录需扩充 `commands.rs` 与执行锁定检查（W13 Stage-I 禁止 `plugin_invoke` 等执行面）。

## 12. Extraction Readiness

- **高。** 纯模块、零命令、零 `AppState` 耦合、跨模块依赖仅 SHARED（`domain` / `security_policy`）+ 外部 `sha2`。
- 当前已可独立审核；达到阈值后可随 `plugin` 能力提级为 crate `mvp-plugin-rust`（不改领域语义）。
- 同能力下一个低风险同批候选：W13 命令层 `commands.rs`（需先解除对 `bridge` hub 的耦合，见矩阵 §4）。
