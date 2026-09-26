# Capability Native Module: `database` (Rust)

> 迁移自 `src-tauri/src/database.rs`（Native Physical Boundary Matrix Pilot 8）。
> 分类：**CAPABILITY_NATIVE(database)**（矩阵 §2.3；target `src-tauri/src/capabilities/database/`）。
> 对照矩阵：`docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §2.3 / §8.10。
> 同级 TS 能力：`src/capabilities/database/`（成熟度 C1，命令接线仍经 `bridge.ts`）。
> 注意：本模块是**数据库运行时基座**（M4-2 Lane A3），**无 `#[tauri::command]`**；db 命令体在 `bridge.rs`。

## 1. DDD 职责（Domain Responsibility）

数据库运行时基座（与 `domain.rs` 的 `DbConnectionConfig`/`DbErrorCode`/`DbSslMode`/`SupportedDb` 协同）：

- 连接池：`DbPool`（同步 `rusqlite` bundled + `mysql` + `postgres` stub）。
- 取消：`QueryCancel`（L1/L2/L3 取消三层，参考 M4-1.c）。
- 凭据：`credential_key(conn_id)` + `DB_CRED_PREFIX = "db:"`（键 `db:<conn_id>`，只经 `KeyringStore`，绝不落 DSN）。
- 错误：`DbErrorCode`（18 码闭合）+ `sanitize_message`（出模块前脱敏）。
- 上限常量：`DB_MAX_SQL_BYTES` / `DB_MAX_ROWS` / `DB_MAX_RESULT_BYTES` / `DB_DEFAULT_QUERY_TIMEOUT_SECS` / `DB_MAX_QUERY_TIMEOUT_SECS`（逐字收口到 `domain.rs`，此处仅再导出）。
- 多语句静态检出（进入驱动前第二道）。

## 2. 边界（Boundary / Non-Responsibility）

- **不含任何 `#[tauri::command]`**：db 命令（`db_query`/`db_connect`/`db_disconnect` 等）命令体在 `bridge.rs`（`bridge::db_query` 等），本模块只提供 `DbPool`/`QueryCancel` 基座。
- **不做 SQL 风险分类 / 生产判定**：落 `security_policy.rs`（零 db 依赖）。
- **不引入 `tokio` 直接依赖** / 不自建 async runtime / 禁 JDBC 侧车。
- **不伪造能力**：MySQL/Postgres 取数通道与 TLS **未实现** → 一律显式 `DB_NOT_SUPPORTED`，禁止静默失败 / 假装有能力。
- 禁止 DSN / 连接串（凭据只经 `KeyringStore` 键 `db:<conn_id>`）；SQLite 禁 `execute_batch`（走 `prepare`）。

## 3. Commands

**本模块无命令。** db 命令归属 database 能力，命令体注册在 `bridge.rs`（经 `generate_handler!`），命令体调用本模块 `DbPool`/`QueryCancel`。命令体在 **bridge.rs 逐 command 分解阶段**迁移至 `capabilities/database/commands.rs`。

## 4. Resources

- **sqlite**（bundled rusqlite）；凭据经 keyring 键 `db:<conn_id>`。
- 无 WebView / PTY / socket / 子进程 / 定时器 / watcher 在模块级；`DbPool` 由 bridge.rs 命令体按连接创建。
- 无 AppState 字段（本模块不 `.manage()` 任何状态；连接池随命令体生命周期）。

## 5. 生命周期（Lifecycle）

- 无模块级长生命周期资源；`DbPool` 由 `bridge.rs` 命令体按需创建 / 释放。
- 凭据取用：`KeyringStore::get_token(credential_key(conn_id))` → 用完即弃，不缓存明文。

## 6. 依赖（Dependencies）

- `crate::domain`：`DbConnectionConfig`/`DbErrorCode`/`DbSslMode`/`SupportedDb` + `DB_MAX_TEXT_FIELD_BYTES`（类型真源，SHARED）。
- `crate::security_policy`：`check_path_component`/`check_path_within_roots`（SHARED）。
- 外部 crate：`serde`（Serialize/Deserialize）；标准库 `std::path`/`std::sync::atomic`/`std::sync::Arc`/`std::time`。

## 7. 禁止依赖（Forbidden Dependencies）

- 禁止 `crate::bridge`（AppState 共享态枢纽 / 命令 hub）。
- 禁止 DSN / 连接串（凭据只经 `KeyringStore` 键 `db:<conn_id>`）。
- 禁止 `tokio` 直接依赖 / 自建 async runtime / JDBC 侧车。
- 禁止静默失败（必须 `DB_NOT_SUPPORTED`）；SQLite 禁 `execute_batch`。

## 8. Public / Native Contract

- 对外基座契约：`DbPool` / `QueryCancel` / `DbConnectionConfig` / `DbErrorCode` / `DbSslMode` / `SupportedDb` / `DbQueryResult` / `DbValue` / `credential_key` / `DB_CRED_PREFIX` / `sanitize_message` / `DB_MAX_*` / `DB_DEFAULT_QUERY_TIMEOUT_SECS` / `DB_MAX_QUERY_TIMEOUT_SECS`。
- 既有 `crate::database::` 调用点（5 处，全在 `bridge.rs`）经 `main.rs` 顶部 `pub use crate::capabilities::database::database;` re-export shim 解析，**未逐处改写**。
- TS 侧经 `bridge.ts` 命令调用，不直接 `import` 本 crate。

## 9. Security / ACL

- `DB_CRED_PREFIX = "db:"`：`db:<conn_id>` 是凭据唯一键前缀；禁止其它前缀（会与 git 的 `repo_id` 静默互串，守 `check-database-policy.py`）。
- `sanitize_message`：任何错误串出模块前脱敏（凭据 / 路径不泄露）。
- ACL：db 命令在 `permissions/default-commands.toml` 放行（`db_query`/`db_connect`/`db_disconnect` 等），与 `bridge.ts`/`src/types.ts` 镜像一致。
- 门禁真源：`scripts/check-database-policy.py`（**扫 `bridge.rs` 命令层**，不扫本文件——本文件仅在 `#[cfg(test)]` fixture 用 `execute_batch`，合法）。

## 10. Tests

- `database.rs` 内 `#[cfg(test)] mod`（line ~784 起：建表 fixture / 多语句检出 / 凭据脱敏 / 超时分层 / 错误码闭合）。
- 运行：`cd src-tauri && cargo test capabilities::database`（或 `cargo test` 全量）。

## 11. Source of Truth

- 运行时基座：`src-tauri/src/capabilities/database/database.rs`（本模块）。
- 类型 + 上限常量真源：`src-tauri/src/domain.rs`。
- 命令体（暂留）：`src-tauri/src/bridge.rs`（`db_query`/`db_connect`/`db_disconnect`）。
- 前端接线：`src/bridge.ts`（`dbQuery` 包装）+ `src/types.ts`（`DbQueryResult` 等）+ `workspace/DatabasePanel.vue`。
- 门禁真源：`scripts/check-database-policy.py`（扫 bridge.rs）。

## 12. Known Debt

- M4-1.c checkpoint 被 A0 标 `STOPPED_EMPTY_ARTIFACT`：常量值已采用并上报，待 A0 重新裁定状态（若裁定变更只需改本段常量，取数机制不变）。
- MySQL/Postgres 取数通道 + TLS **未实现**（现状 `DB_NOT_SUPPORTED`）；接取数属后续 wave（A3/A4 稳定后）。
- db 命令体仍在 `bridge.rs`（LEGACY_MIXED_MODULE 残核）；属矩阵 §8 末段 bridge 分解计划。

## 13. Extraction Readiness

- **高。** 纯运行时基座、零命令、零 AppState 字段、跨模块依赖仅 SHARED（`domain`/`security_policy`）+ 标准库。
- 达到阈值后可随 `database` 能力提级为 crate `mvp-database-rust`（不改领域语义）。
- 同能力下一个低风险同批候选：db 命令体 `commands.rs`（需先解除对 `bridge` hub 的耦合，见矩阵 §4 / 协议 §8）。
