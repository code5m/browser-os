//! Skill 能力原生后端（纯解析 / 校验 + 只读命令层）。
//!
//! 本目录承载：
//! - `skills.rs`：纯逻辑（`SkillDef` / `SkillExec` 的外部 JSON 解析与静态校验，
//!   K6 禁内联、能力单源、凭据防泄露；不执行、不安装、不联网、不引桥）。
//! - `commands.rs`：`skill_parse` / `skill_validate` / `skill_permission_preview`
//!   三个只读 Tauri 命令及其私有解析/校验 helper（CAPABILITY_NATIVE_ADAPTER），
//!   迁移自 `bridge.rs`（native-physical-batch-skill）。
//!
//! 设计约束（与 `skills.rs` 头注释一致）：
//! - 不执行、不安装、不联网、不持久化写。
//! - 执行层在调用方经 `script_runner` 委托，而非在此处直接执行。
//! - 迁移自 `src-tauri/src/skills.rs`（Native Physical Boundary Matrix Pilot，见
//!   `docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §6）。
pub mod commands;
pub mod skills;
