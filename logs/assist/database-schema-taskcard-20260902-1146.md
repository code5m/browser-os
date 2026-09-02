# database-schema-taskcard（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 锚定：WBS §6 **M4-1 ~ M4-4**（需求 #6 数据库等功能）
> 状态：⏸ **只写文档 / 未执行任何迁移 / 未连任何数据库 / 不宣称 PASS**
> 关联：`M4-14.a-prework-20260902-1055.md`（拆解 + 迁移回滚范式 §5-bis）· `M5-13.a-prework-20260902-1055.md`（图谱，共享存储选型）

---

## 1. 目标

整理 M4 数据库任务卡：**schema 草案、迁移策略、回滚、版本记录、备份、兼容旧数据、验收脚本草案**。
🚨 **本卡只写文档，不执行迁移**。

---

## 2. 现状证据（2026-09-02 实测）

| 项 | 现状 |
|---|---|
| 数据库依赖 | ❌ `src-tauri/Cargo.toml` 中**无** `rusqlite` / `sqlx` / `diesel` |
| 数据库命令 | ❌ 59 个白名单命令中无 `db_*` |
| 现有持久化 | ✅ 纯 JSON 文件（`workspace.rs`）：`workspace/*.json`、`repos.json`、`audit.json` |
| 现有领域模型 | `Artifact` / `AuditEntry` / `RepoConfig`（`domain.rs`，92 行） |
| 凭据存储 | ✅ `keyring_store.rs`（keyring 3），`DbConnConfig` **不含**密码字段（K3） |
| 原子写 | ❌ 现有 `fs::write` **非原子**（`save_artifact` / `save_repos`） |
| 两段式安全闸门范式 | ✅ `src-tauri/src/bridge.rs:688-790` |
| 退出收口 | ❌ 无 `ShutdownCoordinator` → 连接池关闭无处接入（依赖 TASK-10） |

### 2.1 现有落盘文件清单

| 文件 | 内容 | 写入方式 |
|---|---|---|
| `<app_data>/mvp-browser-os/workspace/<id>.json` | 单个 Artifact | `fs::write`（非原子） |
| `<app_data>/mvp-browser-os/repos.json` | `Vec<RepoConfig>` | `fs::write`（非原子） |
| `<app_data>/mvp-browser-os/audit.json` | `Vec<AuditEntry>`，1000 条上限 | `fs::write`（非原子） |

---

## 3. 必改文件候选（**本卡只写文档**）

| 文件 | 性质 | 说明 |
|---|---|---|
| **新增** `src-tauri/migrations/001_init.sql` | 迁移脚本 | 首次 schema |
| **新增** `src-tauri/migrations/00N_*.sql` | 后续迁移 | 每次一条，**永不修改已落地的迁移** |
| **新增** `src-tauri/src/db.rs` | 连接管理 + `MIGRATIONS` 列表 + 版本记录 | 实现阶段 |
| **新增** `src-tauri/src/db_migrate.rs` | 迁移执行器 + 回滚 | 实现阶段 |
| `src-tauri/src/domain.rs` | 新增 DB 相关结构（**不含密码字段**） | 实现阶段 |
| `src-tauri/Cargo.toml` | 加 `rusqlite`（bundled，决策点 §7） | 实现阶段 |
| `src-tauri/src/shutdown.rs` | 注册 `DbShutdown` | 实现阶段 |
| **新增** `scripts/db-verify.sh` | 验收脚本草案（**本卡只写草案**） | 本卡产出 |

---

## 4. schema 草案

### 4.1 设计原则

| 原则 | 说明 |
|---|---|
| 兼容旧数据 | `Artifact` 从 JSON 迁移，**新字段一律 `NOT NULL DEFAULT`**，避免迁移失败 |
| 凭据分离 | **表中绝不出现密码列**；凭据只存 keyring，表内只存 `credential_ref` |
| 版本可溯 | `schema_meta` 表记录当前版本、每次迁移的 applied_at / checksum |
| 幂等 | 每个迁移用 `CREATE TABLE IF NOT EXISTS` / `ALTER TABLE ADD COLUMN`（SQLite 无 `IF NOT EXISTS` 于 ADD COLUMN，需先查 `PRAGMA table_info`） |
| 外键 | 显式 `PRAGMA foreign_keys = ON` |
| 时区 | 全部存 **UTC**（ISO 8601 文本或 unix 秒），展示层转本地 |

### 4.2 `001_init.sql`（草案，**不执行**）

```sql
-- 001_init.sql
-- 目标：承载 Artifact / Repo / Audit 三类既有数据 + 后续 M4/M5 扩展
-- 约定：所有时间存 UTC（TEXT ISO8601 或 INTEGER unix 秒）；禁止存明文凭据

PRAGMA foreign_keys = ON;

-- ---------- 元数据 ----------
CREATE TABLE IF NOT EXISTS schema_meta (
    version     INTEGER PRIMARY KEY,        -- 迁移版本号，单调递增
    name        TEXT    NOT NULL,           -- 迁移名（文件名主体）
    checksum    TEXT    NOT NULL,           -- 迁移文件 sha256，防被篡改后静默重放
    applied_at  TEXT    NOT NULL            -- UTC ISO8601
);

-- ---------- 成果库（来自既有 workspace/*.json） ----------
CREATE TABLE IF NOT EXISTS artifact (
    id          TEXT PRIMARY KEY,           -- 与既有 JSON 文件名一致，保证迁移无损
    kind        TEXT NOT NULL DEFAULT 'note',
    title       TEXT NOT NULL DEFAULT '',
    content     TEXT NOT NULL DEFAULT '',
    source_url  TEXT,
    created_at  TEXT NOT NULL,              -- UTC ISO8601
    updated_at  TEXT NOT NULL,
    -- 兼容旧数据：既有 JSON 里可能存在但模型未声明的字段，统一放 extra
    extra       TEXT NOT NULL DEFAULT '{}'  -- JSON 文本
);
CREATE INDEX IF NOT EXISTS idx_artifact_created_at ON artifact(created_at DESC);

-- ---------- 仓库配置（来自既有 repos.json，不含 token） ----------
CREATE TABLE IF NOT EXISTS repo_config (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    local_path  TEXT NOT NULL,
    remote_url  TEXT,
    branch      TEXT NOT NULL DEFAULT 'main',
    -- K3：凭据只存 keyring，这里只存引用
    credential_ref TEXT,                    -- 形如 "keyring:<service>:<user>"，可为 NULL
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

-- ---------- 审计（来自既有 audit.json，上限策略见 §4.6） ----------
CREATE TABLE IF NOT EXISTS audit_entry (
    seq         INTEGER PRIMARY KEY AUTOINCREMENT,
    at          TEXT NOT NULL,              -- UTC ISO8601
    action      TEXT NOT NULL,
    detail      TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_audit_at ON audit_entry(at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_action ON audit_entry(action);

-- ---------- 后续里程碑预留（本卡只建表，不实现逻辑） ----------
-- M4-5~M4-8 定时任务
CREATE TABLE IF NOT EXISTS scheduled_task (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    cron            TEXT NOT NULL,
    timezone        TEXT NOT NULL DEFAULT 'UTC',
    script_id       TEXT NOT NULL,
    params_json     TEXT NOT NULL DEFAULT '[]',
    enabled         INTEGER NOT NULL DEFAULT 1,
    misfire_policy  TEXT NOT NULL DEFAULT 'skip',
    retry_json      TEXT NOT NULL DEFAULT '{}',
    timeout_sec     INTEGER NOT NULL DEFAULT 60,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    last_run_at     TEXT,
    next_run_at     TEXT
);

CREATE TABLE IF NOT EXISTS task_run (
    run_id          TEXT PRIMARY KEY,
    task_id         TEXT NOT NULL REFERENCES scheduled_task(id) ON DELETE CASCADE,
    idempotency_key TEXT NOT NULL UNIQUE,   -- 幂等的物理约束
    scheduled_at    TEXT NOT NULL,
    started_at      TEXT NOT NULL,
    finished_at     TEXT,
    status          TEXT NOT NULL,          -- running|success|failed|cancelled|timeout
    attempt         INTEGER NOT NULL DEFAULT 1,
    exit_code       INTEGER,
    duration_ms     INTEGER,
    trigger         TEXT NOT NULL,          -- scheduled|manual|catch_up|retry
    error           TEXT
);
CREATE INDEX IF NOT EXISTS idx_task_run_task ON task_run(task_id, started_at DESC);
```

### 4.3 数据字典要点

| 表 | 关键约束 | 理由 |
|---|---|---|
| `schema_meta` | `checksum` | 防迁移文件被改后重放造成分叉 |
| `artifact.id` | 沿用原 JSON 文件名 | 保证迁移**零丢失**、可回滚到 JSON |
| `artifact.extra` | JSON 文本 | 兼容旧数据里未知的额外字段（**K**：`#[serde(default)]` 的 DB 等价物） |
| `repo_config.credential_ref` | 可为 NULL | **K3**：表内绝无密码 |
| `task_run.idempotency_key` | `UNIQUE` | 幂等由**数据库约束**兜底，不只靠应用层 |
| `audit_entry` | 无 FK | 审计是只追加的独立事实表 |

---

## 5. 迁移策略

### 5.1 版本号与命名

```
src-tauri/migrations/
├── 001_init.sql
├── 002_add_artifact_tags.sql
└── 003_xxx.sql
```

| 规则 | 说明 |
|---|---|
| 命名 | `NNN_<snake_case>.sql`，`NNN` 三位零填充 |
| 不可变 | **已落地的迁移文件永不修改**；要改就写新迁移 |
| 顺序 | 按 `NNN` 升序执行；跳号即报错 |
| 事务 | 每个迁移在**单事务**内执行；失败整体回滚该迁移 |
| 记录 | 成功后写 `schema_meta`（version/name/checksum/applied_at） |

### 5.2 执行器流程（草案）

```
open db
  → PRAGMA foreign_keys = ON
  → 读 schema_meta 得到 current_version
  → 扫描 migrations/，筛出 version > current_version 的，按序
  → 每个迁移：
       1) 校验 checksum（若该版本已 applied 且 checksum 不符 → 中止并报警，不自动修）
       2) BEGIN
       3) 执行 SQL
       4) INSERT INTO schema_meta(...)
       5) COMMIT
       失败 → ROLLBACK → 中止（保留现场，不自动回滚到旧版本）
  → 完成
```

### 5.3 回滚策略

| 层 | 策略 |
|---|---|
| **单迁移内** | 事务自动回滚（SQLite DDL 支持事务） |
| **跨迁移回滚** | **不做自动 down 迁移**。理由：down 迁移易与数据不兼容，风险高于收益 |
| **兜底** | **全量备份 + 时间点恢复**（见 §6） |
| **紧急回退** | 恢复到迁移前的备份文件，并在 `schema_meta` 手动校正版本（人工操作，需留审计） |

**红线**：不得因为「加个 down 脚本更快」而实现自动 down 迁移；本项目的恢复手段是备份。

### 5.4 破坏性变更的正确姿势（SQLite）

`ALTER TABLE` 能力有限（不支持 `DROP COLUMN` 老版本、不支持改约束）。标准做法是**重建表**：

```sql
-- 模板：重建表（expand → migrate → contract）
BEGIN;
  -- 1) 建新表
  CREATE TABLE artifact_new ( ... 新结构 ... );
  -- 2) 搬数据
  INSERT INTO artifact_new (id, kind, title, content, created_at, updated_at, extra)
    SELECT id, kind, title, content, created_at, updated_at, extra FROM artifact;
  -- 3) 换名
  DROP TABLE artifact;
  ALTER TABLE artifact_new RENAME TO artifact;
  -- 4) 重建索引
  CREATE INDEX idx_artifact_created_at ON artifact(created_at DESC);
COMMIT;
```

**要点**：整个流程在事务内；`legacy_compatibility` 依赖 `INSERT ... SELECT` 的显式列列表（避免列顺序漂移）。

### 5.5 兼容旧数据（JSON → SQLite）

```
迁移日（一次性，附在 001_init 之后的 002_import_json.sql 或独立导入命令）：
  1) 扫描 <app_data>/mvp-browser-os/workspace/*.json
  2) 逐个 serde_json::from_str::<serde_json::Value>()（**不**用强类型 Artifact）
       → 原因：旧 JSON 可能缺字段；用 Value 可保留全部字段进 extra
  3) 取已知字段映射到列；其余整体进 extra
  4) INSERT OR IGNORE（幂等，可重复执行）
  5) 导入后**不删除**原 JSON，改名为 *.json.imported 作为回滚依据
     （保留策略：保留 30 天或用户手动清理）
  6) repos.json / audit.json 同理
```

**红线（沿用既有经验）**：`domain.rs` 中 `Artifact` 的新字段**必须** `#[serde(default)]`，否则历史 JSON 会被 `load_artifacts` 静默丢弃（`workspace.rs:53` 的 `if let Ok` 吞错）。DB 侧对应做法：列必须 `NOT NULL DEFAULT`。

---

## 6. 备份

| 项 | 策略 |
|---|---|
| 时机 | ① **每次迁移前**（强制）② 每日首次启动（可选）③ 用户手动 |
| 方式 | SQLite `VACUUM INTO '<backup>'`（**优于**文件拷贝：保证一致性快照） |
| 命名 | `mvp-<YYYYMMDD-HHMMSS>-v<version>.db` |
| 位置 | `<app_data>/mvp-browser-os/backups/` |
| 保留 | 最近 **10 份** + 最近 7 天各 1 份；超出按时间淘汰 |
| 校验 | 备份后 `PRAGMA integrity_check`；失败即报警 **不删除**原库 |
| 恢复 | 关闭连接 → 备份当前库为 `.broken` → 复制备份回位 → 重启校验 |

```sql
-- 一致性备份（推荐）
VACUUM INTO '/path/to/backups/mvp-20260902-114600-v1.db';
```

**禁止**：直接 `fs::copy` 正在被写入的 db 文件（可能拿到撕裂的页）。

---

## 7. 风险

| 级别 | 风险 | 缓解 |
|---|---|---|
| **高** | **存储选型未决策**：`rusqlite` bundled vs 系统 sqlite vs 不引 DB | `M5-13.a`（图谱）也需要 SQLite，两处共用同一决策。**未决策前不得开工**，应先出选型结论（见 §9 决策点） |
| 高 | 无 `ShutdownCoordinator` → 连接池无处关闭 | 依赖 TASK-10 |
| 中 | JSON → SQLite 导入丢字段 | 用 `serde_json::Value` + `extra` 兜底；导入后做**条目数 + 字段抽样**比对 |
| 中 | 迁移中途崩溃 | 单迁移单事务；**迁移前强制备份** |
| 中 | `checksum` 校验过严导致合法修改被拒 | 报警而非自动修；人工确认后手动更新 `schema_meta` |
| 中 | `bundled` 编译耗时长 / 体积增大 | `rusqlite` bundled 约 +1~2 MB；对照 `logs/baseline-2026-08-27.md` 评估 |
| 低 | 审计表无限增长 | 保留策略：>90 天或 >10 万条转存归档文件 |

---

## 8. 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 空目录首次启动 | 建库 + 应用 001（及后续）+ `schema_meta` 有记录 |
| R2 | 已迁移库再次启动 | 跳过已应用迁移，幂等，无重复执行 |
| R3 | 手动篡改某迁移文件内容后启动 | `checksum` 不符 → **中止并报警**，不自动修 |
| R4 | 迁移 SQL 语法错误 | 事务回滚，库停留在旧版本，应用给出可读错误（不静默） |
| R5 | 旧 `workspace/*.json` 缺字段 | 导入成功，缺失字段用默认值，`extra` 保留未知字段 |
| R6 | 旧 JSON 有未知额外字段 | 进 `extra`，**不丢** |
| R7 | 导入过程被中断（kill -9） | 重启后可重跑导入（`INSERT OR IGNORE`），**不产生重复** |
| R8 | 备份文件用 `PRAGMA integrity_check` | `ok` |
| R9 | 从备份恢复 | 数据完整，应用正常启动 |
| R10 | 凭据列检查 | 全库 `SELECT` 不到任何密码明文（**K3**） |
| R11 | 并发启动两个实例（若可能） | 有锁保护，后者等待或报可读错误，不损坏库 |
| R12 | 退出时有未提交事务 | 走 `DbShutdown` 优雅关闭，无 `-wal` 残留膨胀 |

---

## 9. 决策点（**必须先拍板，否则无法开工**）

| # | 决策 | 选项 | 影响 |
|---|---|---|---|
| D1 | **是否引入 SQLite** | ① `rusqlite` bundled ② 系统 sqlite 动态链接 ③ 继续用 JSON | 影响 M4-1~4 与 M5-7~9（图谱） |
| D2 | 存储范围 | ① 全部迁 SQLite ② 仅新增数据用 SQLite，既有 JSON 保持 | 影响迁移工作量与回滚复杂度 |
| D3 | 迁移时机 | ① 首次启动自动迁移 ② 用户手动触发 | 影响故障影响面 |
| D4 | 备份保留策略 | 10 份 / 7 天（本卡建议） | — |

> 本卡给出**建议**：D1=①（bundled，与图谱共享）、D2=②（渐进，风险最低）、D3=①（自动，但强制先备份）、D4=本卡建议值。
> **但决策权在人工**，未拍板前不得开工。

---

## 10. 验收脚本草案（`scripts/db-verify.sh`，**只写不执行**）

```bash
#!/usr/bin/env bash
# scripts/db-verify.sh —— M4 数据库验收脚本草案（本批次仅产出草案，未执行）
# 用途：检查什么 / 输入 / 预期退出码 / 失败信息 / 是否可 CI 化
set -uo pipefail

DB="${1:?用法: db-verify.sh <path-to-mvp.db>}"
FAILURES=0

fail() { echo "FAIL: $*"; FAILURES=$((FAILURES+1)); }
pass() { echo "PASS: $*"; }

# --- 检查项 1：库完整性 ---
# 检查什么：SQLite 物理完整性
# 输入：db 路径
# 预期退出码：0；失败信息："integrity_check 非 ok"
# 是否可 CI 化：✅ 可（无需 GUI）
ic=$(sqlite3 "$DB" "PRAGMA integrity_check;" 2>/dev/null)
[ "$ic" = "ok" ] && pass "integrity_check" || fail "integrity_check 非 ok: $ic"

# --- 检查项 2：迁移版本连续 ---
# 检查什么：schema_meta.version 从 1 连续无跳号
# 预期退出码：0；失败信息："版本不连续"
# 是否可 CI 化：✅ 可
vers=$(sqlite3 "$DB" "SELECT version FROM schema_meta ORDER BY version;" 2>/dev/null)
expected=$(seq 1 "$(echo "$vers" | wc -l)" | tr '\n' ' ')
[ "$(echo "$vers" | tr '\n' ' ')" = "$expected" ] && pass "迁移版本连续" \
  || fail "迁移版本不连续: [$vers] 期望 [$expected]"

# --- 检查项 3：无明文凭据列（K3）---
# 检查什么：所有表的列名不含 password/secret/token
# 预期退出码：0；失败信息："发现疑似凭据列"
# 是否可 CI 化：✅ 可
bad=$(sqlite3 "$DB" "SELECT name FROM sqlite_master WHERE type='table';" 2>/dev/null \
  | while read -r t; do
      sqlite3 "$DB" "PRAGMA table_info($t);" 2>/dev/null \
        | cut -d'|' -f2 | grep -iE "password|passwd|secret|token" | sed "s/^/$t./"
    done)
[ -z "$bad" ] && pass "无明文凭据列" || fail "发现疑似凭据列: $bad"

# --- 检查项 4：Artifact 条目数对账 ---
# 检查什么：JSON 目录下的 .json 数 == artifact 表行数
# 输入：第二个参数为 workspace 目录
# 预期退出码：0；失败信息："导入丢失 N 条"
# 是否可 CI 化：✅ 可（需给定目录）
if [ "${2:-}" ]; then
  json_n=$(ls -1 "$2"/*.json 2>/dev/null | wc -l)
  db_n=$(sqlite3 "$DB" "SELECT COUNT(*) FROM artifact;" 2>/dev/null)
  [ "$json_n" = "$db_n" ] && pass "条目数一致 ($db_n)" \
    || fail "条目数不一致: json=$json_n db=$db_n"
fi

# --- 检查项 5：幂等键唯一约束存在 ---
# 检查什么：task_run.idempotency_key 上有 UNIQUE 索引
# 预期退出码：0；失败信息："缺少幂等唯一约束"
# 是否可 CI 化：✅ 可
uq=$(sqlite3 "$DB" "SELECT COUNT(*) FROM sqlite_master WHERE type='index' \
  AND sql LIKE '%idempotency_key%' AND sql LIKE '%unique%';" 2>/dev/null)
[ "$uq" -ge 1 ] && pass "幂等唯一约束存在" || fail "缺少幂等唯一约束"

# --- 检查项 6：备份可恢复（人工）---
# 检查什么：VACUUM INTO 的备份文件 integrity_check 为 ok
# 是否可 CI 化：⚠️ 半可（需要构造临时库）

echo "----"
[ "$FAILURES" -eq 0 ] && { echo "ALL PASS"; exit 0; }
echo "$FAILURES 项失败"; exit 1
```

### 10.1 脚本清单（草案）

| # | 脚本 | 检查什么 | 输入 | 预期退出码 | 失败信息 | 可 CI 化 |
|---|---|---|---|---|---|---|
| 1 | `scripts/db-verify.sh` | 完整性 / 版本 / 凭据列 / 条目对账 / 幂等约束 | db 路径 + workspace 目录 | 0 | 逐项 `FAIL:` 前缀 | ✅ |
| 2 | `scripts/db-migrate-dryrun.sh` | 在**临时副本**上跑迁移，验证不破坏 | 源 db 路径 | 0 | 迁移错误详情 | ✅ |
| 3 | `scripts/db-backup-check.sh` | 备份存在性 + `integrity_check` + 保留数量 | backups 目录 | 0 | 缺失/损坏 | ✅ |
| 4 | `scripts/db-rollback-drill.sh` | 从备份恢复到临时位置并校验 | 备份文件 | 0 | 恢复失败 | ✅ |
| 5 | `scripts/db-perf-baseline.sh` | 关键查询耗时基线（1 万条 artifact 下的列表查询） | db 路径 | 0 | 超过阈值 | ⚠️ 需固定机器 |
| 6 | `scripts/db-audit-scan.sh` | 扫描全库是否出现疑似凭据明文 | db 路径 | 0 | 命中行 | ✅ |

> ⚠️ 以上脚本**均未创建、未执行**；`sqlite3` CLI 是否在目标环境可用需先确认（否则改用 Rust 侧 `rusqlite` 实现）。

---

## 11. 验收命令（**均未执行**）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# 11.1 尚未引入 DB 依赖（本卡结束时仍应如此）
grep -c "rusqlite\|sqlx\|diesel" src-tauri/Cargo.toml     # 期望 0

# 11.2 迁移目录尚未创建（本卡只写文档）
ls src-tauri/migrations 2>&1 | head -1                     # 期望 No such file

# 11.3 无 db 命令进 ACL（尚未实现）
grep -c "db_" src-tauri/permissions/default-commands.toml  # 期望 0

# 11.4 现有 JSON 落盘点（迁移时的对账基准）
ls -1 src-tauri/src/*.rs | xargs grep -ln "fs::write"      # 期望命中 workspace.rs

# 11.5 凭据不在模型中（K3）
grep -n "password\|secret\|token" src-tauri/src/domain.rs | wc -l   # 期望 0

# 11.6 实现后的验收（未来）
# bash scripts/db-verify.sh <db> <workspace-dir>
# bash scripts/db-migrate-dryrun.sh <db>
# bash scripts/db-backup-check.sh <backups-dir>
```

---

## 12. 失败动作

| 失败 | 动作 |
|---|---|
| D1 存储选型未拍板 | **停止**，等人工决策；不得擅自 `cargo add rusqlite` |
| 迁移失败 | 事务回滚 → 库留旧版本 → **不自动修** → 报警并附备份路径 |
| 导入条目数不一致 | 定位丢失条目（对比 id 集合）→ 修复导入逻辑 → 重跑（`INSERT OR IGNORE` 幂等） |
| 备份 `integrity_check` 失败 | 保留现场、**不删除**原库，人工介入 |
| 发现明文凭据列 | 视为 P0 安全缺陷，立即整改（K3） |
| 迁移文件被篡改（checksum 不符） | 中止并报警，**不自动修正** `schema_meta` |

---

## 13. 推荐模型

- 选型决策（D1~D4）：`AI:DEEP-xhigh` + **人工拍板**
- schema 与迁移实现：`AI:DEEP`
- 验收脚本实现：`AI:BALANCED`
- **人工验收必做**：R5/R6（旧数据兼容）、R9（备份恢复）、R10（凭据扫描）
