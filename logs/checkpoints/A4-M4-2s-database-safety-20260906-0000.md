# A4 — M4-2.s 数据库安全闸门 + M4-3 命令层（部分）

- **Lane**: A4
- **BASE**: `3544c09`（master；指挥板写 `f376346`，BASE 字段滞后 2 提交，与 A1 展开卡一致）
- **HEAD**: 未提交（按指挥板，仅 Lane A0 可 push；本 lane 交补丁 + 检查点）
- **状态**: `START → DONE`（本 lane 范畴内）

## 一、交付内容（仅 A4 范畴）

### 1. M4-2.s 安全闸门（纯函数，零 db 依赖）— `src-tauri/src/security_policy.rs`
- SQL 风险分类器：`classify_sql_risk` / `is_write_statement`（掩蔽注释+字符串+字面量后按结构判定；不可解析/多语句/超长即 fail-closed）。
- 生产判定：`ProductionSignals` + `is_production_database`（§3.3 信号契约）：无信号/域名不可解析/非加密公网 → `Unknown`（按生产处理）。
- fail-closed 写闸门：`require_write_confirmation`（生产/未知一律拒绝写；非生产需 `allow_write` 且二次确认）。
- 命令层组合纯函数：`evaluate_db_query_gate`（db_query 执行前必经）、`build_db_signals`、`host_is_loopback_or_private_ip`。
- 测试：**36** 个分类器/生产判定/写闸门用例（N-sql-1~16 失败用例 + 防 fail-closed 过头反向用例 + 5 个组合闸门用例）。

### 2. M4-3 命令层（Lane A4 Must Deliver）— `src-tauri/src/bridge.rs` + `main.rs` + `permissions/default-commands.toml`
- `db_connect` / `db_query` / `db_disconnect` 三个后端命令，注册进 `generate_handler!` 与 ACL（置于 `list_artifact_images` 之前）。
- `db_query` 在**任何语句执行前**调用 `evaluate_db_query_gate`；任一步拒绝即短路返回，绝不降级放行（F1/F4/G-4/G-5）。
- 凭据：password 永不进 `DbConnectionConfig`（F2）；仅在 `db_connect` 时写入系统密钥库，键 = `db:<conn_id>`（G-1，与 git 的 `repo_id` 命名空间隔离）；`db_query` 不传 password。
- 审计 detail 禁含 SQL 原文/凭据（G-3）：只记 `conn_id` / `kind` / `rows` / `truncated` / `field_truncated`。
- 连接模型：**按需重连**。`DbPool` 包裹 `rusqlite::Connection` 等驱动句柄（非 `Send`），不能驻留 Tauri 全局 state，故配置登记进 `DbConnectionRegistry`（Send 安全），每次 `db_query` 即时建连。`db_disconnect` 撤销登记并删密钥。SQLite 文件级、MySQL/PG 取数通道未实现（D27），该模型对当前可验证路径完全成立。

### 3. 静态门禁夹具 — `scripts/check-database-policy.py`（新增）
- 沿用 A6 `check-scheduler-policy.py` 范式：7 ACTIVE 码 + 8 pending 码，三模式（`--self-test` / 默认 / `--expect-pending`），含变异防呆与「产物存在才判」。
- 已接入 `scripts/pre-merge.sh`（文档清单 + 主流程 db 块 + 存在性自检）。

## 二、验证

| 项 | 结果 |
|---|---|
| `cargo test --manifest-path src-tauri/Cargo.toml` | **328 passed**（基线 232 + A4 新增 41：36 分类/生产/闸门 + 5 组合闸门） |
| `cargo test ... security_policy` | 72 passed（31 既有 + 41 新增） |
| `python3 scripts/check-database-policy.py --self-test` | PASS（2 好样本零违规 + 15 坏样本全检出） |
| `python3 scripts/check-database-policy.py`（默认） | `all invariants hold`（ACTIVE=7） |
| `python3 scripts/check-database-policy.py --expect-pending` | NONE（8 pending 码位未违规） |
| `git diff --check`（本 lane 全部改动） | 干净，无冲突标记 |
| `bash -n scripts/pre-merge.sh` | 语法 OK |

> 注：A7 此前有 `scheduler.rs` 半成品编译错误（与 A4 无关），现已修复；A4 代码独立编译通过。

## 三、挂账 / 待裁决（非 A4 范畴，交对应卡）

- **D27**：MySQL / PostgreSQL 取数通道未实现（无真实服务端可验证），`DbPool::query` 对二者显式返回 `DB_NOT_SUPPORTED`。属 M4-2.b，非本 lane。
- **db_cancel / db_forget_connection**：命令名见 M4-1.c §3（未冻结）/ D28（未裁决），未纳入本批次；`DbQueryResult.query_id` 已预留取消句柄位。
- **O-A4-1**：SQLite 本地文件若不显式给拓扑信号，按 §3.3 规则 4 判定为 `Unknown`（按生产处理 fail-closed）。契约歧义，待 A0 确认是否应视本地文件为回环。
- **db 配置持久化落盘**：本批次 `db_connect` 仅校验可达性 + 登记配置到内存 `DbConnectionRegistry`，**未**落 `data_dir/db_connections.json`。正式持久化（原子写 + 加载）属 M4-2 持久化，待 A3/A4 后续卡；当前按需重连模型不依赖持久化。

## 四、并发协作事实

- A3 `database.rs`：本批次早期为 0 字节空文件（违反 Batch Rule 5 临时态），执行末已落地 47KB，API 稳定（`DbPool::connect/query/close`、`credential_key`、`DB_*` 常量）。A3 明确把写闸门留给 A4（doc 注明 `query` 不判写），本 lane 据此接线。
- A5 `DatabasePanel.vue` / `useDatabaseStore.ts` / `dbUi.ts`：明确注释「db_connect/db_query/db_disconnect 尚未注册，等 A4」，DTO 字段名（`DbQueryResult` 的 `query_id/columns/rows/row_count/truncated/field_truncated`）与后端一致。前端 wiring 归 A5，未越界改动。
- 未触碰任何 Lane 范围外文件（`domain.rs`/`bridge.ts`/`types.ts`/`dbUi.ts`/scheduler/tasks 等）。
