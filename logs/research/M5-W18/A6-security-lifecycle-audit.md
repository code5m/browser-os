# A6 · M5-W18-R 安全与生命周期审计（dbx → mvp-browser-os-v3）· R2 证据闭环版

> Lane: A6 · Role: RESEARCH ONLY（只产出报告与 checkpoint，不改产品代码、不 push）
> BASE: `78d2cfb`（`origin/master` 在 `m5-w18-a6` worktree 的对齐点）
> 参考源（已钉版本）：
> - dbx 源码快照 `/home/ainfinit/Documents/极智简单/V3/research/dbx-src`（Apache-2.0，`Cargo.lock` SHA-256 `c0a7be12c05d8dffe867f1a70d4b82e881dec2da3bb622b5d3e3410c3d10e3a7`）
> - 现有分析 `/home/ainfinit/Documents/极智简单/V3/dbx-study/dbx-借鉴分析.md`（V5，已逐行核实）
> - 当前产品 = mvp-browser-os-v3（worktree `m5-w18-a6`，HEAD `78d2cfb`）
> 范围：审计 dbx 的「安全与生命周期」10 主题，逐条比对当前产品既有守卫，产出强制 W19 策略/测试案例。
> R2 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W18-R2（L1439-1469）+ `logs/checkpoints/A0-M5-W18-R1-audit-20260908.md`（A6=REWORK）。

---

## 0. R2 修订声明（回应 A0 R1 审计的两条误判）

A0 R1 审计第 4 条与第 41 条直接点名 A6 两处不准确，本版逐条纠正：

- **误判 1（已纠正）**：R1 报告原 §1.1 称当前产品凭据「**永不进入 JS 内存/网络/日志**」。
  事实（已用源码核实）：密码**确实瞬时进入 JS**——`useDatabaseStore.connect(password)`（`src/stores/useDatabaseStore.ts:86`）是 JS 函数参数，`bridge.dbConnect(cfg, password)`（`src/bridge.ts:375-376`）把 `password` 作为 IPC 调用实参跨出 webview 进程边界进入 Rust 命令 `db_connect(password: Option<String>)`（`src-tauri/src/bridge.rs:6003`）。
  **准确结论**：密码**瞬时存在于 JS 内存并跨 IPC 边界**，但**不持久化**于 Pinia store / 配置 DTO（`DbConnectionConfig` 结构性无 password 字段，`domain.rs:821`/`908` 注释 + `DB_CFG_HAS_PASSWORD_FIELD` 强制），也不写入 HTML payload / 表单 / 日志 / 审计；错误串经 `sanitize_message` 脱敏（`database.rs:225-233`）。原结论的「JS 内存」半句为假，本版全量修正。
- **误判 2（已纠正）**：R1 报告称 dbx 默认凭据主路径是明文 `FileSecretStore` JSON，且 `EncryptedPayload` 是「正确加密路径」。
  事实（已用源码核实）：
  - `FileSecretStore`（`crates/dbx-core/src/connection_secrets.rs:37`）**是死代码**——`FileSecretStore::new` 全仓零调用（`grep -rn "FileSecretStore::new" --include="*.rs" .` 返回 1=无匹配）。
  - dbx 桌面端 `src-tauri/src/commands/connection_secrets.rs` 已标注 legacy：「All storage now goes through `dbx_core::storage::Storage` (SQLite)」。
  - **dbx 活路径是 SQLite 明文 `connection_secrets` 表**（`storage.rs:326-329`：`secret TEXT NOT NULL`），经 `persist_secret_in_tx`（`storage.rs:4147`）写入，`save_password=true` 时落 `password` 明文（`storage.rs:2374-2380`）。
  - `EncryptedPayload`（`state_persistence.rs:460-547`，argon2id+AES-256-GCM+HMAC）**同样是死代码**——除 `state_persistence.rs` 自身测试（`:984+`）外零生产调用方。
  故 R1 的「FileSecretStore 主路径」与「EncryptedPayload 正确路径」两个判断均落空；本版改为按**真实活路径（SQLite 明文表 + 内存会话凭据仓）** 重新分类。

附带 R2 必须消的跨 lane 矛盾：A4 W18-R 源映射（`logs/research/M5-W18/A4-dbx-backend-architecture-map.md`）同样把 `connection_secrets.rs:37 FileSecretStore` 标为「⛔REJECT 主路径」，与 A6 R1 互相印证了同一错误前提。本版在 §6 MERGE_NOTES 显式消解：A4 与 A6 应统一改为「REJECT dbx SQLite 明文 `connection_secrets` 表」，而非已死的 `FileSecretStore`。

R2 还要求补齐：双方凭据 source-to-sink 全链路、dbx `save_password=false` 会话仓 / 迁移 / 清理调用路径、取消/超时/写确认竞态时间线与强制 fail-closed 测试——均已补入 §1.1、§1.4、§1.5、§4。

---

## 1. 逐主题审计

### 1.1 凭据存储与 source-to-sink 全链路（credential storage & flow）

#### 1.1.1 当前产品（source-to-sink）

```
[form 输入]
  └─ useDatabaseStore.connect(password: string)        src/stores/useDatabaseStore.ts:86
       ├─ 注释 L83：password 仅作「瞬时参数」传入后端
       ├─ L99：不进 payload/store/表单（Pinia store 无 password 字段）
       └─ bridge.dbConnect(cfg, password)              src/bridge.ts:375-376
            └─ invoke("db_connect", { cfg, password: password || null })   ← 跨 IPC 边界，password 为 JS 值
                 └─ #[tauri::command] db_connect(app, webview, cfg, password: Option<String>)  src-tauri/src/bridge.rs:6003
                      ├─ 探测连通：DbPool::connect(&cfg, password.as_deref(), &roots)   bridge.rs:6017
                      │     └─ password 作为独立参数（非 DSN / 非 DbConnectionConfig 字段） database.rs:423
                      ├─ 落密钥库：KeyringStore::save_token(&credential_key(&conn_id), &p)  bridge.rs:6024
                      │     └─ key = "db:<conn_id>"  (DB_CRED_PREFIX="db:"  database.rs:65; credential_key  L68)
                      │        └─ KeyringStore 用 keyring crate → 系统密钥库  src-tauri/src/core/keyring_store.rs:5-27
                      ├─ 登记配置：DbConnectionRegistry.configs 插入 cfg（无 password）        bridge.rs:6029
                      └─ 审计：log_audit("db.connect", "conn_id=.. kind=..") 无 SQL/凭据       bridge.rs:6031

[查询]
  └─ db_query(...)                                       src-tauri/src/bridge.rs:6036
       ├─ password = KeyringStore::get_token(&credential_key(&conn_id)).ok()  bridge.rs:6065  ← 从密钥库取回
       ├─ DbPool::connect(&cfg, password.as_deref(), &roots)                     bridge.rs:6075
       ├─ evaluate_db_query_gate(&sql, &cfg, encrypted, confirm_write)          bridge.rs:6082
       └─ 审计：仅 rows/truncated 标志                                       bridge.rs:6091

[断开]
  └─ db_disconnect(conn_id)                              src-tauri/src/bridge.rs:6100
       ├─ 移除登记 configs                                                bridge.rs:6106
       └─ let _ = KeyringStore::delete_token(&credential_key(&conn_id));    bridge.rs:6109  ← 错误被吞（见下）
```

**已核实守卫（真实有效）**：
- 凭据只落 OS 密钥库（`db:<conn_id>` 命名空间，与 git `repo_id` 隔离）；`DbConnectionConfig` 结构性无 password 字段。
- 错误串脱敏：`sanitize_message`（`database.rs:225-233`）截断 512 字符，键集 `["password","passwd","pwd","secret","token"]` 整体替换为 `***`；测试 `database.rs:939-951` 覆盖 `password=hunter2`→`password=***`。
- 审计 detail 禁含 SQL/凭据（`bridge.rs:6031`/`6091`）。
- `database.rs` 含 **23 个 `#[test]`** 用例（A0 R1 #2 已确认，原 R1 记 22 偏少，以 23 为准）。

**R1 发现的真实风险（非误判，保留）**：
- `db_disconnect` 用 `let _ =` 吞掉 keyring 删除错误（`bridge.rs:6109`）：密钥库删除失败时凭据**静默残留在 OS keyring**，审计仍记 `removed=true`。→ 计入 G4。
- 应用关闭路径：`ShutdownCoordinator` 不触碰 DB keyring（DbPool 每查询 drop，无长连），故 `db:<conn_id>` 凭据**跨重启持久**于 OS keyring；若连接配置被删除但 keyring 未清，会留下**孤儿 `db:` 键**（当前无迁移/清理调用路径）。→ 计入 G4。

#### 1.1.2 dbx（已核实的真实活路径，非 FileSecretStore）

```
[save_password=true 持久连接]
  └─ save_connection_config → persist_secret_in_tx(tx, id, "password", &config.password)  storage.rs:2374-2380
       └─ INSERT OR REPLACE INTO connection_secrets(connection_id, key, secret) VALUES(...,'password',<明文>)  storage.rs:4147-4159
          └─ 明文落 SQLite 表 connection_secrets(secret TEXT NOT NULL)                    storage.rs:326-329

[save_password=false 会话连接]
  └─ prepare_runtime_config 把 password 从 config 抽出                  dbx-web/src/routes/connection.rs:36
       └─ record_session_credentials → session_credentials.set(owner, id, password)  connection.rs:59 / dbx-core connection.rs:3303
            └─ SessionCredentialStore：内存 RwLock，键 (owner_scope, connection_id)  session_credentials.rs:109
                 └─ 进程退出即丢，绝不落盘（注释 session_credentials.rs:1-20）

[断开/清理]
  └─ release_runtime_config_on_disconnect → session_credentials.clear_connection(id)  runtime_config.rs:66 / 122
  └─ 配置删除 → delete_removed_connection_secrets 清 SQLite secrets                       connection_secrets.rs:335-365
  └─ 启动迁移 → load_connections 把明文 password 移入 secret store 并重写脱敏文件          connection_secrets.rs:127-191
```

**关键纠正**：
- dbx **活路径是 SQLite 明文 `connection_secrets` 表**，不是 `FileSecretStore`（`FileSecretStore::new` 零调用 = 死代码）。
- `EncryptedPayload`（argon2id+AES-256-GCM+HMAC）**零生产调用**，仅为自带测试的工具函数；dbx 当前**没有任何加密兜底**在使用。
- `save_password=false` 走 `SessionCredentialStore`（内存，按登录会话 owner 隔离，Web 端 `session_credential_status` 仅返布尔 `connection.rs:505`）。

#### 1.1.3 差距 / 分类（修订后）

- 当前产品的 **OS keyring（`db:<conn_id>`）优于 dbx 的 SQLite 明文 `connection_secrets` 表**——主结论仍成立，但比较对象须改为 SQLite 明文表，而非已死的 FileSecretStore。
- dbx SQLite 明文表 → **REJECT**（明文 at rest，且是当前活路径，比 FileSecretStore 更该拒）。
- `EncryptedPayload` 因零生产调用，降级为 **ADAPT（仅测试可用，无运行态行为可抄）**：若未来需要 headless/Docker 兜底，可 ADAPT 其 argon2id+AES-256-GCM+HMAC 形态；但须注明无既存调用方，移植时需自写调用点与测试。
- 当前产品 keyring 删除被 `let _ =` 吞错 + 无孤儿键清理 → **G4（ADAPT）**。

#### 1.1.4 W19 强制
- `DB_CRED_FALLBACK_ENCRYPTED`（G10）：任何非密钥库兜底必须 argon2id+AES-256-GCM+HMAC；**拒绝明文**（SQLite `connection_secrets` 表模式）。
- `DB_CRED_RETAIN_POLICY`（G4）：一次性/草稿断开即清会话凭据；已保存连接保留；`connections.json` 永不含明文凭据（沿用 `DB_CFG_HAS_PASSWORD_FIELD`）；**新增 keyring 删除失败的显性处理 + 配置删除时的孤儿 `db:` 键清理**。

### 1.2 DSN / 凭据脱敏（redaction）
- **dbx**：`models/connection.rs` `redact_connection_debug_value`（`:206`，键含 `password/passphrase/token/secret/apikey/connectionstring/initscript`→`[REDACTED]`）、`scrub_secrets`（`:296`）、`redacted_connection_url`（`:961`）、`redacted_url_params`（`:1509`）、`redact_postgres_url_params`（`:2138`）。
- **当前产品**：`database.rs:225 sanitize_message`（512 字符截断，键集 `["password","passwd","pwd","secret","token"]` `:233`）；`redact_token`（`:263`）；`DbPool` 自定义 `Debug`（`:413`）只打印驱动种类；错误串统一过 `sanitize_message`（`:444`）。
- **差距 / 分类**：当前产品覆盖错误串，但键集偏窄（缺 `apikey`/`connectionstring`/`initscript`/`passphrase`），且无「DSN URL 脱敏」专用函数。→ **REIMPLEMENT_FROM_BEHAVIOR**（扩展键集 + 新增 `redact_dsn`）（G9）。
- **W19 强制**：`DB_REDACT_DSN`（G9）。

### 1.3 写确认 / production 判定（write confirmation / production verdict）
- **dbx**：`production_safety.rs` `is_production_database`（`:98`，基于 `config.is_production` 或 `production_databases`）、`targets_production_database`（`:184`，**解析 SQL 跨库限定名**如 `DELETE FROM prod_app.users`）、`mongo_pipeline_targets_production_database`（`:110`，覆盖 `$out`/`$merge`）。`sql_risk.rs:686/703` 解析失败对写 fail-closed。`agent_tools.rs:90 confirmed_write_sql_permissions`（fail-closed）+ `:111 verify_confirmed_target`（后端二次校验确认 SQL 与执行 SQL 一致）+ `:699 normalize_sql_for_confirmation`（仅 trim 空白）。
- **当前产品**：`security_policy.rs` `ProductionVerdict{Production,NonProduction,Unknown}`（`:1106`，`Unknown` 按生产拒绝 `:1109`）、`is_production_database`（`:1146`，**仅基于所选连接信号**，不解析 SQL 内跨库限定名）、`require_write_confirmation`（`:1206`）、`evaluate_db_query_gate(&sql, &cfg, encrypted, confirm_write)`（`:1269`，**只收 `confirm_write: bool`，不比对确认的 SQL 文本**）。
- **差距 / 分类**：
  - **G1（ADAPT）**：当前 `confirm_write: bool` 可被「确认允许写、却跑另一条（可能指向生产）语句」绕过 → 升级为「确认 SQL 文本比对」。新增 `DB_WRITE_CONFIRM_TEXT_MATCH`（见 §4 竞态）。
  - **G2（ADAPT）**：新增 `production_databases: Vec<String>` 到 `DbConnectionConfig` + `targets_production(sql)`（跨库限定名解析）+ Mongo `$out/$merge` 分支（Mongo 支持时 DEFER）。新增 `DB_CROSS_DB_PROD_VERDICT`。
- **W19 强制**：`DB_WRITE_CONFIRM_TEXT_MATCH` + `DB_CROSS_DB_PROD_VERDICT`。

### 1.4 查询超时 / 取消竞态（timeout / cancel races）
- **dbx**：`models/connection.rs` `connect_timeout_secs`/`query_timeout_secs`/`idle_timeout_secs`（`:115-119`）、`effective_query_timeout_secs`（`:831`，**下限保护**：Spanner ≥120s `:829`）。取消：`query_cancel.rs` `RunningQueries` 注册表（`:84`）+ `register_interrupt` + `CancellationToken` + `DETACHED_REGISTRATION_GRACE_PERIOD=30min`（`:18`）；`query_result_export.rs:48 disconnect_with_timeout` / `:1261 kill_query_with_opts`（MySQL `KILL QUERY`）。
- **当前产品**：`database.rs` `DB_DEFAULT_QUERY_TIMEOUT_SECS=30`/`DB_MAX_QUERY_TIMEOUT_SECS=600`（`:47-49`），`resolve_timeout_secs`（`:369` 钳制 1..600，0/None→默认）。`QueryCancel`（`:168`）：L1 取消标志 + L2 取数循环每 `DB_CANCEL_CHECK_EVERY_ROWS=64` 行检查（`:53`/`:184-192`）。**但**：无 `db_cancel` 命令（`QueryCancel::cancel` 标 `#[allow(dead_code)]` `:187`）；MySQL/Postgres 取数返回 `NotSupported`（`:490-497`）；**无服务端 KILL**；无 `connect_timeout`/`idle_timeout`/驱动下限。
- **差距 / 分类**：
  - **G3（REIMPLEMENT_FROM_BEHAVIOR）**：补 `db_cancel` 命令 + 驱动侧 `kill_query`（MySQL `KILL QUERY`/Postgres `pg_cancel_backend`），并复测取消期间结果写回竞态。新增 `DB_CANCEL_SERVER_SIDE`。
  - **G7（REIMPLEMENT_FROM_BEHAVIOR）**：补 `connect_timeout`+`idle_timeout`+驱动查询超时下限（如 Spanner≥120s）。`DB_CONNECT_IDLE_TIMEOUT`。
- **W19 强制**：`DB_CANCEL_SERVER_SIDE`、`DB_CONNECT_IDLE_TIMEOUT`。

### 1.5 连接池 / 会话清理（pool / session cleanup）
- **dbx**：`runtime_config.rs` `should_release_runtime_config_on_disconnect`（`:51`）+ `release_runtime_config_on_disconnect`（`:66`，丢弃一次性/草稿连接的会话凭据，保留已保存配置 `:122-167` 测试）。导出 `disconnect_with_timeout`（`:48`/`:1392`）带清理预算。
- **当前产品**：`database.rs:404 DbPool` 枚举，**释放语义 = `drop`**（`:400-403`），`bridge.rs` 每次 `db_query`/`db_connect` 新建并随作用域 drop（`:6020`/`:6069`）。**未注册 `ShutdownCoordinator`**（每查询连接，关闭影响低）。keyring 凭据清理见 §1.1.1（`let _ =` 吞错 + 无孤儿键清理）。
- **差距 / 分类**：
  - **G8（DEFER/ADAPT）**：当前每查询 drop 合理，但一旦引入连接池/keepalive，必须注册 `ShutdownCoordinator` 清理（对齐 `bridge.rs:780 register_shutdown_tasks`）。`DB_POOL_SHUTDOWN_CLEANUP`。
  - **G4（ADAPT）**：凭据保留策略——当前 keyring 凭据长期保留且无失败处理/孤儿清理（§1.1.1）。`DB_CRED_RETAIN_POLICY`。
- **W19 强制**：`DB_POOL_SHUTDOWN_CLEANUP`（条件）、`DB_CRED_RETAIN_POLICY`。

### 1.6 上限约束（bounded rows / bytes / files）
- **dbx**：`agent_tools.rs`（LLM 上下文特化）`EXECUTE_QUERY_LIMIT=50`（`:19`）、`MAX_ALLOWED_ROWS=100`（`:28`）、cell 截断 `:31/34/55`。
- **当前产品**：`database.rs` `DB_MAX_SQL_BYTES=65536`（`:38`）、`DB_MAX_ROWS=1000`（`:41`）、`DB_MAX_RESULT_BYTES=4_194_304`（`:43`）、`DB_MAX_EXPORT_BYTES=16_777_216`（`:56`）、`DB_MAX_NAME_BYTES=128`（`:58`）。`QueryLimiter`（`:578`）+ `DbQueryResult.truncated/field_truncated`（`:156-158`），由 `check-database-policy.py` 的 `DB_*` 门禁码位强制。
- **差距 / 分类**：当前产品上限（面向人类 UI）已完备且被策略门禁覆盖，量级合理（dbx 50/100 是 LLM 特化，不照搬）。**COPY/保持**。
- **W19 强制**：无新增。

### 1.7 历史持久化（history persistence）
- **dbx**：`storage.rs` `CREATE TABLE history`（`:332`）、`save_history_entry`（`:1000`）、`DELETE ... LIMIT MAX_HISTORY`（`:1029`）、`escape_history_like_pattern`（`:926`）+ 绑定参数 LIKE 检索（防注入）。
- **当前产品**：**尚未实现**。`check-database-policy.py` 的 `DB_PERSIST_NOT_ATOMIC` 仍为 **PENDING**（连接配置落盘未做），查询历史更无。
- **差距 / 分类**：**G5（ADAPT）**：只存 `connection_id + 截断 SQL + 标志位`，绝不存凭据；`MAX_HISTORY` 封顶；检索参数化 + LIKE 转义。`DB_HISTORY_SANITIZED`。
- **W19 强制**：`DB_HISTORY_SANITIZED`（含 `DB_PERSIST_NOT_ATOMIC` 提升 ACTIVE）。

### 1.8 导入 / 导出（import / export）
- **dbx**：`query_result_export.rs`（带 `disconnect_with_timeout`、size budget）、`export_download.rs:9 sanitize_archive_file_name`（`:972` 测试 `../../prod\r\n".zip`→`prod___.zip`）。
- **当前产品**：定义 `DB_MAX_EXPORT_BYTES=16_777_216`（`:56`），但**无导出命令实现**（M4 范围外）。
- **差距 / 分类**：**G6（REIMPLEMENT_FROM_BEHAVIOR）**：导出文件名净化（防 `../`/空字节/强制后缀）、大小受 `DB_MAX_EXPORT_BYTES` 约束、内容不含凭据/SQL 原文。`DB_EXPORT_SANITIZED`。
- **W19 强制**：`DB_EXPORT_SANITIZED`。

### 1.9 错误 / 日志脱敏（error / log sanitization）
- **dbx**：`redact_connection_debug_value`/`scrub_secrets`/`redacted_connection_url`（见 1.2）；`agent_kv.rs:1150 sanitize_metrics_url`。
- **当前产品**：`sanitize_message`（`:225`）、`redact_token`（`:263`）、审计 detail 禁含 `sql=/password/...`（`DB_CRED_IN_AUDIT` + `AUDIT_FORBIDDEN` 键集 `:197`）。
- **差距 / 分类**：与 1.2 同源，合并为 **G9（REIMPLEMENT_FROM_BEHAVIOR）**。
- **W19 强制**：`DB_REDACT_DSN`。

### 1.10 关闭流程（shutdown）
- **dbx**：`app_settings.rs:83 complete_app_close`（单一真退出入口）——但这是 `dbx-web` 后端；桌面端另有路径。
- **当前产品**：`shutdown.rs` `ShutdownCoordinator`（幂等/失败隔离/重入不死锁 `:152`/`:185`/`:338` 测试），`bridge.rs:780 register_shutdown_tasks`（grid/terminals/tabs/background-workers）。DbPool 每查询 drop（1.5），无需单独清理。
- **差距 / 分类**：当前关闭协调器**强于** dbx 描述。唯一缺口是 1.5 的 `DB_POOL_SHUTDOWN_CLEANUP`（条件触发）。**COPY/保持**。
- **W19 强制**：无新增（除条件性 `DB_POOL_SHUTDOWN_CLEANUP`）。

---

## 2. 上游源映射（精确文件:符号:行，已逐条源码核实）

| 主题 | dbx 证据（真实活路径） | 当前产品证据 |
|---|---|---|
| 凭据存储 | storage.rs:326/4147/2374-2380（SQLite 明文表）；session_credentials.rs:109/set; runtime_config.rs:51/66；connection_secrets.rs:335-365 迁移清理 | core/keyring_store.rs:5-27；bridge.rs:6003/6017/6024/6065/6109；database.rs:65/68/423；useDatabaseStore.ts:86/99/101；bridge.ts:375-376；domain.rs:821/908 |
| 脱敏 | models/connection.rs:206/296/961/1509/2138 | database.rs:225/233/263/444；check-database-policy.py DB_* |
| 写确认/生产 | production_safety.rs:98/110/184；sql_risk.rs:686/703；agent_tools.rs:90/111/699 | security_policy.rs:1106/1146/1206/1269 |
| 超时/取消 | models/connection.rs:115/814/831；query_cancel.rs:18/84 + register_interrupt/cancel；query_result_export.rs:48/1261/1392 | database.rs:47-49/53/168/184-192/369/490-497；bridge.rs:6020/6069 |
| 池/会话清理 | runtime_config.rs:51/66 | database.rs:400-409；bridge.rs:780 |
| 上限 | agent_tools.rs:19/28/31/34 | database.rs:38-58/156-158/578 |
| 历史 | storage.rs:332/1000/1029/1098/926 | （无，PENDING: DB_PERSIST_NOT_ATOMIC） |
| 导入/导出 | query_result_export.rs；export_download.rs:9/972 | database.rs:56（仅常量） |
| 错误/日志 | models/connection.rs（同1.2） | database.rs:225/263；check-database-policy.py DB_* |
| 关闭 | app_settings.rs:83 | shutdown.rs 全；bridge.rs:780 |

**已死亡/不得作为「活路径」引用的 dbx 符号**（R2 已核实）：`connection_secrets.rs:37 FileSecretStore`（`FileSecretStore::new` 零调用）、`state_persistence.rs:460-547 EncryptedPayload`（零生产调用方，仅自带测试）。

---

## 3. 分类计数（修订后）

- **COPY（3）**：OS keyring 凭据存储（1.1.3，当前产品 `db:<conn_id>` 优于 dbx SQLite 明文表）、结果上限常量与 `truncated` 标志（1.6）、`ShutdownCoordinator` 幂等关闭（1.10）。
- **ADAPT（4）**：`verify_confirmed_target` 的「确认 SQL 文本比对」（G1）、跨库生产判定 + `production_databases`（G2）、连接配置持久化原子写 + 凭据保留/清理策略（G4/G5）、加密兜底 `EncryptedPayload`（G10，注明零生产调用、仅测试可用）。
- **REIMPLEMENT_FROM_BEHAVIOR（3）**：服务端取消 `kill_query`（G3）、连接/空闲超时与驱动下限（G7）、DSN/调试脱敏扩展（G9）。
- **DEFER（2）**：Mongo `$out/$merge` 生产判定（G2 子项）、`DB_POOL_SHUTDOWN_CLEANUP`（G8，待连接池引入）。
- **REJECT（1，修订）**：dbx **SQLite 明文 `connection_secrets` 表**（1.1.2 活路径），替代原误判的已死 `FileSecretStore`。JDBC 侧车维持 REJECT（已在 `DB_JDBC_SIDECAR` 拒绝）。

> 注：R1 原 REJECT=2（`FileSecretStore` + JDBC 侧车）。R2 核实 `FileSecretStore` 为死代码后，REJECT 主项改为 SQLite 明文表；JDBC 侧车仍 REJECT，故 REJECT 仍计 2（对象变了，数量不变）。

---

## 4. 竞态时间线与强制 fail-closed 测试（R2 新增）

### 4.1 写确认竞态（G1 / `DB_WRITE_CONFIRM_TEXT_MATCH`）

**当前产品现状（危险）**：`evaluate_db_query_gate(&sql, &cfg, encrypted, confirm_write: bool)`（`security_policy.rs:1269`）只收布尔。用户在 UI 点「允许写」后，`confirm_write=true` 即放行**任意** `sql`，包括指向生产库的跨库限定名语句（如 `DELETE FROM prod_app.users`）。

**攻击时间线**：
```
T0 UI 弹出写确认，展示 SQL_A="INSERT INTO local.t"
T1 用户点确认 → confirm_write=true（仅布尔，未绑定 SQL_A）
T2 前端（或被篡改的 IPC 调用）发送 db_query{ sql=SQL_B, confirm_write=true }
T3 db_query 经 check_invocation_source 放行 → evaluate_db_query_gate(SQL_B, cfg, enc, true)
T4 require_write_confirmation 见 confirm_write=true 即放行 → SQL_B 执行
```
因 `confirm_write` 不携带/不比对 SQL 文本，T2 的 SQL_B 可完全不同于 T0 展示的 SQL_A。

**dbx 正确形态（待 ADAPT）**：`confirmed_write_sql_permissions`（`agent_tools.rs:90`）+ `verify_confirmed_target`（`agent_tools.rs:111`，校验 `confirmed_connection_id/database/schema` 与执行目标一致）+ `normalize_sql_for_confirmation`（`agent_tools.rs:699`，仅 trim 空白、语义不变须逐字相等）。

**强制 fail-closed 测试（W19 验收断言）**：
1. 给定 `confirmed_sql="INSERT INTO t VALUES(1)"`、`allow_write=true`、`production=false`，执行 `sql="DELETE FROM prod.users"` → **拒绝**（确认文本 ≠ 执行文本）。
2. `normalize_sql_for_confirmation` 仅 trim：前后空白差异放行，但大小写/内部空白/注释/字面量差异 → **拒绝**。
3. `production=true` 时无论 `confirmed_sql` 是否为空 → **一律拒绝**（fail-closed）。
4. `allow_write=false` 或 `confirmed_sql` 为空 → 任何写语句 **拒绝**。
5. 确认的 `connection_id/database/schema` 与实际执行目标不一致 → **拒绝**（防跨库限定名绕过）。

### 4.2 取消竞态（G3 / `DB_CANCEL_SERVER_SIDE`）

**当前产品现状**：`QueryCancel`（`database.rs:168`）仅有 L1 原子标志 + L2 取数循环每 64 行检查（`database.rs:184-192`）；`QueryCancel::cancel` 标 `#[allow(dead_code)]`（`:187`）——**无 `db_cancel` 命令触发**，故当前竞态不可达（无外部触发点）。MySQL/Postgres 取数 `NotSupported`（`:490-497`），无服务端 KILL。

**dbx 正确形态（待 REIMPLEMENT）**：`query_cancel.rs` `RunningQueries` 注册表（`:84`）把 `execution_id` 与 `CancellationToken` + `InterruptFn` 同锁存放（避免「任务已移除但驱动刚注册 interrupt 闭包」的泄漏窗口，注释 `query_cancel.rs:71-79`）；`register_interrupt` 注册驱动级带外中断；`DETACHED_REGISTRATION_GRACE_PERIOD=30min`（`:18`）回收孤儿注册。

**强制 fail-closed 测试（W19 验收断言）**：
1. `db_cancel(execution_id)` 存在且 ACL 顺序正确（在 `generate_handler!` 注册 + `default-commands.toml` 放行 + `bridge.ts` 暴露）。
2. 取消标志置位后，取数循环在下一个 64 行检查点内停止；**已写回的部分结果不被二次写入 / 不 panic / 不双发**。
3. MySQL/Postgres 走 `KILL QUERY`/`pg_cancel_backend` 带外中断；SQLite 软取消标志被检查。
4. 取消后 `RunningQueries` 注册在 30min grace 内可回收，无 interrupt 闭包泄漏（复测 dbx 注释中的双 map 竞态窗口）。
5. 取消期间连接处于「正在写结果」态时，取消不得导致连接句柄处于不一致状态（drop 安全）。

### 4.3 超时竞态（G7 / `DB_CONNECT_IDLE_TIMEOUT`）

**当前产品现状**：`resolve_timeout_secs`（`database.rs:369`）钳制 1..600，0/None→30 默认；但仅作用于查询。无 `connect_timeout`/`idle_timeout`，无驱动查询超时下限（dbx 对 Spanner 强制 ≥120s `:829`）。

**dbx 正确形态（待 REIMPLEMENT）**：`effective_query_timeout_secs`（`:831`）带下限保护；`connect_timeout_secs`/`idle_timeout_secs`（`:115-119`）独立维度。

**强制 fail-closed 测试（W19 验收断言）**：
1. `resolve_timeout_secs(Some(0))`→默认 30；`Some(700)`→`DB_INVALID_CONFIG` 错误（非 `DB_TIMEOUT`，属配置校验）。
2. 引入 `connect_timeout`：连接建立超过阈值 → **fail-closed 中止**（不无限挂起）。
3. 引入驱动查询超时下限（如 Spanner≥120s）：设置低于下限的值被钳到下限。
4. 超时触发时，若驱动不支持带外取消，连接/池须被**弃用**（hard drop），不得复用半开连接。
5. 超时与取消并发：超时先到则取消标志不得再触发对已弃连接的带外中断（无 use-after-free）。

---

## 5. 强制 W19 策略/测试案例清单

| # | 码位（建议） | 来源 | 必须通过的测试/策略断言 |
|---|---|---|---|
| G1 | `DB_WRITE_CONFIRM_TEXT_MATCH` | 1.3/§4.1 | 确认 `confirmed_write_sql` 与执行 SQL 经 `normalize_sql_for_confirmation`（仅 trim）后**逐字相等**才放行；确认空/不同/目标不一致 → 拒绝；`production` 无论确认与否拒绝。 |
| G2 | `DB_CROSS_DB_PROD_VERDICT` | 1.3 | `DELETE FROM prod_app.users`（选中库=staging）被 `targets_production` 判生产；`production_databases` 列表生效；Mongo `$out`/`$merge` 指向生产库命中（Mongo 支持时）。 |
| G3 | `DB_CANCEL_SERVER_SIDE` | 1.4/§4.2 | `db_cancel` 命令存在且 ACL 顺序正确；MySQL/Postgres 走 `kill_query`；取消期间结果不双写/不 panic；`RunningQueries` 30min 内可回收无泄漏；SQLite 软取消每 64 行检查。 |
| G4 | `DB_CRED_RETAIN_POLICY` | 1.1/1.5 | 一次性/草稿断开即清会话凭据；已保存连接保留；`connections.json` 永不含明文凭据；**keyring 删除失败显式处理 + 配置删除时清理孤儿 `db:` 键**。 |
| G5 | `DB_HISTORY_SANITIZED` | 1.7 | 历史只存 `conn_id + 截断 SQL + 标志`，无凭据；`MAX_HISTORY` 封顶；检索参数化 + LIKE 转义（注入不可达）。 |
| G6 | `DB_EXPORT_SANITIZED` | 1.8 | 导出文件名净化（去 `../`/空字节/强制后缀）；大小 ≤ `DB_MAX_EXPORT_BYTES`；内容不含凭据/SQL 原文。 |
| G7 | `DB_CONNECT_IDLE_TIMEOUT` | 1.4/§4.3 | 新增 `connect_timeout`/`idle_timeout` 常量；驱动查询超时下限生效（如 Spanner≥120s）；`resolve_timeout_secs` 兼容；超时弃用半开连接。 |
| G8 | `DB_POOL_SHUTDOWN_CLEANUP` | 1.5 | 若引入连接池，必须在 `register_shutdown_tasks` 注册清理任务；幂等、失败隔离（复用 `ShutdownCoordinator`）。 |
| G9 | `DB_REDACT_DSN` | 1.2/1.9 | 新增 `redact_dsn(url)`；`sanitize_message` 键集含 `apikey/connectionstring/initscript/passphrase`；所有 `Debug`/`Display` 不含主机/凭据。 |
| G10 | `DB_CRED_FALLBACK_ENCRYPTED` | 1.1 | 任何非密钥库兜底须 argon2id+AES-256-GCM+HMAC；**拒绝明文**（SQLite `connection_secrets` 表模式）。 |

**升级现有 PENDING**：`DB_PERSIST_NOT_ATOMIC` → 随 G5 落地后转 ACTIVE。

---

## 6. 未决问题（交 A0 / 后续 lane）

1. **Mongo/Spanner 支持**：G2（Mongo `$out/$merge`）、G7（Spanner 下限）依赖 W19 是否纳入对应驱动；当前 `DbPool` 仅 Sqlite 实装（MySQL/Postgres `NotSupported`，受 D27 无真实服务端约束）。
2. **确认 SQL 文本比对的 UX**：G1 要求前端在写确认弹窗携带精确 SQL 文本回传，需 A5 UI lane 配合；当前 `evaluate_db_query_gate` 仅收 `confirm_write: bool`。
3. **历史持久化存储介质**：dbx 用独立 SQLite `history` 表；当前产品偏好 `atomic_write` JSON（F10）。W19 定夺是否并入 `db_connections.json` 或独立文件（建议独立、同样原子写）。
4. **服务端取消可行性**：SQLite 无 `KILL` 原语，仅软轮询；MySQL/Postgres 实现 `kill_query` 须持有服务端连接 id（当前每查询连接可支持）。
5. **孤儿 keyring 凭据清理**：当前 `db_disconnect` 用 `let _ =` 吞掉 keyring 删除错误（`bridge.rs:6109`），且关闭/配置删除路径无 `db:` 键清理 → G4 必须补足（否则凭据残留 OS keyring）。

---

## 7. 依赖 / 合并顺序 / 安全债务（MERGE_NOTES）

- A6 研究**无产品代码改动**，与 A1（基线）、A4（db 后端）、A5（db 工作台 UI）、A10（移植账本）强相关。
- 合并顺序（按 board）：`A1 -> A2/A3 -> A4/A5/A6 -> A7/A8/A9 -> A10 -> A11 -> A0`。G1/G2 依赖 A4 的 `security_policy.rs` 与 `domain.rs`；G5/G6 依赖 A3 的 `database.rs`；G3/G7 依赖 A3 的驱动取数通道。
- **跨 lane 矛盾消解（R2 必做）**：A4 W18-R 源映射把 `connection_secrets.rs:37 FileSecretStore` 标为「⛔REJECT 主路径」，与 A6 R1 基于同一错误前提。本版确认 `FileSecretStore` 为死代码，活路径是 `storage.rs` 的 SQLite 明文 `connection_secrets` 表 + 内存 `SessionCredentialStore`。**A4 与 A6 须统一**：REJECT 对象改为 SQLite 明文表；`EncryptedPayload` 改为「零生产调用的 ADAPT 工具（仅测试）」。A0 在 W19 开档时应以本版为准。
- 许可：dbx Apache-2.0；移植单元须保留 `EncryptedPayload`/`production_safety`/`sql_risk`/`agent_tools` 的 NOTICE 与修改声明（交 A10 ledger）。
- 安全债务：本波仅研究，不修产品；G1–G10 为 W19 强制项，须由 A0 在 W19 开档时转为实现卡 + 策略门禁（A10 上账、A11 纳入验收矩阵）。
