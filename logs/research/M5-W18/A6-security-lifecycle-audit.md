# A6 · M5-W18-R 安全与生命周期审计（dbx → mvp-browser-os-v3）

> Lane: A6 · Role: RESEARCH ONLY（只产出报告与 checkpoint，不改产品代码、不 push）
> BASE: `78d2cfb`（`origin/master` 在 `m5-w18-a6` worktree 的对齐点）
> 参考源（已钉版本）：
> - dbx 源码快照 `/home/ainfinit/Documents/极智简单/V3/research/dbx-src`（Apache-2.0，`Cargo.lock` SHA-256 `c0a7be12c05d8dffe867f1a70d4b82e881dec2da3bb622b5d3e3410c3d10e3a7`）
> - 现有分析 `/home/ainfinit/Documents/极智简单/V3/dbx-study/dbx-借鉴分析.md`（V5，已逐行核实）
> - 当前产品 = mvp-browser-os-v3（worktree `m5-w18-a6`，HEAD `78d2cfb`）
> 范围：审计 dbx 的「安全与生命周期」10 主题，逐条比对当前产品既有守卫，产出强制 W19 策略/测试案例。

---

## 0. 摘要

当前产品在 M4（A3 连接池 + A4 命令/安全闸门）已建立**强于 dbx 默认路径**的多项守卫：
OS 密钥库（非明文文件）、纯函数 SQL 分类器 + 生产判定 + 写闸门、结果上限、超时分层、
`ShutdownCoordinator` 幂等关闭。dbx 的**默认凭据路径 `FileSecretStore` 是明文 JSON**（仅加密
`EncryptedPayload` 才是正确路径），当前产品直接采用 OS keyring 反而更安全。

但 dbx 在以下维度比当前产品**更严密**，构成 W19 强制补强的 10 个差距（G1–G10）：
二次确认的「SQL 文本比对」、跨库限定名写的生产判定、服务端取消（KILL）、连接配置持久化与
凭据保留策略、查询历史持久化（含脱敏+参数化检索）、导出文件名净化、连接/空闲超时与驱动下限、
关闭时的连接池/会话清理、DSN/调试输出脱敏广度、以及「明文凭据文件兜底」必须拒绝。

分类汇总（详见 §3）：COPY 3 / ADAPT 4 / REIMPLEMENT_FROM_BEHAVIOR 3 / DEFER 2 / REJECT 2。

---

## 1. 逐主题审计

### 1.1 凭据存储（credential storage）
- **dbx**：`crates/dbx-core/src/connection_secrets.rs`
  - `ConnectionSecretStore` trait（`:28`）+ `FileSecretStore`（`:37`）：**明文 JSON 文件**读写（`read_store`/`write_store` `:46-69`）——这是 dbx 的默认主路径，存在明文落盘风险。
  - `save_connections_to_file`/`load_connections_from_file`（`:97`/`:127`）把 `password` 等分离存储到 secret store。
  - 正确路径：`state_persistence.rs:476-547` `EncryptedPayload::encrypt/decrypt`（**argon2id + AES-256-GCM**，KDF 19 MiB / 2 轮 / HMAC-SHA256 完整性 `sign`/`verify`）。
  - 凭据键常量 `:6-26`（`MAIN_PASSWORD_KEY` 等）。
- **当前产品**：`src-tauri/src/core/keyring_store.rs:5-27` `KeyringStore` 用系统密钥库（macOS Keychain / Windows Credential Manager / Linux secret-service），**凭据永不进入 JS 内存/网络/日志**。`DbPool::connect` 把 `password` 作为**独立参数**传入（`:422`），不拼接 DSN，不落在 `DbConnectionConfig` 上（`check-database-policy.py::DB_CFG_HAS_PASSWORD_FIELD` 结构性禁止）。
- **差距 / 分类**：当前产品的 keyring 路径**优于** dbx 明文默认。dbx 明文 `FileSecretStore` → **REJECT**。若未来需要 headless/Docker 兜底，必须复用 dbx `EncryptedPayload` 的 argon2id+AES-256-GCM+HMAC → **ADAPT**（G10）。
- **W19 强制**：`DB_CRED_FALLBACK_ENCRYPTED`——任何非密钥库兜底必须加密且带 HMAC；禁止明文 JSON。

### 1.2 DSN / 凭据脱敏（redaction）
- **dbx**：`crates/dbx-core/src/models/connection.rs`
  - `redact_connection_debug_value`（`:206`，递归扫描 JSON 键含 `password/passphrase/token/secret/apikey/connectionstring/initscript` → `[REDACTED]`，`:219`）。
  - `scrub_secrets`（`:296`，清空 SSH/Proxy/HTTP 隧道密码/passphrase/token）。
  - `redacted_connection_url`（`:961`）、`redacted_url_params`（`:1509`）、`redact_postgres_url_params`（`:2138`）。
- **当前产品**：`database.rs:230 sanitize_message`（截断 512 字符，把 `password=/passwd=/pwd=/secret=/token:` 形态替换为 `***`，`:233` 键集）；`redact_token`（`:263`）；`DbPool` 自定义 `Debug`（`:413`）只打印驱动种类，不泄露句柄/主机/凭据；错误串统一过 `sanitize_message`（`:444`）。
- **差距 / 分类**：当前产品覆盖错误串，但键集偏窄（缺 `apikey`/`connectionstring`/`initscript`/`passphrase`），且无「DSN URL 脱敏」专用函数（dbx `redacted_connection_url`）。→ **REIMPLEMENT_FROM_BEHAVIOR**（扩展键集 + 新增 `redact_dsn`）（G9）。
- **W19 强制**：`DB_REDACT_DSN`——新增 `redact_dsn(url)` 净化连接 URL；`sanitize_message` 键集补齐 `apikey`/`connectionstring`/`initscript`/`passphrase`；确认所有 `Debug`/`Display` 不含主机/凭据。

### 1.3 写确认 / production 判定（write confirmation / production verdict）
- **dbx**：`production_safety.rs`
  - `is_production_database`（`:98`，基于 `config.is_production` 或 `production_databases` 列表）。
  - `targets_production_database`（`:184`，**解析 SQL 中跨库限定名**如 `DELETE FROM prod_app.users`，选中库是 staging 也能命中）。
  - `mongo_pipeline_targets_production_database`（`:110`，覆盖 `$out`/`$merge` 写阶段可指向不同库）。
  - `sql_risk.rs:686 classify_sql_risk_for_database` / `:703 is_dangerous_sql_for_database`（**解析失败对写 fail-closed**）。
- **当前产品**：`security_policy.rs`
  - `ProductionVerdict{Production,NonProduction,Unknown}`（`:1106`），`Unknown` = 按生产拒绝（`:1109`）。
  - `is_production_database`（`:1146`）：仅基于**所选连接**的信号（SQLite/主机回环私网 `host_is_loopback_or_private_ip` `:1240` / 名称词表 / `production_hint` / `encrypted`）。**不解析 SQL 内跨库限定名**。
  - `require_write_confirmation`（`:1206`）：`Write` 需 `allow_write` + `NonProduction` + `confirmed`。
  - `evaluate_db_query_gate`（`:1269`）：`confirm_write: bool` —— **只确认布尔，不比对确认的 SQL 文本**。
- **差距 / 分类**：
  - **G1（ADAPT）**：dbx `agent_tools.rs:90 confirmed_write_sql_permissions`（fail-closed）+ `:111 verify_confirmed_target`（后端二次校验确认的 SQL 与执行 SQL 一致）+ `:699 normalize_sql_for_confirmation`（仅 trim 空白，语义不变则必须精确匹配）。当前产品 `confirm_write: bool` 可被「确认允许写、却跑另一条（可能指向生产）语句」绕过 → 必须升级为「确认 SQL 文本比对」。新增 `DB_WRITE_CONFIRM_TEXT_MATCH`。
  - **G2（ADAPT）**：当前产品需新增 `production_databases: Vec<String>` 到 `DbConnectionConfig`，并加 `targets_production(sql)`（跨库限定名解析）+ Mongo `$out/$merge` 分支（Mongo 支持时）。新增 `DB_CROSS_DB_PROD_VERDICT`。
- **W19 强制**：`DB_WRITE_CONFIRM_TEXT_MATCH` + `DB_CROSS_DB_PROD_VERDICT`（含 Mongo 分支 DEFER 到 Mongo 支持）。

### 1.4 查询超时 / 取消竞态（timeout / cancel races）
- **dbx**：`models/connection.rs`
  - `connect_timeout_secs`/`query_timeout_secs`/`idle_timeout_secs`（`:115-119`），`effective_connect_timeout_secs`（`:814`）、`effective_query_timeout_secs`（`:831`，**下限保护**：Spanner 强制 ≥120s，`:829`）。
  - 取消：`query_result_export.rs:48 disconnect_with_timeout(cleanup_timeout)`、`crate::db::mysql::kill_query_with_opts`（`:1261` `running_queries.register_interrupt` + MySQL `KILL QUERY`）、流错误后 `:1392` 带超时断开。
- **当前产品**：`database.rs`
  - `DB_DEFAULT_QUERY_TIMEOUT_SECS=30` / `DB_MAX_QUERY_TIMEOUT_SECS=600`（`:47-49`），`resolve_timeout_secs`（`:369` 钳制 1..600，0/None→默认）。
  - `QueryCancel`（`:168`）：L1 取消标志 + L2 取数循环每 `DB_CANCEL_CHECK_EVERY_ROWS=64` 行检查（`:53`、`:184-192`）。
  - **但**：无 `db_cancel` 命令（命令名未冻结，`QueryCancel::cancel` 标 `#[allow(dead_code)]`，`:187`）；MySQL/Postgres 取数通道未实现（`:490-497` `NotSupported`）；**无服务端 KILL**，SQLite 软轮询无法强制中断长查询。无 `connect_timeout`/`idle_timeout`。
- **差距 / 分类**：
  - **G3（REIMPLEMENT_FROM_BEHAVIOR）**：实现 `db_cancel` + 驱动侧 `kill_query`（MySQL `KILL QUERY` / Postgres `pg_cancel_backend`），并重测「取消期间结果写回」竞态（dbx `register_interrupt` 模式）。新增 `DB_CANCEL_SERVER_SIDE`。
  - **G7（REIMPLEMENT_FROM_BEHAVIOR）**：补 `connect_timeout` + `idle_timeout` + 驱动查询超时下限（如 Spanner 类 ≥120s 下限）。`DB_CONNECT_IDLE_TIMEOUT`。
- **W19 强制**：`DB_CANCEL_SERVER_SIDE`、`DB_CONNECT_IDLE_TIMEOUT`。

### 1.5 连接池 / 会话清理（pool / session cleanup）
- **dbx**：`runtime_config.rs`
  - `should_release_runtime_config_on_disconnect`（`:51`）+ `release_runtime_config_on_disconnect`（`:66`）：断开时**丢弃一次性/草稿连接的会话凭据**，但保留已保存配置（`:122-167` 测试）。
  - 导出连接 `disconnect_with_timeout`（`:48`/`:1392`）带清理预算，避免悬挂。
- **当前产品**：`database.rs:404 DbPool` 枚举，**释放语义 = `drop`**（`:400-403`，曾删除空实现 `close(self)` 以免「以为关了其实没关」）。`bridge.rs` 每次 `db_query`/`db_connect` 新建 `DbPool` 并随作用域 drop（`:6020`/`:6069`）。**未注册到 `ShutdownCoordinator`**（当前为无状态每查询连接，关闭影响低）。
- **差距 / 分类**：
  - **G8（DEFER/ADAPT）**：当前每查询 drop 设计合理，但**一旦 W19 引入连接池/keepalive/长连接**，必须注册 `ShutdownCoordinator` 清理任务（对齐 `bridge.rs:780 register_shutdown_tasks` 的 grid/terminals/tabs/background 模式）。新增 `DB_POOL_SHUTDOWN_CLEANUP`。
  - **G4 子项（ADAPT）**：断开时凭据保留策略——当前 keyring 凭据长期保留；可借鉴 dbx「一次性连接断开即清会话凭据」语义（仅对 `one_time`/草稿生效）。`DB_CRED_RETAIN_POLICY`。
- **W19 强制**：`DB_POOL_SHUTDOWN_CLEANUP`（条件触发）、`DB_CRED_RETAIN_POLICY`。

### 1.6 上限约束（bounded rows / bytes / files）
- **dbx**：`agent_tools.rs`（**Agent/MCP 上下文**，面向 LLM 故更小）：`EXECUTE_QUERY_LIMIT=50`（`:19`）、`MAX_ALLOWED_ROWS=100`（`:28`）、`DEFAULT_QUERY_CELL_CHAR_LIMIT=200`/`MAX_QUERY_CELL_CHAR_LIMIT=4000`（`:31`/`:34`），cell 截断 `bounded_query_cell_option`（`:55`）。
- **当前产品**：`database.rs`
  - `DB_MAX_SQL_BYTES=65536`（`:38`）、`DB_MAX_ROWS=1000`（`:41`）、`DB_MAX_RESULT_BYTES=4_194_304`（`:43`）、`DB_MAX_EXPORT_BYTES=16_777_216`（`:56`）、`DB_MAX_NAME_BYTES=128`（`:58`）。
  - `QueryLimiter`（`:578`，`bytes/truncated/field_truncated`），单字段按 `DB_MAX_TEXT_FIELD_BYTES` 字符边界截断（`:595-604`）。
  - `DbQueryResult` 含 `truncated`/`field_truncated`（`:156-158`），由 `check-database-policy.py::DB_RESULT_LIMIT_MISSING` 强制。
- **差距 / 分类**：当前产品上限（面向人类 UI）已完备且被策略门禁覆盖，量级合理（dbx 的 50/100 是 LLM 上下文特化，不直接照搬）。**COPY/保持**。仅建议补：cell 截断亦在审计/导出可见（已含 `field_truncated`）。
- **W19 强制**：无新增；保持 `DB_RESULT_LIMIT_MISSING` + 现有常量。

### 1.7 历史持久化（history persistence）
- **dbx**：`storage.rs`
  - `CREATE TABLE history`（`:332`），`save_history_entry`（`:1000`），`DELETE ... LIMIT MAX_HISTORY`（`:1029` 容量封顶），`load/search/clear/delete`（`:1039`/`:1098`/`:1188`/`:1192`）。
  - `escape_history_like_pattern`（`:926`）+ 绑定参数做 LIKE 检索（防注入）；`HistoryConnectionFilter` 等。
- **当前产品**：**尚未实现**。`check-database-policy.py::DB_PERSIST_NOT_ATOMIC` 仍是 **PENDING**（连接配置落盘未做），查询历史更无。
- **差距 / 分类**：
  - **G5（ADAPT）**：W19 实现历史时，**只存** `connection_id + 截断 SQL + 标志位`，**绝不存凭据**；容量封顶 `MAX_HISTORY`；检索走参数化 + LIKE 转义（抄 dbx）。新增 `DB_HISTORY_SANITIZED`。
- **W19 强制**：`DB_HISTORY_SANITIZED`（含 `DB_PERSIST_NOT_ATOMIC` 提升为 ACTIVE）。

### 1.8 导入 / 导出（import / export）
- **dbx**：`query_result_export.rs`（带 `disconnect_with_timeout`、size budget）、`export_download.rs:9 sanitize_archive_file_name`/`dbx-web/.../nacos.rs:729/972`（文件名净化：`../../prod\r\n".zip` → `prod___.zip`，`:1021` 测试）。
- **当前产品**：定义 `DB_MAX_EXPORT_BYTES=16_777_216`（`:56`），但**无导出命令实现**（M4 范围外）。
- **差距 / 分类**：
  - **G6（REIMPLEMENT_FROM_BEHAVIOR）**：W19 实现导出时，文件名必须净化（防 `../`、空字节、非法字符、强制后缀），**禁止路径穿越**，大小受 `DB_MAX_EXPORT_BYTES` 约束，导出文件**不得含凭据/SQL 原文**（用截断后的展示值）。新增 `DB_EXPORT_SANITIZED`。
- **W19 强制**：`DB_EXPORT_SANITIZED`。

### 1.9 错误 / 日志脱敏（error / log sanitization）
- **dbx**：`redact_connection_debug_value`/`scrub_secrets`/`redacted_connection_url`（见 1.2）；`agent_kv.rs:1150 sanitize_metrics_url`（指标 URL 脱敏）。整体原则：凭据与查询体不进日志。
- **当前产品**：`sanitize_message`（`:230`）、`redact_token`（`:263`）、审计 detail 禁含 `sql=/password/...`（见 `check-database-policy.py::DB_CRED_IN_AUDIT` 与 `AUDIT_FORBIDDEN` 键集 `:197`）。
- **差距 / 分类**：与 1.2 同源，合并为 **G9（REIMPLEMENT_FROM_BEHAVIOR）** 扩展键集 + 新增 `redact_dsn`。
- **W19 强制**：`DB_REDACT_DSN`（合并 1.2）。

### 1.10 关闭流程（shutdown）
- **dbx**：`app_settings.rs:83 complete_app_close`（单一真退出入口，`app.exit(0)` 整体回收）——但这是 `dbx-web` 后端；桌面端另有自己的关闭路径。
- **当前产品**：`shutdown.rs` `ShutdownCoordinator`（幂等、失败隔离、重入不死锁、结构化报告，`:152`/`:185`/`:338` 测试），`bridge.rs:780 register_shutdown_tasks`（grid/terminals/tabs/background-workers 顺序注册）。DbPool 每查询 drop（1.5），无需单独清理。
- **差距 / 分类**：当前产品关闭协调器**强于** dbx 描述。唯一缺口是 1.5 的 `DB_POOL_SHUTDOWN_CLEANUP`（条件触发）。**COPY/保持**。
- **W19 强制**：无新增（除条件性 `DB_POOL_SHUTDOWN_CLEANUP`）。

---

## 2. 上游源映射（精确文件:符号:行）

| 主题 | dbx 证据 | 当前产品证据 |
|---|---|---|
| 凭据存储 | connection_secrets.rs:37/97/127；state_persistence.rs:476-547 | core/keyring_store.rs:5-27；database.rs:422 |
| 脱敏 | models/connection.rs:206/296/961/1509/2138 | database.rs:230/263；check-database-policy.py:197 |
| 写确认/生产 | production_safety.rs:98/110/184；sql_risk.rs:686/703；agent_tools.rs:90/111/699 | security_policy.rs:1106/1146/1206/1269；check-database-policy.py |
| 超时/取消 | models/connection.rs:115/814/831；query_result_export.rs:48/1261/1392 | database.rs:47-53/369；bridge.rs:6020/6069 |
| 池/会话清理 | runtime_config.rs:51/66 | database.rs:400-409；bridge.rs:780 |
| 上限 | agent_tools.rs:19/28/31/34 | database.rs:38-58/156-158/578 |
| 历史 | storage.rs:332/1000/1029/1039/1098/926 | （无，PENDING: DB_PERSIST_NOT_ATOMIC） |
| 导入/导出 | query_result_export.rs；export_download.rs:9/972；nacos.rs:729/972 | database.rs:56（仅常量） |
| 错误/日志 | models/connection.rs（同1.2）；agent_kv.rs:1150 | database.rs:230/263；check-database-policy.py:197 |
| 关闭 | app_settings.rs:83 | shutdown.rs 全；bridge.rs:780 |

---

## 3. 分类计数

- **COPY（直接沿用/保持，3）**：OS keyring 凭据存储（1.1）、结果上限常量与 `truncated` 标志（1.6）、`ShutdownCoordinator` 幂等关闭（1.10）。
- **ADAPT（移植并适配，4）**：`verify_confirmed_target` 的「确认 SQL 文本比对」（G1）、跨库生产判定 + `production_databases` 列表（G2）、连接配置持久化原子写 + 凭据保留策略（G4/G5）、加密兜底 `EncryptedPayload`（G10）。
- **REIMPLEMENT_FROM_BEHAVIOR（按行为重写，3）**：服务端取消 `kill_query`（G3）、连接/空闲超时与驱动下限（G7）、DSN/调试脱敏扩展（G9）。
- **DEFER（条件触发/二期，2）**：Mongo `$out/$merge` 生产判定（G2 子项，待 Mongo 支持）、`DB_POOL_SHUTDOWN_CLEANUP`（G8，待连接池引入）。
- **REJECT（拒绝，2）**：dbx 明文 `FileSecretStore` JSON 路径（1.1）、任何 JDBC 侧车（已在 `DB_JDBC_SIDECAR` 拒绝）。

---

## 4. 强制 W19 策略/测试案例清单

每个案例给出**建议码位名**与**必须通过的断言形态**（供 A10 上账、A11 纳入验收矩阵）：

| # | 码位（建议） | 来源 | 必须通过的测试/策略断言 |
|---|---|---|---|
| G1 | `DB_WRITE_CONFIRM_TEXT_MATCH` | 1.3 | 确认 `confirmed_write_sql` 与实际执行 SQL 经 `normalize_sql_for_confirmation`（仅 trim）后**逐字相等**才放行；确认空/不同 → 拒绝；`production` 目标无论确认与否拒绝。 |
| G2 | `DB_CROSS_DB_PROD_VERDICT` | 1.3 | `DELETE FROM prod_app.users`（选中库=staging）被 `targets_production` 判生产；`production_databases` 列表生效；Mongo `$out`/`$merge` 指向生产库命中（Mongo 支持时）。 |
| G3 | `DB_CANCEL_SERVER_SIDE` | 1.4 | `db_cancel` 命令存在且 ACL 顺序正确；MySQL/Postgres 走 `kill_query`；取消期间结果写回不 panic/不双写；SQLite 软取消标志在取数循环每 64 行被检查。 |
| G4 | `DB_CRED_RETAIN_POLICY` | 1.5 | 一次性/草稿连接断开即清会话凭据；已保存连接保留；`connections.json` 永不含明文凭据（沿用 `DB_CFG_HAS_PASSWORD_FIELD`）。 |
| G5 | `DB_HISTORY_SANITIZED` | 1.7 | 历史只存 `conn_id + 截断 SQL + 标志`，无凭据；`MAX_HISTORY` 封顶（LIMIT 淘汰）；检索参数化 + LIKE 转义（注入不可达）。 |
| G6 | `DB_EXPORT_SANITIZED` | 1.8 | 导出文件名净化（去 `../`/空字节/强制后缀）；大小 ≤ `DB_MAX_EXPORT_BYTES`；导出内容不含凭据/SQL 原文。 |
| G7 | `DB_CONNECT_IDLE_TIMEOUT` | 1.4 | 新增 `connect_timeout`/`idle_timeout` 常量；驱动查询超时下限生效（如 Spanner≥120s）；`resolve_timeout_secs` 兼容。 |
| G8 | `DB_POOL_SHUTDOWN_CLEANUP` | 1.5 | 若引入连接池，必须在 `register_shutdown_tasks` 注册清理任务；幂等、失败隔离（复用 `ShutdownCoordinator`）。 |
| G9 | `DB_REDACT_DSN` | 1.2/1.9 | 新增 `redact_dsn(url)`；`sanitize_message` 键集含 `apikey/connectionstring/initscript/passphrase`；所有 `Debug`/`Display` 不含主机/凭据。 |
| G10 | `DB_CRED_FALLBACK_ENCRYPTED` | 1.1 | 任何非密钥库兜底须 argon2id+AES-256-GCM+HMAC；**拒绝明文 JSON**（`FileSecretStore` 模式）。 |

**升级现有 PENDING**：`DB_PERSIST_NOT_ATOMIC` → 随 G5 落地后转 ACTIVE。

---

## 5. 未决问题（交 A0 / 后续 lane）

1. **Mongo/Spanner 支持**：G2（Mongo `$out/$merge`）、G7（Spanner 下限）依赖 W19 是否纳入对应驱动；当前 `DbPool` 仅 Sqlite 取数通道实装（MySQL/Postgres 取数为 `NotSupported`，受 D27 无真实服务端约束）。
2. **确认 SQL 文本比对的 UX**：G1 要求前端在「写确认」弹窗里携带精确 SQL 文本回传，需 A5/A6 UI lane 配合；当前 `evaluate_db_query_gate` 仅收 `confirm_write: bool`。
3. **历史持久化存储介质**：dbx 用独立 SQLite `history` 表；当前产品偏好 `atomic_write` JSON（F10）。W19 需定夺历史是否并入 `db_connections.json` 或独立文件（建议独立、同样原子写）。
4. **服务端取消可行性**：SQLite 无 `KILL` 原语，仅能软轮询；MySQL/Postgres 实现 `kill_query` 需要持有服务端连接 id（当前每查询连接可支持）。

---

## 6. 依赖 / 合并顺序 / 安全债务（MERGE_NOTES）

- A6 研究**无产品代码改动**，与 A1（基线）、A4（db 后端）、A5（db 工作台 UI）、A10（移植账本）强相关。
- 合并顺序（按 board）：`A1 -> A2/A3 -> A4/A5/A6 -> A7/A8/A9 -> A10 -> A11 -> A0`。本报告 G1/G2 依赖 A4 的 `security_policy.rs` 与 `domain.rs`；G5/G6 依赖 A3 的 `database.rs`；G3/G7 依赖 A3 的驱动取数通道。
- 安全债务：本波仅研究，不修产品；上述 G1–G10 为 W19 强制项，须由 A0 在 W19 开档时转为实现卡 + 策略门禁（A10 上账、A11 纳入验收矩阵）。
- 许可：dbx Apache-2.0；移植单元须保留 `EncryptedPayload`/`production_safety`/`sql_risk`/`agent_tools` 的 NOTICE 与修改声明（交 A10 ledger）。
