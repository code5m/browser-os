//! M4-2 数据库运行时基础（Lane A3）。
//!
//! 契约依据（全部冻结，勿凭记忆改动）：
//! - `logs/checkpoints/M4-1.a-20260905-2245.md` —— 同步栈 `rusqlite`(bundled) + `mysql` + `postgres`，
//!   **不引入 tokio 直接依赖**、不自建 async runtime、禁 JDBC 侧车。
//! - `logs/checkpoints/M4-1.b-20260905-2250.md` —— `SupportedDb` / `DbConnectionConfig` / `DbErrorCode`
//!   （闭合 18 码）；凭据键 `db:<conn_id>`；`DbConnectionConfig` 结构性不含凭据字段。
//! - `logs/checkpoints/M4-1.c-20260905-2255.md` —— 上限 / 取消三层 / 截断 / 超时分层。
//! - `logs/checkpoints/M4-1.d-20260905-2300.md` —— 生产判定信号 S1~S5、D27/D28/D29。
//!
//! 本模块**只做运行时基础**，不做命令层（`#[tauri::command]`，归 A4/M4-3），
//! 不做 SQL 风险分类与生产判定（归 A4/M4-2.s，落 `security_policy.rs`，零 db 依赖）。
//!
//! 三条硬红线（本模块逐条落实）：
//! 1. **多语句**：SQLite 走 `prepare`（禁 `execute_batch`），并在进入驱动前做第二道静态检出。
//! 2. **凭据**：本模块**不接受** DSN / 连接串；连接参数一律以类型化字段构造，
//!    凭据只经 `KeyringStore`（键 `db:<conn_id>`），错误串出模块前一律 `sanitize_message`。
//! 3. **不伪造**：MySQL/Postgres 的取数通道与 TLS 能力在本包内**未实现**，
//!    一律显式返回 `DB_NOT_SUPPORTED` 并说明原因，不得假装有能力。

use crate::domain::{DbConnectionConfig, DbErrorCode, DbSslMode, SupportedDb};
use crate::security_policy::{check_path_component, check_path_within_roots};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// 上限常量（M4-1.c §1 冻结值）
//
// ⚠️ 契约状态声明：`M4-1.c-20260905-2255.md` 正文（1-125 行）完整，
// 但 A0 因写入竞态把该文件标记为 `STOPPED_EMPTY_ARTIFACT`（126-149 行）。
// 本模块**采用正文数值**，并在 checkpoint 中上报，请 A0 重新裁定 M4-1.c 状态。
// 若裁定值变更，只需改本段常量，取数机制不变。
// ---------------------------------------------------------------------------

/// 单条 SQL 上限（复用 `security_policy::MAX_TEXT_FIELD_BYTES` 量级）。
pub const DB_MAX_SQL_BYTES: usize = 65_536;
/// 结果行数上限。**前端不可调大**（同 M1-8 `max_total` 口径）。
pub const DB_MAX_ROWS: usize = 1_000;
/// 结果字节上限（4 MiB）。
pub const DB_MAX_RESULT_BYTES: usize = 4_194_304;
// M5-1 切片 0b：常量收口到 `domain.rs`（值逐字不变），此处仅再导出。
pub use crate::domain::DB_MAX_TEXT_FIELD_BYTES;
/// 默认查询超时（秒）。
pub const DB_DEFAULT_QUERY_TIMEOUT_SECS: u64 = 30;
/// 查询超时上限（秒），超过按 `DB_INVALID_CONFIG` 拒绝。
pub const DB_MAX_QUERY_TIMEOUT_SECS: u64 = 600;
// M5-1 切片 0b：常量收口到 `domain.rs`（值逐字不变；注意 domain 为 `u32`，下方 `from_secs` 需 `as u64`）。此处仅再导出。
pub use crate::domain::DB_SOFT_TO_HARD_GRACE_SECS;
/// 每 N 行检查一次取消/超时标志（避免逐行原子读）。
pub const DB_CANCEL_CHECK_EVERY_ROWS: usize = 64;
/// 导出/落盘上限（16 MiB）。**尚无消费方**（A5 未确认是否做导出），仅占位常量。
#[allow(dead_code)] // consumer: A5 导出（占位，M4-2.b 待定）
pub const DB_MAX_EXPORT_BYTES: usize = 16_777_216;
/// 连接展示名 / id 长度上限（本包自设，用于配置校验的输入面收敛）。
pub const DB_MAX_NAME_BYTES: usize = 128;

// ---------------------------------------------------------------------------
// 凭据命名空间（M4-1.b §3 / A10 G-1）
// ---------------------------------------------------------------------------

/// Keyring 键前缀。与 git 侧 `repo_id` 命名空间隔离，防 `conn_id` / `repo_id` 互相覆盖。
pub const DB_CRED_PREFIX: &str = "db:";

/// 构造凭据键。格式**必须**为 `db:<conn_id>`（夹具码位 `DB_CRED_NAMESPACE` 守）。
pub fn credential_key(conn_id: &str) -> String {
    format!("{DB_CRED_PREFIX}{conn_id}")
}

// ---------------------------------------------------------------------------
// 错误类型
// ---------------------------------------------------------------------------

/// 数据库错误出口。
///
/// `message` **必须已脱敏**：构造前一律过 [`sanitize_message`]，
/// 不得夹带口令、DSN、连接串（A10 G-2/G-3）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbError {
    pub code: DbErrorCode,
    /// 已脱敏的人类可读说明（可进日志与审计 detail）。
    pub message: String,
    /// 该连接是否**不可复用**、必须丢弃而非归还池。
    /// 仅 hard 超时置 `true`（M4-1.c §4：soft 后 5s 仍未结束 → 弃连接）。
    pub discard_connection: bool,
}

impl DbError {
    pub fn new(code: DbErrorCode, message: impl Into<String>) -> Self {
        DbError {
            code,
            message: sanitize_message(&message.into()),
            discard_connection: false,
        }
    }

    /// 致命错误（连接必须丢弃）。
    pub fn fatal(code: DbErrorCode, message: impl Into<String>) -> Self {
        DbError {
            code,
            message: sanitize_message(&message.into()),
            discard_connection: true,
        }
    }

    /// 稳定错误码串（前端与夹具按此分支）。
    pub fn code_str(&self) -> &'static str {
        self.code.as_str()
    }
}

// ---------------------------------------------------------------------------
// 结果 DTO（M4-1.c §2，字段名为准）
// ---------------------------------------------------------------------------

/// 可序列化标量。**BLOB 一律不回传字节**（只回长度），
/// 避免二进制经 JSON 打到前端（M4-1.c §2）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DbValue {
    Null,
    Bool(bool),
    I64(i64),
    F64(f64),
    Text(String),
    /// 二进制列：只携带长度，值为固定占位 `<binary>`。
    Binary {
        bytes: usize,
    },
}

impl DbValue {
    /// 计费字节数（用于 `DB_MAX_RESULT_BYTES` 累积）。
    pub fn byte_len(&self) -> usize {
        match self {
            DbValue::Null => 4,
            DbValue::Bool(_) => 1,
            DbValue::I64(_) => 8,
            DbValue::F64(_) => 8,
            DbValue::Text(s) => s.len(),
            DbValue::Binary { bytes } => *bytes,
        }
    }
}

/// 查询结果。**字段名与语义以 M4-1.c §2 为准**，A4 的 TS 镜像须逐字对齐。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DbQueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<DbValue>>,
    /// 实际返回行数（≤ `DB_MAX_ROWS`）。
    pub row_count: usize,
    /// 行/字节**任一**超限即 `true`。超限**不是错误**，但禁止静默截断。
    pub truncated: bool,
    /// 任一单字段被 `DB_MAX_TEXT_FIELD_BYTES` 截断即 `true`。
    pub field_truncated: bool,
    pub elapsed_ms: u64,
    /// 取消句柄标识（供 `db_cancel` 定位，命令名由 A4/M4-3.a 冻结）。
    pub query_id: String,
}

// ---------------------------------------------------------------------------
// 取消（M4-1.c §3，L1 标志层）
// ---------------------------------------------------------------------------

/// 取消标志（L1）。跨线程共享，取数循环每 `DB_CANCEL_CHECK_EVERY_ROWS` 行检查一次（L2），
/// 命中后尽力发起驱动级带外中断（L3）。
#[derive(Debug, Clone, Default)]
pub struct QueryCancel {
    flag: Arc<AtomicBool>,
}

impl QueryCancel {
    pub fn new() -> Self {
        QueryCancel {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 发起取消（L1 标志层）。
    ///
    /// **当前无生产消费方**：`db_cancel` 命令名未冻结（M4-1.c §3），A4/M4-3 的
    /// `db_query` 每次新建 `QueryCancel` 且无外部触发路径，故非测试构建里它不可达。
    /// 它是 `db_cancel` 落地的**唯一入口**，不得删除；命令名冻结后移除本 allow。
    #[allow(dead_code)] // consumer: A4/M4-3 `db_cancel`（命令名待 M4-1.c §3 冻结）
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

/// 超时分层（M4-1.c §4）。**机制与 M2-4 不同**：无进程可杀，
/// soft = 带外取消，hard = 弃连接。
pub(crate) struct QueryDeadline {
    start: Instant,
    soft: Duration,
}

impl QueryDeadline {
    pub(crate) fn new(soft: Duration) -> Self {
        QueryDeadline {
            start: Instant::now(),
            soft,
        }
    }

    fn soft_reached(&self) -> bool {
        self.start.elapsed() >= self.soft
    }

    fn hard_reached(&self) -> bool {
        self.start.elapsed() >= self.soft + Duration::from_secs(DB_SOFT_TO_HARD_GRACE_SECS as u64)
    }
}

// ---------------------------------------------------------------------------
// 脱敏
// ---------------------------------------------------------------------------

/// 错误串脱敏：截断到 512 字符，并把 `password=xxx` / `token:xxx` 形态整体替换为 `***`。
///
/// 驱动错误可能把连接参数回显出来（A10 G-2），因此**任何**出模块的错误串都必须过此函数。
/// 实现刻意避开字节切片（只在 `Vec<char>` 上按字符推进），杜绝 UTF-8 边界 panic
/// ——本 crate 的 release profile 是 `panic = "abort"`，一次 panic 即杀进程。
pub fn sanitize_message(raw: &str) -> String {
    const MAX_CHARS: usize = 512;
    let chars: Vec<char> = raw.chars().take(MAX_CHARS).collect();
    let keys: [&str; 5] = ["password", "passwd", "pwd", "secret", "token"];
    let mut out = String::new();
    let mut token = String::new();
    let mut idx = 0usize;

    while idx <= chars.len() {
        let current = chars.get(idx).copied();
        let at_end = current.is_none();
        let ch = current.unwrap_or(' ');
        if at_end || is_token_separator(ch) {
            if !token.is_empty() {
                out.push_str(&redact_token(&token, &keys));
                token.clear();
            }
            if at_end {
                break;
            }
            out.push(ch);
        } else {
            token.push(ch);
        }
        idx += 1;
    }
    out
}

fn is_token_separator(c: char) -> bool {
    matches!(c, ' ' | ',' | ';' | '\n' | '\r' | '\t' | '&' | '"' | '\'')
}

fn redact_token(token: &str, keys: &[&str]) -> String {
    let lower = token.to_ascii_lowercase();
    for key in keys {
        if let Some(after) = lower.strip_prefix(key) {
            if after.starts_with('=') || after.starts_with(':') {
                return format!("{key}=***");
            }
        }
    }
    token.to_string()
}

// ---------------------------------------------------------------------------
// 配置校验（M4-1.b §2.2 / M4-1.d 交接矩阵）
// ---------------------------------------------------------------------------

/// 连接配置校验。**不涉及**凭据（凭据不在此结构上，见模块级红线 2）。
pub fn validate_config(cfg: &DbConnectionConfig) -> Result<(), DbError> {
    if cfg.id.trim().is_empty() {
        return Err(DbError::new(DbErrorCode::InvalidConfig, "连接 id 不可为空"));
    }
    if cfg.id.len() > DB_MAX_NAME_BYTES {
        return Err(DbError::new(
            DbErrorCode::InvalidConfig,
            format!("连接 id 超过 {} 字节上限", DB_MAX_NAME_BYTES),
        ));
    }
    if cfg.name.trim().is_empty() {
        return Err(DbError::new(DbErrorCode::InvalidConfig, "连接名不可为空"));
    }
    if cfg.name.len() > DB_MAX_NAME_BYTES {
        return Err(DbError::new(
            DbErrorCode::InvalidConfig,
            format!("连接名超过 {} 字节上限", DB_MAX_NAME_BYTES),
        ));
    }
    // 空库名同时是「生产判定 → Unknown」的信号之一（M4-1.d S5）；
    // 但配置层先按非法拒绝，避免把空串一路带到驱动层。
    if cfg.database.trim().is_empty() {
        return Err(DbError::new(
            DbErrorCode::InvalidConfig,
            "数据库名/文件路径不可为空",
        ));
    }
    if cfg.kind.is_remote() {
        let host = cfg.host.as_deref().unwrap_or("").trim();
        if host.is_empty() {
            return Err(DbError::new(
                DbErrorCode::InvalidConfig,
                "网络库必须提供 host",
            ));
        }
        let username = cfg.username.as_deref().unwrap_or("").trim();
        if username.is_empty() {
            return Err(DbError::new(
                DbErrorCode::InvalidConfig,
                "网络库必须提供 username",
            ));
        }
        if cfg.port == Some(0) {
            return Err(DbError::new(DbErrorCode::InvalidConfig, "端口号不可为 0"));
        }
    }
    Ok(())
}

/// SQLite 文件路径校验：**父目录**必须落在允许根内，文件名须过 `check_path_component`。
///
/// 直接对文件本身调用 `check_path_within_roots` 不可行——该函数内部 `canonicalize`
/// 要求路径已存在，而新建的 SQLite 库文件此时还不存在。
pub fn validate_sqlite_path(database: &str, roots: &[PathBuf]) -> Result<PathBuf, DbError> {
    let raw = Path::new(database.trim());
    if database.trim().is_empty() {
        return Err(DbError::new(
            DbErrorCode::InvalidConfig,
            "SQLite 文件路径不可为空",
        ));
    }
    let file_name = raw
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| DbError::new(DbErrorCode::InvalidConfig, "SQLite 文件路径缺少文件名"))?;

    check_path_component(file_name).map_err(|e| {
        DbError::new(
            DbErrorCode::PathOutsideRoots,
            format!("SQLite 文件名非法: {e}"),
        )
    })?;

    let parent = raw.parent().filter(|p| !p.as_os_str().is_empty());
    let dir = match parent {
        Some(p) => p,
        None => Path::new("."),
    };
    let canonical_dir = check_path_within_roots(&dir.to_string_lossy(), roots).map_err(|e| {
        DbError::new(
            DbErrorCode::PathOutsideRoots,
            format!("SQLite 文件目录不在允许根内: {e}"),
        )
    })?;
    Ok(canonical_dir.join(file_name))
}

/// 超时归一：`None` / `0` → 默认 30s；超过 600s → `DB_INVALID_CONFIG`
/// （属配置校验，**不是** `DB_TIMEOUT`）。
pub fn resolve_timeout_secs(requested: Option<u64>) -> Result<u64, DbError> {
    match requested {
        None | Some(0) => Ok(DB_DEFAULT_QUERY_TIMEOUT_SECS),
        Some(v) if v > DB_MAX_QUERY_TIMEOUT_SECS => Err(DbError::new(
            DbErrorCode::InvalidConfig,
            format!("查询超时 {}s 超过上限 {}s", v, DB_MAX_QUERY_TIMEOUT_SECS),
        )),
        Some(v) => Ok(v),
    }
}

/// 多语句**第二道**静态检出。
///
/// 第一道是 A4 的分类器（剥离注释与字符串字面量后整批处理）；本函数只是在进入驱动前的
/// 兜底，**不处理**分号位于字符串字面量内的情况——那属于分类器的职责（M4-1.c §5）。
pub fn detect_multiple_statements(sql: &str) -> bool {
    let trimmed = sql.trim_end();
    let body = trimmed
        .strip_suffix(';')
        .map(|s| s.trim_end())
        .unwrap_or(trimmed);
    body.contains(';')
}

// ---------------------------------------------------------------------------
// 连接池持有者（M4-1.a §5 第 2 处：新增驱动必须在此加变体，编译期穷尽）
// ---------------------------------------------------------------------------

/// 单连接持有者。本包交付的是**可稳定调用的边界**，不是成熟的池管理器；
/// 池上限/复用策略在 A4 命令层落地后按其需要扩展。
///
/// **释放语义 = 直接 `drop`**：三个驱动句柄（`rusqlite::Connection` / `mysql::Conn` /
/// `postgres::Client`）均在 `Drop` 时关闭连接，本模块**不提供**显式 `close`
/// ——曾有一个空实现的 `close(self)`，零行为且零消费方（A4 的 `db_disconnect`
/// 走 drop 语义），已删除以免留下「以为关了其实没关」的假象。
#[allow(dead_code)] // consumer: A4/M4-3 命令层与 M4-2.b 取数通道
pub enum DbPool {
    Sqlite(rusqlite::Connection),
    MySql(mysql::Conn),
    Postgres(postgres::Client),
}

/// 手写 `Debug`：只打印驱动种类，**不泄露**连接句柄、主机、凭据
/// （`.expect_err` / `.unwrap_err` 在 panic 时会格式化 `T: Debug`）。
impl std::fmt::Debug for DbPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DbPool")
            .field(&self.kind().as_str())
            .finish()
    }
}

impl DbPool {
    /// 建立连接。**凭据以独立参数传入**，不出现在 `DbConnectionConfig` 上，也不拼接 DSN。
    pub fn connect(
        cfg: &DbConnectionConfig,
        password: Option<&str>,
        roots: &[PathBuf],
    ) -> Result<Self, DbError> {
        validate_config(cfg)?;

        // D29：非 Linux 未验证，按契约**显式降级**而非静默失败。
        if !cfg!(target_os = "linux") {
            return Err(DbError::new(
                DbErrorCode::NotSupported,
                "M4-2 数据库能力当前仅在 Linux 验证过，其他平台显式降级为 DB_NOT_SUPPORTED（D29）",
            ));
        }

        match cfg.kind {
            SupportedDb::Sqlite => {
                let path = validate_sqlite_path(&cfg.database, roots)?;
                let conn = rusqlite::Connection::open(&path).map_err(|e| {
                    DbError::new(
                        DbErrorCode::ConnectFailed,
                        format!("打开 SQLite 失败: {}", sanitize_message(&e.to_string())),
                    )
                })?;
                Ok(DbPool::Sqlite(conn))
            }
            SupportedDb::MySql => connect_mysql(cfg, password),
            SupportedDb::Postgres => connect_postgres(cfg, password),
        }
    }

    pub fn kind(&self) -> SupportedDb {
        match self {
            DbPool::Sqlite(_) => SupportedDb::Sqlite,
            DbPool::MySql(_) => SupportedDb::MySql,
            DbPool::Postgres(_) => SupportedDb::Postgres,
        }
    }

    /// 实际传输层加密状态（M4-1.d 信号 S4/S6 的输入）。
    ///
    /// - `None` = 不适用或未知。**未知按生产处理**（fail-closed），不得当成未加密放行。
    /// - SQLite 为本地文件、无传输层 → `None`。
    /// - 本包 TLS 未接入（见 [`connect_postgres`]），MySQL/Postgres 一律 `None`。
    #[allow(dead_code)] // consumer: A4/M4-2.s 生产判定（M4-1.d S4/S6）
    pub fn encrypted(&self) -> Option<bool> {
        None
    }

    /// 执行查询并构造受限结果集。
    ///
    /// **写语句不在此处判定**：写闸门全链路（生产判定 + `allow_write` + 二次确认 + 分类器）
    /// 归 A4/M4-2.s 与 M4-3，本函数只负责取数通道。
    pub fn query(
        &mut self,
        sql: &str,
        cancel: &QueryCancel,
        timeout_secs: Option<u64>,
        query_id: &str,
    ) -> Result<DbQueryResult, DbError> {
        let secs = resolve_timeout_secs(timeout_secs)?;
        let deadline = QueryDeadline::new(Duration::from_secs(secs));
        match self {
            DbPool::Sqlite(conn) => {
                query_sqlite_with_deadline(conn, sql, cancel, &deadline, query_id)
            }
            // 不伪造：MySQL/Postgres 取数通道尚未实现（D27：无真实服务端可验证）。
            DbPool::MySql(_) => Err(DbError::new(
                DbErrorCode::NotSupported,
                "MySQL 取数通道未实现（M4-2.b 交付，受 D27 无真实服务端约束）",
            )),
            DbPool::Postgres(_) => Err(DbError::new(
                DbErrorCode::NotSupported,
                "PostgreSQL 取数通道未实现（M4-2.b 交付，受 D27 无真实服务端约束）",
            )),
        }
    }
}

fn connect_mysql(cfg: &DbConnectionConfig, password: Option<&str>) -> Result<DbPool, DbError> {
    // TLS：本包未建模 SslOpts（CA / 客户端证书），故 `Require` 显式降级而非假装支持。
    if matches!(cfg.ssl_mode, DbSslMode::Require) {
        return Err(DbError::new(
            DbErrorCode::NotSupported,
            "MySQL ssl_mode=require 需要 SslOpts（CA/客户端证书），M4-2.a 未建模，显式降级",
        ));
    }
    let host = cfg.host.clone().unwrap_or_default();
    let port = cfg
        .port
        .unwrap_or_else(|| cfg.kind.default_port().unwrap_or(3306));
    let user = cfg.username.clone().unwrap_or_default();

    // 类型化构造：不拼 DSN。多语句由驱动默认关闭，不依赖 crate 默认值另行开启。
    let builder = mysql::OptsBuilder::new()
        .ip_or_hostname(Some(host))
        .tcp_port(port)
        .user(Some(user))
        .pass(password.map(|p| p.to_string()))
        .db_name(Some(cfg.database.clone()));

    let conn = mysql::Conn::new(builder).map_err(|e| {
        DbError::new(
            DbErrorCode::ConnectFailed,
            format!("连接 MySQL 失败: {}", sanitize_message(&e.to_string())),
        )
    })?;
    Ok(DbPool::MySql(conn))
}

fn connect_postgres(cfg: &DbConnectionConfig, password: Option<&str>) -> Result<DbPool, DbError> {
    // TLS：本包未启用 `with-native-tls` feature，只有 `NoTls` 可用；
    // `Require` 若放行即为「假装加密」，故显式降级（D29 同口径）。
    if matches!(cfg.ssl_mode, DbSslMode::Require) {
        return Err(DbError::new(
            DbErrorCode::NotSupported,
            "PostgreSQL ssl_mode=require 需要启用 with-native-tls，M4-2.a 未启用，显式降级",
        ));
    }
    let host = cfg.host.clone().unwrap_or_default();
    let port = cfg
        .port
        .unwrap_or_else(|| cfg.kind.default_port().unwrap_or(5432));
    let user = cfg.username.clone().unwrap_or_default();

    let mut config = postgres::Config::new();
    config
        .host(&host)
        .port(port)
        .user(&user)
        .dbname(&cfg.database)
        .ssl_mode(match cfg.ssl_mode {
            DbSslMode::Disable => postgres::config::SslMode::Disable,
            DbSslMode::Prefer => postgres::config::SslMode::Prefer,
            DbSslMode::Require => postgres::config::SslMode::Require,
        });
    if let Some(p) = password {
        config.password(p);
    }

    let client = config.connect(postgres::tls::NoTls).map_err(|e| {
        DbError::new(
            DbErrorCode::ConnectFailed,
            format!("连接 PostgreSQL 失败: {}", sanitize_message(&e.to_string())),
        )
    })?;
    Ok(DbPool::Postgres(client))
}

// ---------------------------------------------------------------------------
// 取数核心（SQLite）
// ---------------------------------------------------------------------------

/// 累积器：在**取数循环内**计数，任一上限命中即停止并丢弃剩余
/// （A10 G-6：物化后再截断等于没截断）。
struct QueryLimiter {
    rows: Vec<Vec<DbValue>>,
    bytes: usize,
    truncated: bool,
    field_truncated: bool,
}

impl QueryLimiter {
    fn new() -> Self {
        QueryLimiter {
            rows: Vec::new(),
            bytes: 0,
            truncated: false,
            field_truncated: false,
        }
    }

    /// 单字段截断（保留前 `DB_MAX_TEXT_FIELD_BYTES` 字节，按字符边界安全切分）。
    fn normalize_field(&mut self, value: DbValue) -> DbValue {
        match value {
            DbValue::Text(s) if s.len() > DB_MAX_TEXT_FIELD_BYTES => {
                self.field_truncated = true;
                let mut kept = String::new();
                let mut used = 0usize;
                for ch in s.chars() {
                    let len = ch.len_utf8();
                    if used + len > DB_MAX_TEXT_FIELD_BYTES {
                        break;
                    }
                    kept.push(ch);
                    used += len;
                }
                DbValue::Text(kept)
            }
            other => other,
        }
    }

    /// 压入一行。返回 `false` 表示已触顶，调用方应**立即停止取数**。
    fn push(&mut self, row: Vec<DbValue>) -> bool {
        let row_bytes: usize = row.iter().map(|v| v.byte_len()).sum();
        if self.rows.len() >= DB_MAX_ROWS || self.bytes + row_bytes > DB_MAX_RESULT_BYTES {
            self.truncated = true;
            return false;
        }
        self.bytes += row_bytes;
        self.rows.push(row);
        true
    }
}

fn from_sqlite_value(value: rusqlite::types::Value) -> DbValue {
    match value {
        rusqlite::types::Value::Null => DbValue::Null,
        rusqlite::types::Value::Integer(i) => DbValue::I64(i),
        rusqlite::types::Value::Real(f) => DbValue::F64(f),
        rusqlite::types::Value::Text(s) => DbValue::Text(s),
        rusqlite::types::Value::Blob(b) => DbValue::Binary { bytes: b.len() },
    }
}

/// SQLite 取数（含上限 / 截断 / 取消 / 分层超时）。
///
/// `deadline` 可注入，便于测试**不改生产常量**地验证超时边界（M4-1.c §6）。
pub(crate) fn query_sqlite_with_deadline(
    conn: &rusqlite::Connection,
    sql: &str,
    cancel: &QueryCancel,
    deadline: &QueryDeadline,
    query_id: &str,
) -> Result<DbQueryResult, DbError> {
    let started = Instant::now();

    if sql.trim().is_empty() {
        return Err(DbError::new(DbErrorCode::SqlEmpty, "SQL 为空"));
    }
    if sql.len() > DB_MAX_SQL_BYTES {
        return Err(DbError::new(
            DbErrorCode::SqlTooLarge,
            format!("SQL 超过 {} 字节上限", DB_MAX_SQL_BYTES),
        ));
    }
    if detect_multiple_statements(sql) {
        return Err(DbError::new(
            DbErrorCode::MultipleStatements,
            "检出多语句：单条查询只允许一个语句",
        ));
    }

    // L3 驱动级带外中断句柄：取消或超时时尽力而为，失败不阻塞返回。
    let interrupt = conn.get_interrupt_handle();

    let mut stmt = conn.prepare(sql).map_err(|e| {
        DbError::new(
            DbErrorCode::QueryFailed,
            format!("SQL 预处理失败: {}", sanitize_message(&e.to_string())),
        )
    })?;

    let column_count = stmt.column_count();
    let mut columns = Vec::with_capacity(column_count);
    for i in 0..column_count {
        // `column_name` 只在索引越界时报错，此处索引来自 `column_count`，用 unwrap_or 兜底。
        columns.push(stmt.column_name(i).unwrap_or("").to_string());
    }

    let mut limiter = QueryLimiter::new();
    let mut rows = stmt.query([]).map_err(|e| {
        DbError::new(
            DbErrorCode::QueryFailed,
            format!("查询执行失败: {}", sanitize_message(&e.to_string())),
        )
    })?;

    let mut index: usize = 0usize;
    loop {
        // L2：每 64 行检查一次取消与超时，避免逐行原子读。
        if index % DB_CANCEL_CHECK_EVERY_ROWS == 0 {
            if cancel.is_cancelled() {
                interrupt.interrupt();
                return Err(DbError::new(DbErrorCode::Cancelled, "查询已取消"));
            }
            if deadline.hard_reached() {
                interrupt.interrupt();
                return Err(DbError::fatal(
                    DbErrorCode::Timeout,
                    "查询 hard 超时：该连接必须丢弃而非归还池",
                ));
            }
            if deadline.soft_reached() {
                interrupt.interrupt();
                return Err(DbError::new(DbErrorCode::Timeout, "查询 soft 超时"));
            }
        }

        let next = rows.next().map_err(|e| {
            DbError::new(
                DbErrorCode::QueryFailed,
                format!("读取行失败: {}", sanitize_message(&e.to_string())),
            )
        })?;
        let row = match next {
            Some(r) => r,
            None => break,
        };

        let mut values = Vec::with_capacity(column_count);
        for i in 0..column_count {
            let raw = row.get::<_, rusqlite::types::Value>(i).map_err(|e| {
                DbError::new(
                    DbErrorCode::QueryFailed,
                    format!("读取列失败: {}", sanitize_message(&e.to_string())),
                )
            })?;
            values.push(limiter.normalize_field(from_sqlite_value(raw)));
        }

        if !limiter.push(values) {
            break;
        }
        index += 1;
    }

    let row_count = limiter.rows.len();
    Ok(DbQueryResult {
        columns,
        rows: limiter.rows,
        row_count,
        truncated: limiter.truncated,
        field_truncated: limiter.field_truncated,
        elapsed_ms: started.elapsed().as_millis() as u64,
        query_id: query_id.to_string(),
    })
}

// ---------------------------------------------------------------------------
// 测试（ID 段 T-db-p1~，对应 A1 §5 给 M4-2 的池与取数矩阵）
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::DbSslMode;
    use std::fs;
    use std::sync::atomic::Ordering as AtomicOrdering;

    fn temp_root(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("mvp-db-test-{tag}-{nanos}"));
        fs::create_dir_all(&dir).expect("创建临时根目录");
        dir
    }

    fn sqlite_config(id: &str, database: &str) -> DbConnectionConfig {
        let now = chrono::Utc::now();
        DbConnectionConfig {
            id: id.to_string(),
            name: "test".to_string(),
            kind: SupportedDb::Sqlite,
            host: None,
            port: None,
            database: database.to_string(),
            username: None,
            ssl_mode: DbSslMode::Prefer,
            allow_write: false,
            production_hint: None,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    fn remote_config(kind: SupportedDb) -> DbConnectionConfig {
        let now = chrono::Utc::now();
        DbConnectionConfig {
            id: "r-1".to_string(),
            name: "remote".to_string(),
            kind,
            host: Some("127.0.0.1".to_string()),
            port: None,
            database: "appdb".to_string(),
            username: Some("app".to_string()),
            ssl_mode: DbSslMode::Prefer,
            allow_write: false,
            production_hint: None,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    // -- T-db-p1 凭据命名空间 --

    #[test]
    fn t_db_p1_credential_key_uses_db_namespace() {
        assert_eq!(credential_key("abc"), "db:abc");
        assert_eq!(DB_CRED_PREFIX, "db:");
        // 与 git 侧 repo_id 命名空间隔离：git 键不带前缀，二者不会互相覆盖。
        assert_ne!(credential_key("repo-1"), "repo-1");
        assert!(credential_key("").starts_with("db:"));
    }

    // -- T-db-p2 配置校验 --

    #[test]
    fn t_db_p2_validate_config_rejects_empty_fields() {
        let mut cfg = sqlite_config("id-1", "/tmp/x.db");
        cfg.id = "  ".to_string();
        assert_eq!(
            validate_config(&cfg).unwrap_err().code,
            DbErrorCode::InvalidConfig
        );

        let mut cfg = sqlite_config("id-1", "/tmp/x.db");
        cfg.name = "".to_string();
        assert_eq!(
            validate_config(&cfg).unwrap_err().code,
            DbErrorCode::InvalidConfig
        );

        let cfg = sqlite_config("id-1", "   ");
        assert_eq!(
            validate_config(&cfg).unwrap_err().code,
            DbErrorCode::InvalidConfig
        );
    }

    #[test]
    fn t_db_p2_validate_config_requires_host_and_user_for_remote() {
        let mut cfg = remote_config(SupportedDb::MySql);
        cfg.host = None;
        assert_eq!(
            validate_config(&cfg).unwrap_err().code,
            DbErrorCode::InvalidConfig
        );

        let mut cfg = remote_config(SupportedDb::Postgres);
        cfg.username = Some("  ".to_string());
        assert_eq!(
            validate_config(&cfg).unwrap_err().code,
            DbErrorCode::InvalidConfig
        );

        let mut cfg = remote_config(SupportedDb::MySql);
        cfg.port = Some(0);
        assert_eq!(
            validate_config(&cfg).unwrap_err().code,
            DbErrorCode::InvalidConfig
        );

        assert!(validate_config(&remote_config(SupportedDb::MySql)).is_ok());
    }

    #[test]
    fn t_db_p2_validate_config_accepts_minimal_sqlite() {
        assert!(validate_config(&sqlite_config("id-1", "/tmp/ok.db")).is_ok());
    }

    // -- T-db-p3 SQLite 路径校验 --

    #[test]
    fn t_db_p3_sqlite_path_inside_roots_is_accepted() {
        let root = temp_root("inside");
        let path = root.join("a.db");
        let got = validate_sqlite_path(&path.to_string_lossy(), &[root.clone()]);
        assert!(got.is_ok(), "合法路径应通过: {got:?}");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn t_db_p3_sqlite_path_outside_roots_is_rejected() {
        let root = temp_root("outside");
        let err = validate_sqlite_path("/etc/passwd", &[root.clone()]).unwrap_err();
        assert_eq!(err.code, DbErrorCode::PathOutsideRoots);
        // 脱敏与稳定码：错误串不得把根目录路径拼进不受控位置之外的地方。
        assert_eq!(err.code_str(), "DB_PATH_OUTSIDE_ROOTS");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn t_db_p3_sqlite_path_traversal_component_is_rejected() {
        let root = temp_root("traversal");
        let err = validate_sqlite_path(
            &root.join("../../evil.db").to_string_lossy(),
            &[root.clone()],
        )
        .unwrap_err();
        assert_eq!(err.code, DbErrorCode::PathOutsideRoots);
        fs::remove_dir_all(&root).ok();
    }

    // -- T-db-p4 超时归一 --

    #[test]
    fn t_db_p4_resolve_timeout_defaults_and_bounds() {
        assert_eq!(resolve_timeout_secs(None).unwrap(), 30);
        assert_eq!(resolve_timeout_secs(Some(0)).unwrap(), 30);
        assert_eq!(resolve_timeout_secs(Some(600)).unwrap(), 600);
        let err = resolve_timeout_secs(Some(601)).unwrap_err();
        assert_eq!(err.code, DbErrorCode::InvalidConfig);
        assert_eq!(err.code_str(), "DB_INVALID_CONFIG");
    }

    // -- T-db-p5 多语句第二道检出 --

    #[test]
    fn t_db_p5_detect_multiple_statements() {
        assert!(detect_multiple_statements("select 1; select 2"));
        assert!(detect_multiple_statements("select 1; drop table t"));
        assert!(!detect_multiple_statements("select 1"));
        assert!(!detect_multiple_statements("select 1;"));
        assert!(!detect_multiple_statements("select 1 ;  "));
    }

    // -- T-db-p6 脱敏 --

    #[test]
    fn t_db_p6_sanitize_message_redacts_credentials() {
        let raw = "connect failed: password=hunter2 host=db.internal";
        let out = sanitize_message(raw);
        assert!(!out.contains("hunter2"), "口令不得残留: {out}");
        assert!(out.contains("password=***"));

        let raw = "bad token:abcdef";
        assert!(sanitize_message(raw).contains("token=***"));

        // 长串截断，且不得 panic（panic=abort 下一次即杀进程）
        let long = "x".repeat(5_000);
        assert!(sanitize_message(&long).chars().count() <= 512);
        // 多字节字符不得被切坏
        assert!(sanitize_message(&"中文口令 password=中文值 end").contains("password=***"));
    }

    // -- T-db-p7 SQLite 正常路径（真实库） --

    #[test]
    fn t_db_p7_sqlite_happy_path_returns_rows() {
        let root = temp_root("happy");
        let db = root.join("happy.db");
        let cfg = sqlite_config("id-1", &db.to_string_lossy());
        let mut pool = DbPool::connect(&cfg, None, &[root.clone()]).expect("连接 SQLite");
        assert_eq!(pool.kind(), SupportedDb::Sqlite);

        // 建表与灌数直接用连接，不走受限查询通道（写入语义归 M4-2.s 分类器与写闸门）。
        if let DbPool::Sqlite(conn) = &pool {
            conn.execute_batch(
                "CREATE TABLE t (id INTEGER, name TEXT);
                 INSERT INTO t (id, name) VALUES (1, 'a'), (2, 'b');",
            )
            .expect("建表");
        }

        let cancel = QueryCancel::new();
        let result = pool
            .query("SELECT id, name FROM t ORDER BY id", &cancel, None, "q-1")
            .expect("查询");
        assert_eq!(result.columns, vec!["id".to_string(), "name".to_string()]);
        assert_eq!(result.row_count, 2);
        assert!(!result.truncated);
        assert!(!result.field_truncated);
        assert_eq!(result.query_id, "q-1");
        assert_eq!(result.rows[0][0], DbValue::I64(1));
        assert_eq!(result.rows[1][1], DbValue::Text("b".to_string()));

        // 释放：本模块无显式 close，drop 即关闭驱动连接（见 `DbPool` 文档）。
        drop(pool);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn t_db_p7_sqlite_blob_is_not_returned_as_bytes() {
        let root = temp_root("blob");
        let db = root.join("blob.db");
        let cfg = sqlite_config("id-1", &db.to_string_lossy());
        let mut pool = DbPool::connect(&cfg, None, &[root.clone()]).expect("连接");
        if let DbPool::Sqlite(conn) = &pool {
            conn.execute_batch("CREATE TABLE b (data BLOB); INSERT INTO b VALUES (x'0102030405');")
                .expect("建表");
        }
        let cancel = QueryCancel::new();
        let result = pool
            .query("SELECT data FROM b", &cancel, None, "q-blob")
            .expect("查询");
        assert_eq!(result.rows[0][0], DbValue::Binary { bytes: 5 });
        fs::remove_dir_all(&root).ok();
    }

    // -- T-db-p8 行数上限与截断 --

    #[test]
    fn t_db_p8_row_limit_truncates_and_marks() {
        let root = temp_root("rowlimit");
        let db = root.join("rows.db");
        let cfg = sqlite_config("id-1", &db.to_string_lossy());
        let mut pool = DbPool::connect(&cfg, None, &[root.clone()]).expect("连接");
        if let DbPool::Sqlite(conn) = &pool {
            conn.execute_batch(
                "CREATE TABLE t (v INTEGER);
                 WITH RECURSIVE seq(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM seq WHERE n < 1500)
                 INSERT INTO t SELECT n FROM seq;",
            )
            .expect("灌数据");
        }
        let cancel = QueryCancel::new();
        let result = pool
            .query("SELECT v FROM t", &cancel, None, "q-rows")
            .expect("查询");
        assert_eq!(result.row_count, DB_MAX_ROWS, "行数必须被 1000 上限截断");
        assert!(result.truncated, "截断必须显式标记，禁止静默");
        assert!(!result.field_truncated);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn t_db_p8_field_truncation_marks_field_truncated() {
        let root = temp_root("fieldtrunc");
        let db = root.join("f.db");
        let cfg = sqlite_config("id-1", &db.to_string_lossy());
        let mut pool = DbPool::connect(&cfg, None, &[root.clone()]).expect("连接");
        if let DbPool::Sqlite(conn) = &pool {
            conn.execute_batch("CREATE TABLE t (v TEXT);")
                .expect("建表");
            let long = "x".repeat(DB_MAX_TEXT_FIELD_BYTES + 500);
            conn.execute("INSERT INTO t VALUES (?1)", [&long])
                .expect("插入");
        }
        let cancel = QueryCancel::new();
        let result = pool
            .query("SELECT v FROM t", &cancel, None, "q-field")
            .expect("查询");
        assert!(result.field_truncated);
        if let DbValue::Text(s) = &result.rows[0][0] {
            assert!(s.len() <= DB_MAX_TEXT_FIELD_BYTES);
        } else {
            panic!("期望 Text 值");
        }
        fs::remove_dir_all(&root).ok();
    }

    // -- T-db-p9 取消 --

    #[test]
    fn t_db_p9_cancelled_query_returns_cancelled_and_connection_stays_usable() {
        let root = temp_root("cancel");
        let db = root.join("c.db");
        let cfg = sqlite_config("id-1", &db.to_string_lossy());
        let mut pool = DbPool::connect(&cfg, None, &[root.clone()]).expect("连接");
        if let DbPool::Sqlite(conn) = &pool {
            conn.execute_batch("CREATE TABLE t (v INTEGER); INSERT INTO t VALUES (1);")
                .expect("建表");
        }

        let cancel = QueryCancel::new();
        cancel.cancel();
        let err = pool
            .query("SELECT v FROM t", &cancel, None, "q-cancel")
            .expect_err("已取消应返回错误");
        assert_eq!(err.code, DbErrorCode::Cancelled);
        assert_eq!(err.code_str(), "DB_CANCELLED");
        assert!(!err.discard_connection, "用户取消不要求弃连接");

        // A10 G-6：取消后连接必须归还池中并保持可用。
        let fresh = QueryCancel::new();
        let result = pool
            .query("SELECT v FROM t", &fresh, None, "q-after")
            .expect("连接仍可用");
        assert_eq!(result.row_count, 1);
        fs::remove_dir_all(&root).ok();
    }

    // -- T-db-p10 分层超时（注入 deadline，不改生产常量） --

    #[test]
    fn t_db_p10_soft_timeout_returns_timeout_without_discard() {
        let root = temp_root("soft");
        let db = root.join("s.db");
        let cfg = sqlite_config("id-1", &db.to_string_lossy());
        let pool = DbPool::connect(&cfg, None, &[root.clone()]).expect("连接");
        if let DbPool::Sqlite(conn) = &pool {
            conn.execute_batch("CREATE TABLE t (v INTEGER); INSERT INTO t VALUES (1);")
                .expect("建表");
        }
        let DbPool::Sqlite(conn) = &pool else {
            panic!("期望 SQLite 连接");
        };
        // soft = 0 → 首轮检查即命中（deterministic，不依赖真实等待）
        let deadline = QueryDeadline::new(Duration::from_millis(0));
        let err = query_sqlite_with_deadline(
            conn,
            "SELECT v FROM t",
            &QueryCancel::new(),
            &deadline,
            "q-soft",
        )
        .expect_err("soft 超时");
        assert_eq!(err.code, DbErrorCode::Timeout);
        assert!(!err.discard_connection, "soft 超时不弃连接");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn t_db_p10_hard_timeout_marks_connection_for_discard() {
        let root = temp_root("hard");
        let db = root.join("h.db");
        let cfg = sqlite_config("id-1", &db.to_string_lossy());
        let pool = DbPool::connect(&cfg, None, &[root.clone()]).expect("连接");
        if let DbPool::Sqlite(conn) = &pool {
            conn.execute_batch("CREATE TABLE t (v INTEGER); INSERT INTO t VALUES (1);")
                .expect("建表");
        }
        let DbPool::Sqlite(conn) = &pool else {
            panic!("期望 SQLite 连接");
        };
        // 把起点推到过去，使 hard 先于 soft 判定命中
        let mut deadline =
            QueryDeadline::new(Duration::from_secs(DB_SOFT_TO_HARD_GRACE_SECS as u64 + 10));
        deadline.start =
            Instant::now() - Duration::from_secs(DB_SOFT_TO_HARD_GRACE_SECS as u64 + 20);
        let err = query_sqlite_with_deadline(
            conn,
            "SELECT v FROM t",
            &QueryCancel::new(),
            &deadline,
            "q-hard",
        )
        .expect_err("hard 超时");
        assert_eq!(err.code, DbErrorCode::Timeout);
        assert!(err.discard_connection, "hard 超时必须弃连接而非归还池");
        fs::remove_dir_all(&root).ok();
    }

    // -- T-db-p11 空/超长/多语句在进入驱动前被拒 --

    #[test]
    fn t_db_p11_sql_guardrails_reject_before_driver() {
        let root = temp_root("guard");
        let db = root.join("g.db");
        let cfg = sqlite_config("id-1", &db.to_string_lossy());
        let mut pool = DbPool::connect(&cfg, None, &[root.clone()]).expect("连接");
        let cancel = QueryCancel::new();

        assert_eq!(
            pool.query("   ", &cancel, None, "q").unwrap_err().code,
            DbErrorCode::SqlEmpty
        );
        let huge = format!("SELECT {} ", "1".repeat(DB_MAX_SQL_BYTES + 10));
        assert_eq!(
            pool.query(&huge, &cancel, None, "q").unwrap_err().code,
            DbErrorCode::SqlTooLarge
        );
        assert_eq!(
            pool.query("SELECT 1; DROP TABLE t", &cancel, None, "q")
                .unwrap_err()
                .code,
            DbErrorCode::MultipleStatements
        );
        assert_eq!(
            pool.query("SELECT 1", &cancel, Some(601), "q")
                .unwrap_err()
                .code,
            DbErrorCode::InvalidConfig
        );
        fs::remove_dir_all(&root).ok();
    }

    // -- T-db-p12 MySQL/Postgres 不伪造能力 --

    #[test]
    fn t_db_p12_remote_drivers_do_not_forge_support() {
        // 连接失败路径：不可达主机必须返回稳定错误码，且不 panic。
        let mut cfg = remote_config(SupportedDb::MySql);
        cfg.host = Some("127.0.0.1".to_string());
        cfg.port = Some(1); // 必然不可达
        let roots: Vec<PathBuf> = Vec::new();
        let err = DbPool::connect(&cfg, Some("secret-pw"), &roots).expect_err("应连接失败");
        assert_eq!(err.code, DbErrorCode::ConnectFailed);
        // 凭据不得随错误串外泄
        assert!(
            !err.message.contains("secret-pw"),
            "错误串不得含口令: {}",
            err.message
        );

        let mut cfg = remote_config(SupportedDb::Postgres);
        cfg.port = Some(1);
        let err = DbPool::connect(&cfg, Some("secret-pw"), &roots).expect_err("应连接失败");
        assert_eq!(err.code, DbErrorCode::ConnectFailed);
        assert!(
            !err.message.contains("secret-pw"),
            "错误串不得含口令: {}",
            err.message
        );
    }

    #[test]
    fn t_db_p12_ssl_require_is_explicitly_degraded() {
        let roots: Vec<PathBuf> = Vec::new();
        let mut cfg = remote_config(SupportedDb::MySql);
        cfg.ssl_mode = DbSslMode::Require;
        assert_eq!(
            DbPool::connect(&cfg, None, &roots).unwrap_err().code,
            DbErrorCode::NotSupported
        );

        let mut cfg = remote_config(SupportedDb::Postgres);
        cfg.ssl_mode = DbSslMode::Require;
        assert_eq!(
            DbPool::connect(&cfg, None, &roots).unwrap_err().code,
            DbErrorCode::NotSupported
        );
    }

    // -- T-db-p13 平台降级与 DbValue 计费 --

    #[test]
    fn t_db_p13_platform_gate_is_explicit() {
        // 非 Linux 应显式 DB_NOT_SUPPORTED；Linux 上本断言不成立，故只校验常量与分支存在性。
        if !cfg!(target_os = "linux") {
            let root = temp_root("platform");
            let db = root.join("p.db");
            let cfg = sqlite_config("id-1", &db.to_string_lossy());
            assert_eq!(
                DbPool::connect(&cfg, None, &[root.clone()])
                    .unwrap_err()
                    .code,
                DbErrorCode::NotSupported
            );
            fs::remove_dir_all(&root).ok();
        }
    }

    #[test]
    fn t_db_p13_value_byte_len_accounting() {
        assert_eq!(DbValue::Null.byte_len(), 4);
        assert_eq!(DbValue::Bool(true).byte_len(), 1);
        assert_eq!(DbValue::I64(1).byte_len(), 8);
        assert_eq!(DbValue::F64(1.0).byte_len(), 8);
        assert_eq!(DbValue::Text("abc".to_string()).byte_len(), 3);
        assert_eq!(DbValue::Binary { bytes: 9 }.byte_len(), 9);
    }

    #[test]
    fn t_db_p13_cancel_flag_is_thread_visible() {
        let cancel = QueryCancel::new();
        assert!(!cancel.is_cancelled());
        cancel.cancel();
        assert!(cancel.is_cancelled());
        // Arc 共享：克隆后状态一致
        let cloned = cancel.clone();
        assert!(cloned.is_cancelled());
        assert!(cloned.flag.load(AtomicOrdering::SeqCst));
    }
}
