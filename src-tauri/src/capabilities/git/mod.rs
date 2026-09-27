//! Git 能力原生后端（Git 同步，CAPABILITY_NATIVE(git)）。
//!
//! 承载 `sync.rs`：把 artifacts 推到配置仓库的 Git 同步内核（`repo_dir` / `sync_repo` /
//! commit / push / token 取用）；本模块**只做同步执行**，不持有 git 能力语义。
//!
//! **命令层见 `commands.rs`**：git 能力全部 `#[tauri::command]`（git_status /
//! git_diff / git_branch_list / git_log / git_commit_diff / request_git_write /
//! confirm_git_write / request_sync / confirm_sync）已迁入该模块
//! （native-physical-batch-git），本模块 `sync.rs` 仅作 Git 同步内核（被命令层委托）。
//!
//! 迁移自 `src-tauri/src/sync.rs`
//! （Native Physical Boundary Matrix Pilot 12，见
//! `docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §8.15）。
//! 既有 `crate::sync::` 调用点（workbench_smoke.rs ×1：`repo_dir`）经 `main.rs` 顶部
//! re-export shim 解析，无需逐处改写。
//!
//! 依赖（SHARED）：`crate::domain` / `crate::keyring_store`（token）/ `crate::workspace`（repo dir）。
pub mod commands;
pub mod sync;
