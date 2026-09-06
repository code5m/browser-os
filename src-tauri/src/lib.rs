//! `mvp_core`：M5-1 的纯逻辑库边界。
//!
//! 形态：与现有二进制**同 Cargo package 的双 target**（`[lib] mvp_core` + 默认 bin）。
//! 选它的原因是不改动跨 crate 依赖图、编译模型变化最小，且切片可独立签署；
//! 阶段二再按需提升为 workspace member（见 `logs/assist/A2-M5-core-20260906-0749.md` §3.1）。
//!
//! # 边界硬规则（由 `scripts/check-core-boundary.py` 机器守门）
//!
//! - **R-B1** 本 crate 不得依赖 Tauri 运行时、Tauri 构建脚本或任何 Tauri 插件。
//!   ⚠️ 注意：**编译器守不住这条**——阶段一 lib 与 bin 共享 package 级 `[dependencies]`，
//!   所以在 core 里直接写对 Tauri 的 `use` 声明**照样能编译通过**。
//!   唯一能拦住它的是 pre-merge 阶段跑的 `scripts/check-core-boundary.py`。
//! - **R-B2** 不得 `use crate::bridge`，不得引用任何二进制专属模块
//!   （`bridge` / `main` / `terminal` / `grid_process` / `tools` / `shutdown`）。
//! - **R-B5** 不得出现第二执行路径（`std::process::Command` / `sh -c` / `bash -c`）；
//!   执行一律复用 `script_runner`。
//! - **R-B3/R-B4** 依赖方向单向：bin / web / mcp / cli → core，core 不反向依赖任何一方。
//!
//! # re-export 约定（改动最小化的关键）
//!
//! 二进制侧在 `main.rs` 写 `pub use mvp_core::<mod>;`，
//! 于是既有代码里的 `crate::<mod>::X` **无需任何改写**即可继续解析。
//! core 内部同理：本文件的 `pub use crate::core::*;` 让 core 内模块仍可写 `crate::<mod>::X`。
//! （该机制已在最小工程实测通过，见 A2 prework §3.2。）

pub mod core;

/// 让 core 内部（以及 `mvp_core` 的使用方）能以 `crate::<mod>` / `mvp_core::<mod>` 直接访问，
/// 避免模块搬迁时全量改写 `use` 语句。
pub use crate::core::*;
