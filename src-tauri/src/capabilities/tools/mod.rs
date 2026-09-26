//! Tools 能力原生后端（内置种子工具清单/打包 + 子 webview 打开，CAPABILITY_NATIVE(tools)）。
//!
//! 承载 `tools.rs`：
//! - 编译期 `include_str!` 嵌入 5 个内置种子 HTML（`json`/`base64`/`timestamp`/`regex`/`cron`）；
//!   种子 HTML 与 `tools.rs` **同目录**（`src-tauri/src/capabilities/tools/*.html`），随模块一并迁入。
//! - `list_tools` 命令：只读枚举内置 + 用户（`workspace/tools` 运行时扫描）工具清单。
//! - `open_tool` 命令：经 `tool://` 协议在隔离子 webview 打开工具（零能力授予，见矩阵 §2.4）。
//! - `builtin_tool_html` / `tool_html` / `build_tool_list` / `validate_user_tool_path`（canonicalize + starts_with 越权防御）。
//!
//! **本模块自带 2 个 `#[tauri::command]`**（`list_tools` / `open_tool`），命令体即住本文件，
//! `generate_handler!` 中以 `tools::list_tools` / `tools::open_tool` 注册（独立于 `bridge.rs` 命令面）。
//!
//! 设计红线（M2-7/M2-8 契约，F1~F9）：种子 HTML 零外链、零 bridge 写原语；`build.rs` 禁用 `include_dir`；
//! `list_tools` 只读（不写 `log_audit`）；用户工具路径必须 `canonicalize` + `starts_with(workspace/tools)` 防越权。
//!
//! 迁移自 `src-tauri/src/tools.rs`
//! （Native Physical Boundary Matrix Pilot 14，见
//! `docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §8.18）。
//! 既有 `crate::tools::` 调用点（main.rs `tool_html` ×1 + `generate_handler!` 中 `tools::*` ×2）经
//! `main.rs` 顶部 re-export shim 解析，无需逐处改写。
pub mod tools;
