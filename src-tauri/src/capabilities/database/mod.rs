//! Database 能力原生后端（数据库运行时基座，CAPABILITY_NATIVE(database)，M4-2 Lane A3）。
//!
//! 承载 `database.rs`：同步栈 `rusqlite`(bundled) + `mysql` + `postgres` stub 的
//! **运行时基础**——`DbPool` / `QueryCancel` / `credential_key` / `DB_CRED_PREFIX` /
//! `sanitize_message` / `DbErrorCode`（18 码闭合）/ 上限常量 / 超时分层 / 多语句
//! 静态检出。
//!
//! 命令层 `commands.rs`（Native Physical Boundary 分解迁入）：`#[tauri::command]`
//! `db_connect` / `db_list_connections` / `db_cancel` / `db_query` / `db_disconnect`，
//! 委托本模块 `DbPool`/`QueryCancel`/`credential_key`；安全闸门（`evaluate_db_query_gate`）
//! 与连接登记簿（`DbConnectionRegistry`）归命令层。不做 SQL 风险分类 / 生产判定
//! （落 `security_policy.rs`，零 db 依赖）。
//!
//! 三条硬红线（承 M4-1.a~d 冻结契约）：
//! 1. 多语句：SQLite 走 `prepare`（禁 `execute_batch`）+ 进入驱动前第二道静态检出；
//! 2. 凭据：本模块**不接受 DSN / 连接串**，凭据只经 `KeyringStore`（键 `db:<conn_id>`），
//!    错误串出模块前一律 `sanitize_message`；
//! 3. 不伪造：MySQL/Postgres 取数通道与 TLS 能力**未实现**，一律显式返回
//!    `DB_NOT_SUPPORTED` 并说明，不得假装。
//!
//! 迁移自 `src-tauri/src/database.rs`
//! （Native Physical Boundary Matrix Pilot 8，见
//! `docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §8.10）。
//! 既有 `crate::database::` 调用点（5 处，全在 bridge.rs：`DbPool`/`QueryCancel`/
//! `credential_key`/`DbQueryResult`）经 `main.rs` 顶部 re-export shim 解析，无需逐处改写。
//! 注：本模块**无**门禁脚本钉死旧路径（`check-database-policy.py` 扫 `bridge.rs` 命令层
//! 而非本文件），故本批**无需**改 checker 路径。
pub mod commands;
pub mod database;
