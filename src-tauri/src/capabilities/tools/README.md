# Capability Native Module: `tools` (Rust)

> 迁移自 `src-tauri/src/tools.rs`（Native Physical Boundary Matrix Pilot 14）。
> 分类：**CAPABILITY_NATIVE(tools)**（矩阵 §2.3；target `src-tauri/src/capabilities/tools/`）。
> 对照矩阵：`docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §2.3 / §8.18。
> 同级 TS 能力：`src/capabilities/tools/`（成熟度 C1，命令接线仍经 `bridge.ts`）。

## 1. DDD 职责（Domain Responsibility）

内置种子工具清单 / 打包 + 子 webview 打开（M2-7 / M2-8，契约 F1~F9）：

- **种子嵌入**：编译期 `include_str!` 嵌入 5 个内置种子 HTML（`json` / `base64` / `timestamp` / `regex` / `cron`）；种子 HTML 与 `tools.rs` **同目录**（`*.html`），随模块迁入。
- `list_tools(app)`：只读枚举内置（固定 `BUILTIN_TOOLS`）+ 用户（`workspace/tools` 运行时扫描）工具，返回 `ToolMeta` 清单（不含 HTML 字节 / 绝对路径）；用户目录缺失返回空数组（不 panic）。
- `open_tool`：经 `tool://` 协议在隔离子 webview 打开工具（工具窗口 `tool-*` **零能力授予**）。
- `builtin_tool_html` / `tool_html` / `build_tool_list` / `validate_user_tool_path`（canonicalize + starts_with 越权防御）/ `error_page`。

## 2. 边界（Boundary / Non-Responsibility）

- **自带 2 个 `#[tauri::command]`**：`list_tools` / `open_tool`，命令体即住本文件，`generate_handler!` 中以 `tools::list_tools` / `tools::open_tool` 注册，**独立于 `bridge.rs` IPC 命令面 danger-zone**。
- **`list_tools` 只读**：不得含 `log_audit`（审计由调用方负责）。
- **种子 HTML 零外链**：不得含 `http(s)://` / 外部 `script` / `link` / `@import`（F1/F3/F4）。
- **`build.rs` 禁用 `include_dir`**（F8：零新依赖）。
- 无 WebView 创建（创建归 M2-8 前端）；本模块只产出 `tool://` 协议所需的 HTML 字节 / 元信息。

## 3. Commands

- `list_tools`：只读枚举，返回 `Vec<ToolMeta>`。
- `open_tool`：按 id 打开工具（内置走嵌入字节 / 用户走 `workspace/tools` 校验后读盘）。

## 4. Resources

- **文件系统**：用户工具读盘（`workspace/tools`，经 `validate_user_tool_path` canonicalize + starts_with 防越权）；内置 HTML 编译期嵌入（无运行时 IO）。
- `tool://` 协议：main.rs 注册 `register_uri_scheme_protocol("tool", ...)` 提供隔离子 webview 内容。
- 无 AppState 字段 / 无 PTY / 无 socket / 无后台 worker / 无定时器。

## 5. 生命周期（Lifecycle）

- 无状态：函数为无副作用的 IO 助手（除用户工具读盘）；启动即扫描 `workspace/tools`。

## 6. 依赖（Dependencies）

- `crate::domain`：`ToolMeta` / `ToolSource`（类型真源，SHARED）。
- `crate::workspace`：用户工具目录解析（SHARED_NATIVE_INFRASTRUCTURE）。
- `tauri`（`AppHandle` / `Manager` / `WebviewUrl` / `WebviewWindowBuilder`）；`std::fs` / `std::path`。

## 7. 禁止依赖（Forbidden Dependencies）

- 禁止 `crate::bridge`（AppState 共享态枢纽 / 命令 hub）—— 仅 `main.rs` `tool_html` 经 shim `crate::tools::` 调用，未直连 `bridge` 内部态。
- 禁止 `include_dir!` / 第二嵌入路径。
- 禁止种子 HTML 外链。
- 禁止 `std::process` / 网络 / PTY / 后台 worker。

## 8. Public / Native Contract

- 对外：`list_tools` / `open_tool` / `builtin_tool_html` / `tool_html` / `build_tool_list` / `validate_user_tool_path` / `error_page`。
- 既有 `crate::tools::` 调用点（main.rs `tool_html` ×1 + `generate_handler!` 中 `tools::list_tools` / `tools::open_tool`）经 `main.rs` 顶部 `pub use crate::capabilities::tools::tools;` re-export shim 解析，**未逐处改写**。
- TS 侧经 `bridge.ts` 命令调用，不直接 `import` 本 crate。

## 9. Security / ACL

- `list_tools` / `open_tool` 须在 `default-commands.toml` 的 `commands.allow` 中允许，且 `list_tools` 在 `list_artifact_images` **之前**（末位逗号坑）。
- 用户工具路径越权防御：`canonicalize` + `starts_with(workspace/tools)`，拒绝跳出工作区。
- 工具窗口 `tool-*` 零能力授予（隔离），不得在任意 capability 中出现 `tool-*` 条目（F4）。

## 10. Tests

- `tools.rs` 内 `#[cfg(test)]`（用户工具 2 MiB 上限 / 路径防御），运行：`cd src-tauri && cargo test capabilities::tools`。

## 11. Source of Truth

- 原生后端：`src-tauri/src/capabilities/tools/tools.rs` + 种子 `*.html`（本模块）。
- 领域类型：`src-tauri/src/domain.rs`。
- 命令体：`tools.rs` 内（自带命令）；前端接线 `src/bridge.ts` + `src/types.ts`。
- 契约真源：`scripts/check-seed-tools.py` / `check-tools-policy.py` / `check-native-capability-boundaries.mjs`（均钉 `capabilities/tools/tools.rs` + 同目录种子 HTML）。

## 12. Known Debt

- `list_tools` / `open_tool` 命令体本就不在 `bridge.rs`（与其它能力不同），故本模块不属 bridge 分解范畴；其 `main.rs` 注册经 shim 解析，待 SHIM_REMOVAL Pilot 统一改写。
- 工具窗口隔离依赖 `tool://` + 零能力授予契约，须由 `check-tools-policy.py` F4 持续守门。

## 13. Extraction Readiness

- **高。** 自包含命令 + 种子资源，依赖仅 SHARED（`domain` / `workspace`）+ 标准库 + `tauri`。
- 达到阈值后可随 `tools` 能力提级为 crate `mvp-tools-rust`（不改领域语义）。
