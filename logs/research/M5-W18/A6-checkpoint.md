# A6 · M5-W18-R2 Lane Checkpoint（Evidence Closure）

## Required lane output (per PARALLEL_COMMAND_BOARD.md §M5-W18-R2)

```
LANE=A6
STATUS=PASS_WITH_DEBT
BASE=78d2cfb
HEAD=<set on commit>
R2_VERDICT=REWORK_CLOSED  (原 R1=REWORK；本波按 A0 R1 审计第4/41条逐条纠正并补齐 R2 证据)
REFERENCE_EVIDENCE=/home/ainfinit/Documents/极智简单/V3/research/dbx-src (Apache-2.0, Cargo.lock SHA-256 c0a7be12c05d8dffe867f1a70d4b82e881dec2da3bb622b5d3e3410c3d10e3a7，已复核一致); /home/ainfinit/Documents/极智简单/V3/dbx-study/dbx-借鉴分析.md (V5); 当前产品 = mvp-browser-os-v3 worktree m5-w18-a6 @ 78d2cfb
FILES=logs/research/M5-W18/A6-security-lifecycle-audit.md, logs/research/M5-W18/A6-checkpoint.md
```

## R2 证据闭环（board L1462 要求逐条落实）

1. **凭据 source-to-sink 全链路（双方）**：已画。当前产品：`useDatabaseStore.connect(password)`(ts:86) → `bridge.dbConnect(cfg,password)`(bridge.ts:375) → IPC `db_connect(password:Option<String>)`(bridge.rs:6003) → `DbPool::connect`(bridge.rs:6017) → `KeyringStore::save_token(db:<conn_id>,p)`(bridge.rs:6024) → 查询时 `get_token`(bridge.rs:6065) → 断开 `delete_token` 但 `let _ =` 吞错(bridge.rs:6109)。dbx：`save_connection_config`→`persist_secret_in_tx`→SQLite 明文 `connection_secrets` 表(storage.rs:326/4147/2374)；`save_password=false`→`SessionCredentialStore`(session_credentials.rs:109)；断开 `release_runtime_config_on_disconnect`→`clear_connection`(runtime_config.rs:66)。
2. **纠正「never enters JS memory」伪命题**：已确认密码**瞬时进入 JS**（函数参数 + IPC 实参），但**不持久化**于 Pinia/配置 DTO/日志。审计报告 §0 显式纠正。
3. **dbx 真实活路径（非 FileSecretStore）**：已核实 `FileSecretStore::new` 零调用（死代码）；`EncryptedPayload` 零生产调用（仅自带测试）；活路径 = SQLite 明文 `connection_secrets` 表 + 内存 `SessionCredentialStore`。分类与 MERGE_NOTES 已修订，并消解与 A4 源映射的跨 lane 矛盾。
4. **竞态时间线 + 强制 fail-closed 测试**：§4 新增写确认(G1)/取消(G3)/超时(G7)三条时间线与每条 ≥5 条 fail-closed 验收断言。

## 源映射（修订后，逐条源码核实）

```
SOURCE_MAP=
当前产品:
  src/stores/useDatabaseStore.ts:83/86/99/101,
  src/bridge.ts:375-376,
  src-tauri/src/bridge.rs:6003/6017/6024/6065/6109,
  src-tauri/src/core/keyring_store.rs:5-27,
  src-tauri/src/database.rs:17/65/68/225-233/263/369/423/444/490-497/578,
  src-tauri/src/security_policy.rs:1106/1146/1206/1269,
  src-tauri/src/domain.rs:821/908,
  src-tauri/src/shutdown.rs, src-tauri/src/bridge.rs:780,
  check-database-policy.py (DB_* 门禁码位)
dbx (Apache-2.0, 已钉版):
  crates/dbx-core/src/storage.rs:326/4147/2374-2380/332/1000/1029/926,
  crates/dbx-core/src/connection_secrets.rs:37(死)/335-365(迁移清理),
  crates/dbx-core/src/session_credentials.rs:109/set/clear_connection,
  crates/dbx-core/src/runtime_config.rs:51/66,
  crates/dbx-core/src/state_persistence.rs:460-547(死,仅测试),
  crates/dbx-core/src/query_cancel.rs:18/84 + register_interrupt/cancel,
  crates/dbx-core/src/query_result_export.rs:48/1261/1392,
  crates/dbx-core/src/production_safety.rs:98/110/184,
  crates/dbx-core/src/agent_tools.rs:90/111/699,
  crates/dbx-core/src/models/connection.rs:206/296/961/1509/2138,
  crates/dbx-web/src/routes/connection.rs:36/59/505/609/625/640/679/706
```

## 分类（修订后）

```
CLASSIFICATION=
  COPY=3 (OS keyring db:<conn_id>; 结果上限常量+truncated; ShutdownCoordinator 幂等关闭)
  ADAPT=4 (verify_confirmed_target 确认SQL文本比对 G1; 跨库生产判定+production_databases G2;
           连接配置原子写+凭据保留/清理 G4/G5; 加密兜底 EncryptedPayload G10[注明零生产调用])
  REIMPLEMENT_FROM_BEHAVIOR=3 (服务端取消 kill_query G3; connect/idle 超时+驱动下限 G7; DSN/调试脱敏扩展 G9)
  DEFER=2 (Mongo $out/$merge 生产判定 G2子项; DB_POOL_SHUTDOWN_CLEANUP G8)
  REJECT=2 (dbx SQLite 明文 connection_secrets 表[替代已死 FileSecretStore]; JDBC 侧车)
```

## 验证

```
VERIFY=read-only source audit only（零产品代码执行）；
  当前产品守卫与 dbx 证据逐行交叉核对；
  dbx Cargo.lock SHA-256 复核一致（c0a7be12...）；
  FileSecretStore::new / EncryptedPayload 生产调用方以 grep 复核为 0；
  SQLite connection_secrets 表与 persist_secret_in_tx 调用点已定位（storage.rs:326/4147/2374）。
```

## 未解决（交 A0 / W19）

```
NEXT=W19 实现卡 G1–G10（待 A0 架构冻结）；
OPEN:
  - keyring 删除 `let _ =` 吞错 + 孤儿 `db:` 键清理 → G4 必补（bridge.rs:6109）
  - 跨 lane 矛盾已消解：A4 与 A6 统一 REJECT 对象为 SQLite 明文表（非死代码 FileSecretStore）
  - Mongo/Spanner 驱动依赖 W19 范围裁决（G2/G7）
  - 确认 SQL 文本比对 UX 需 A5 配合（G1）
```

## 声明

全部 RESEARCH ONLY：未修改任何产品代码，未 push，仅在 `logs/research/M5-W18/` 写入/修订报告与 checkpoint。
