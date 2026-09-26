# Capability Native Module: `workspace` (Rust)

> 迁移自 `src-tauri/src/workspace.rs`（Native Physical Boundary Matrix Pilot 7）。
> 分类：**CAPABILITY_NATIVE(workspace)**（矩阵 §2.3；target `src-tauri/src/capabilities/workspace/`）。
> 对照矩阵：`docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §2.3 / §8.9。
> 同级 TS 能力：`src/capabilities/workspace/`（成熟度 C1，命令接线仍经 `bridge.ts`）。
> 注意：本模块是**持久化原语切片**（M4-6/M4-7），**无 `#[tauri::command]`**；workspace 相关命令体在 `bridge.rs`。

## 1. DDD 职责（Domain Responsibility）

应用数据目录持久化原语（单一真源路径 = `app_data_dir/mvp-browser-os` 下的 JSON）：

- 目录解析：`data_dir` / `workspace_dir` / `notes_dir` / `sessions_dir`（`app.path().app_data_dir()` 派生）。
- 原子 JSON 列表落盘：`save_json_list_at` / `load_json_list_at`（统一走 `crate::session::atomic_write` 的 tmp+rename 原语）。
- 正文文件助手：`body_path_in` / `write_body_at` / `read_body_at` / `delete_body_at`（scripts/snippets 正文落盘）。
- 审计落盘：`log_audit`（审计条目，FIFO 上限由 caller 控制）。
- 被全仓复用：artifacts / repos / bookmarks / audit / scripts.json / snippets.json / tasks.json / task-runs.json / plugins.json / trusted-pubkeys.json 等持久化均经本模块。

## 2. 边界（Boundary / Non-Responsibility）

- **不含任何 `#[tauri::command]`**：`browse_workspace` / `workspace_images_dir` 命令体在 `bridge.rs`，本模块只提供目录/持久化助手。
- **不持有业务能力语义**：只序列化 `domain` 类型（Artifact/AuditEntry/Bookmark/CommandSnippet/ImageRef/RepoConfig/ScriptMeta），不定义领域行为。
- **不引入第二写路径**：一律 `crate::session::atomic_write`，禁止 `fs::write` 直接覆盖目标文件。
- **不静默清空**：解析失败 → `<file>.corrupt` 备份 + 告警 → 返回空（不丢原字节）。
- **不做并发控制**：调用方负责串行/锁；本模块函数无内部锁。
- 无 WebView / PTY / socket / 子进程 / 定时器。

## 3. Commands

**本模块无命令。** workspace 相关命令（`browse_workspace` / `workspace_images_dir`）归属 workspace 能力，命令体注册在 `bridge.rs`（经 `generate_handler!`），命令体调用本模块目录助手。命令体在 **bridge.rs 逐 command 分解阶段**迁移至 `capabilities/workspace/commands.rs`。

> **注意（Pilot 13 迁入 `fs_cmds.rs`）：** 同能力目录下的 `fs_cmds.rs` **自带 2 个 `#[tauri::command]`**——`reveal_path`（文件树"资源管理器打开"，`open::that` 调系统文件管理器）/ `move_path`（文件树拖拽移动，经 `crate::bridge::allowed_roots` + `security_policy::check_path_within_roots` 做根目录边界校验）。这 2 个命令**独立于 `bridge.rs` IPC 命令面 danger-zone**，命令体即住在 `fs_cmds.rs` 内（不在 `bridge.rs`），详见 §14。

## 4. Resources

- **文件系统**：所有持久化落于 `data_dir` 下的 JSON（原子写）；正文文件落于各元数据相邻目录。
- 无 WebView / PTY / socket / 进程 / 后台 worker / 定时器。
- 无 AppState 字段（本模块**无 `WorkspaceState`**，函数均取 `&AppHandle` 即时解析 `app_data_dir`，无长生命周期资源）。

## 5. 生命周期（Lifecycle）

- 无状态：函数为无副作用的纯 IO 助手（除 append 审计），调用即解析目录、即时读写。
- 无长生命周期资源；审计 FIFO 上限由 caller 控制（`log_audit` 只追加）。
- 首次启动：目录/文件不存在 → 返回空（正常，非数据丢失）；损坏 → `.corrupt` 备份后空。

## 6. 依赖（Dependencies）

- `crate::domain`：`Artifact` / `AuditEntry` / `Bookmark` / `CommandSnippet` / `ImageRef` / `RepoConfig` / `ScriptMeta` + `serde`（类型真源，SHARED）。
- `crate::session`：`atomic_write`（SHARED_NATIVE_INFRASTRUCTURE，唯一原子写原语）。
- 外部 crate：`tauri`（`AppHandle` / `Manager`）、`chrono`（`Utc` 审计时间戳）、`std::fs` / `std::path`。

## 7. 禁止依赖（Forbidden Dependencies）

- 禁止 `crate::bridge`（AppState 共享态枢纽 / 命令 hub）。
- 禁止第二写路径（`fs::write` 直接覆盖）。
- 禁止解析失败静默清空（须 `.corrupt` 备份）。
- 禁止 `std::process` / 网络 / PTY / 后台 worker。

## 8. Public / Native Contract

- 对外持久化原语：`data_dir` / `workspace_dir` / `notes_dir` / `sessions_dir` / `save_json_list_at` / `load_json_list_at` / `body_path_in` / `write_body_at` / `read_body_at` / `delete_body_at` / `log_audit` + 其它路径助手。
- 既有 `crate::workspace::` 调用点（32 处）+ `use crate::workspace;`（sync/bridge/tools）经 `main.rs` 顶部 `pub use crate::capabilities::workspace::workspace;` re-export shim 解析，**未逐处改写**。
- TS 侧经 `bridge.ts` 命令调用，不直接 `import` 本 crate。

## 9. Security / ACL

- 路径均为本地 `app_data_dir`，无网络/跨用户面。
- `log_audit` 是审计落盘（审计 FIFO 上限由 caller）；本模块不持有 ACL 决策。
- 审计内容脱敏由 caller 负责（见 matrix §2.4 既有债务 `log_audit` 上限 1000 FIFO）。

## 10. Tests

- `workspace.rs` 内 `#[cfg(test)] mod`（line ~523 / ~605 起：原子写 / 损坏备份 / body 助手 / 目录解析等）。
- 运行：`cd src-tauri && cargo test capabilities::workspace`（或 `cargo test` 全量）。

## 11. Source of Truth

- 持久化原语：`src-tauri/src/capabilities/workspace/workspace.rs`（本模块）。
- 领域类型：`src-tauri/src/domain.rs`。
- 命令体（暂留）：`src-tauri/src/bridge.rs`（`browse_workspace` / `workspace_images_dir`）。
- 前端接线：`src/bridge.ts` + `src/types.ts`。
- 门禁真源：`scripts/check-command-domain-policy.py` / `check-script-domain-policy.py` / `check-image-policy.py` / `check-scheduler-policy.py`（均钉 `capabilities/workspace/workspace.rs`）。

## 12. Known Debt

- B4-1/B4-2 已修复（原子写 + `.corrupt` 备份）。
- 内联依赖 `crate::session::atomic_write`（SHARED 原语，接受）。
- workspace 命令体仍在 `bridge.rs`（LEGACY_MIXED_MODULE 残核的一部分）；属矩阵 §8 末段 bridge 分解计划。
- 本模块跨能力复用最广（32 处），若后续 reclassify 为 SHARED_NATIVE_INFRASTRUCTURE，可再迁 `shared/`。

## 13. Extraction Readiness

- **高。** 纯持久化原语、零命令、零 AppState 字段、跨模块依赖仅 SHARED（`domain` / `session`）+ 标准库 + `tauri` AppHandle。
- 达到阈值后可随 `workspace` 能力提级为 crate `mvp-workspace-rust`（不改领域语义）。
- 同能力下一个低风险同批候选：workspace 命令体 `commands.rs`（需先解除对 `bridge` hub 的耦合，见矩阵 §4 / 协议 §8）。

## 14. `fs_cmds.rs`（Pilot 13，自带命令的物理模块）

> 迁移自 `src-tauri/src/fs_cmds.rs`（Native Physical Boundary Matrix Pilot 13，见
> `docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §8.16）。
> 分类：**CAPABILITY_NATIVE(workspace)**（矩阵 §2.3，target `src-tauri/src/capabilities/workspace/`）。

### 14.1 职责

- `reveal_path(app, path)`：用系统文件管理器打开指定路径（文件树右键"资源管理器打开"）；文件取其父目录，目录直接打开；`open::that(target)`。
- `move_path(app, src, dst_dir)`：将文件/目录移动到目标目录（文件树拖拽移动）；源与目标都须在允许根目录内，禁止移动到自身或自身子目录内。

### 14.2 命令面与耦合

- **自带 `#[tauri::command]`**（与 `workspace.rs` 的"零命令"不同）：命令体即住本文件，`generate_handler!` 中以 `fs_cmds::reveal_path` / `fs_cmds::move_path` 注册，**独立于 `bridge.rs` IPC 命令面 danger-zone**。
- 依赖：`crate::security_policy as sp`（`check_path_within_roots` 根目录边界校验）；`crate::bridge::allowed_roots(&app)`（中耦合，保留——属矩阵 §9 AppState/bridge hub 阶段待下沉的债务）。
- 无 `WorkspaceState` AppState 字段；命令取 `&AppHandle` 即时解析。

### 14.3 迁移要点

- `main.rs` 删除顶层 `mod fs_cmds;`，顶部加 re-export shim `pub use crate::capabilities::workspace::fs_cmds;`，`generate_handler!` 中 `fs_cmds::*` 调用点无需逐处改写即解析。
- 无门禁脚本钉死 `fs_cmds.rs`（0 脚本引用），无需 §5 路径修正。
- 命令名在 Tauri v2 ACL 中按名授权（非路径），迁移不改 ACL 条目。
