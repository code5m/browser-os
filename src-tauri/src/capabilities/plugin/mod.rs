//! Plugin 能力原生后端（纯策略切片，无 Tauri 命令、无运行时）。
//!
//! 承载 `plugin.rs`（M5-10 / M5-11 插件 manifest 与生命周期策略，Lane A9）：
//! `validate_plugin_manifest` / `verify_plugin_signature_structure` /
//! `can_transition` / `transition` / `permission_preview_for_plugin` / 稳定错误码 `PLUGIN_*`。
//!
//! 设计红线（承 M5-10 §4 / W6 FORBID）：无安装/卸载运行时、无下载/执行/联网、
//! 无独立 webview / stdio 进程、能力白名单单一真源 `security_policy::PLUGIN_CAPABILITY_V1`、
//! 无凭据进入 manifest 或审计。
//!
//! 迁移自 `src-tauri/src/plugin.rs`
//! （Native Physical Boundary Matrix Pilot 5，见
//! `docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §8.7）。
//! 既有 `crate::plugin::` 调用点（bridge.rs 插件命令体共 20 处）经 `main.rs` 顶部
//! re-export shim 解析，无需逐处改写。
pub mod plugin;
