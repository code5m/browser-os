//! Workspace 能力原生后端（应用数据目录持久化原语 + 文件系统命令，CAPABILITY_NATIVE(workspace)）。
//!
//! 承载两个物理模块：
//! - `workspace.rs`：应用数据根目录解析（`data_dir` / `workspace_dir` /
//!   `notes_dir` / `sessions_dir`）+ 原子 JSON 列表持久化（`save_json_list_at` /
//!   `load_json_list_at`，统一走 `crate::session::atomic_write` 的 tmp+rename 原语）
//!   + 正文文件助手（`body_path_in` / `write_body_at` / `read_body_at` /
//!   `delete_body_at`）+ 审计落盘（`log_audit`）。**本模块不含 `#[tauri::command]`**，
//!   `browse_workspace` / `workspace_images_dir` 等命令体注册在 `bridge.rs`，内部委托本模块助手。
//! - `fs_cmds.rs`：**自带 2 个 `#[tauri::command]`**（`reveal_path` / `move_path`），
//!   独立于 `bridge.rs` IPC 命令面 danger-zone（文件树"资源管理器打开"/拖拽移动）。
//!   命令体引用 `crate::bridge::allowed_roots` 做根目录边界校验（中耦合，保留）。
//!
//! 设计红线（承 BUG-HUNT B4-1/B4-2 修复）：
//! - 落盘一律走**唯一**原子写原语 `crate::session::atomic_write`，不新造第二写路径、不 `fs::write` 直接覆盖；
//! - **解析失败绝不静默清空**：备份为 `<file>.corrupt` + 告警后返回空（保证原字节可人工取回）。
//!
//! `workspace.rs` 迁移自 `src-tauri/src/workspace.rs`
//! （Native Physical Boundary Matrix Pilot 7，见
//! `docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §8.9）；
//! `fs_cmds.rs` 迁移自 `src-tauri/src/fs_cmds.rs`
//! （Native Physical Boundary Matrix Pilot 13，见同文档 §8.16）。
//! 既有 `crate::workspace::` 调用点（32 处：scheduler/tasks/plugin/scripts/tools/
//! bridge/workbench_smoke，及 `use crate::workspace;` 在 sync/bridge/tools）+ `crate::fs_cmds::`
//! （generate_handler! 中 `reveal_path`/`move_path`）经 `main.rs` 顶部 re-export shim 解析，无需逐处改写。
pub mod fs_cmds;
pub mod workspace;
