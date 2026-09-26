//! Git 能力原生后端（Git 同步，CAPABILITY_NATIVE(git)）。
//!
//! 承载 `sync.rs`：把 artifacts 推到配置仓库的 Git 同步内核（`repo_dir` / `sync_repo` /
//! commit / push / token 取用）；本模块**只做同步执行**，不持有 git 能力语义。
//!
//! **本模块不含 `#[tauri::command]`**：同步命令（`request_sync` / `confirm_sync` 等）
//! 注册在 `bridge.rs` 经 `generate_handler!`，命令体委托本模块内核。
//!
//! 迁移自 `src-tauri/src/sync.rs`
//! （Native Physical Boundary Matrix Pilot 12，见
//! `docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §8.15）。
//! 既有 `crate::sync::` 调用点（workbench_smoke.rs ×1：`repo_dir`）经 `main.rs` 顶部
//! re-export shim 解析，无需逐处改写。
//!
//! 依赖（SHARED）：`crate::domain` / `crate::keyring_store`（token）/ `crate::workspace`（repo dir）。
pub mod sync;
