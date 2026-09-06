//! `mvp_core::core`：纯逻辑模块集合。
//!
//! 这里的模块必须满足「零 Tauri 耦合」——不得引用 Tauri 运行时、不得引用应用句柄与全局状态、
//! 不得引用命令层模块、不得自行起进程。规则由 `scripts/check-core-boundary.py` 在 pre-merge 断言。
//! （此处刻意不写出那些标识符的字面量：注释会被 `strip_comments` 剥离，但粗粒度 grep 仍会误报。）
//!
//! # 当前成员
//!
//! - `keyring_store`：系统密钥库封装（M5-1.a 的首个搬入件，用于验证边界与 re-export shim 成立）。
//!   选它作第一件的原因：零内部依赖、零 Tauri 耦合、无 `#[cfg(test)]` 块、
//!   引用点仅 `bridge.rs` / `sync.rs` 两处 —— 搬移后行为零变化，风险最低。
//!
//! # 后续切片（不在 M5-W1 范围）
//!
//! - 切片 0b：契约常量收口（`HARD_GRACE_SECS` / `MAX_TIMEOUT_SECS` / `MAX_TEXT_FIELD_BYTES` → `domain.rs`）
//!   与 `workspace.rs` 的 `_at`/`_in` 纯路径函数下沉。**必须先于其余模块搬入**
//!   （`domain.rs:1525-1526` 与 `scripts.rs` 的测试反向依赖 B 类模块，否则 core 会反向依赖 bin 而编译失败）。
//! - 切片 0a：A 类纯模块整文件带测试搬入。
//! - 切片 1/2：注入 `ProgressSink` / `PathResolver` / `RootsProvider` 后搬入 B 类模块，
//!   闭合「调度模块 → 命令层全局状态」这条反向边（当前位于 `scheduler.rs`，共 8 处）。
//!
//! 详见 `logs/assist/A2-M5-core-20260906-0749.md` §6。

pub mod keyring_store;
