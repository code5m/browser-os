//! Terminal 能力原生后端（PTY / 终端输出管道 + 生命周期内核，CAPABILITY_NATIVE(terminal)）。
//!
//! 承载 `terminal.rs`：终端输出管道三段式 worker→mpsc(128)→pump→sink（`EventSink` /
//! `ChannelSink` 双 sink）、临时历史环形数组（TerminalPane 切 Tab 卸载 xterm 但 PTY 仍活需
//! replay）、resize 真实化、`send_with_backoff`（10ms×2^n 上限 30s / 总预算 60s）、
//! `term_spawn` / `term_spawn_channel` 内核。
//!
//! **本模块不含 `#[tauri::command]`**：终端命令（`term_spawn_channel` / `terminal_resize` /
//! `terminal_send` 等）注册在 `bridge.rs` 经 `generate_handler!`，命令体委托本模块内核；
//! 本模块仅 `pub use crate::terminal::{TermInfo, TerminalSession}` 被 bridge.rs 再导出。
//!
//! 迁移自 `src-tauri/src/terminal.rs`
//! （Native Physical Boundary Matrix Pilot 11，见
//! `docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §8.14）。
//! 既有 `crate::terminal::` 调用点（bridge.rs ×2：`ChannelSink` / `EventSink` / `TermInfo` /
//! `TerminalSession`）经 `main.rs` 顶部 re-export shim 解析，无需逐处改写。
//!
//! 依赖（SHARED）：`crate::script_runner`（进程组回收 `process_group_alive`）。无 `bridge::AppState` 耦合。
pub mod terminal;
