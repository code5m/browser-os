# A9 M5-8 图谱存储与查询契约草案（Batch Implementation Dispatch 交付）

> 生成：2026-09-06 07:00 CST · Lane A9（M5 prework only · docs only）
> 性质：可照着实现的**契约草案**。零产品代码（未触 `src/`、`src-tauri/`、`scripts/pre-merge.sh`）。
> 锚定事实：本文件所有「已冻结」陈述均来自对当前工作树 M4 落盘代码的只读核对（见 §0 事实基线），非推断。
> 配套：`logs/assist/A9-M5-split-20260905-2359.md`（§7 指向本批）· `A9-M5-A13plus-cards-20260906-0010.md`（A17 卡细化）· `A9-M5-graph-scheduler-feed-20260906-0700.md`（维护任务接缝）。

---

## 0. 事实基线（2026-09-06 07:00 实测，只读核对）

| 项 | 已落地事实（文件:行） | 对 M5-8 的意义 |
|---|---|---|
| 取数边界 | `src-tauri/src/database.rs:477` `DbPool::query(&mut self, sql, &QueryCancel, timeout_secs: Option<u64>, query_id: &str) -> Result<DbQueryResult, DbError>` | 图查询若走用户连接池，可**直接复用**此边界 + 单语句约束 |
| 结果 DTO | `database.rs:151` `DbQueryResult{columns, rows, row_count, truncated, field_truncated, elapsed_ms, query_id}`；`database.rs:123` `DbValue{Null,Bool,I64,F64,Text,Binary{bytes}}` | 图查询 DTO 须与其字段语义一致，便于复用前端序列化 |
| 限制常量 | `database.rs:41` `DB_MAX_ROWS=1000`；`:43` `DB_MAX_RESULT_BYTES=4MiB`；`:45` `DB_MAX_TEXT_FIELD_BYTES=65536`；`:39` `DB_MAX_SQL_BYTES=65536`；`:47` `DB_DEFAULT_QUERY_TIMEOUT_SECS=30`；`:49` `DB_MAX_QUERY_TIMEOUT_SECS=600` | 图查询硬约束直接复用这些常量 |
| 取消 | `database.rs:172` `QueryCancel`（pub，L1 原子标志）；`database.rs:54` `DB_CANCEL_CHECK_EVERY_ROWS=64` | 图大查询可复用 `QueryCancel` |
| 限流实现 | `database.rs:579` `struct QueryLimiter`（**私有，非 pub**）；`database.rs:635` `query_sqlite_with_deadline`（**pub(crate)**） | **关键缺口**：M5 图存储若要复用 A3 的行/字节截断，必须让 A3 提升该 API（见 §7 G-D2） |
| 凭据命名空间 | `database.rs:66` `DB_CRED_PREFIX="db:"`；`:69` `credential_key(conn_id)` | 图库为 app 自有派生数据，**不应**占用用户 Keyring 命名空间（见 §2 决策） |
| 配置结构 | `domain.rs:853` `DbConnectionConfig`（结构性**不含** password/dsn 字段，凭据只经 Keyring） | 图库**不**经 `DbConnectionConfig` 持久化（绕开用户连接体系） |
| 凭据红线 | F2/F3 同上；审计 `src-tauri/src/workspace.rs:386` `log_audit(app, action, detail)`，审计 detail 不得含凭证/DSN/SQL 正文 | 图审计复用同一函数，detail 仅记稳定标签 |
| 原子写 | `src-tauri/src/session.rs:30` `atomic_write(path, content)`（temp+rename） | 图 schema 版本/迁移标记文件复用此原语 |
| 路径策略 | `database.rs:328` `validate_sqlite_path(database, roots)` | 图库路径由 app 自定（data_dir 下），不经用户 roots 校验 |
| ACL 末条 | `src-tauri/permissions/default-commands.toml` 尾条恒为 `list_artifact_images`；`graph_*` 命中 0 | 图命令一律插 `list_artifact_images` 之前（坑位②） |
| 迁移机制 | `database.rs` 内**无** CREATE TABLE 迁移框架；`execute_batch` 仅出现在测试夹具 | 图 schema 迁移须自建轻量版本文件，不依赖 A3 |

> 上述事实以 `85d2d7b` 为基准（本地 `master` 领先 `origin/master` 2）；M4 代码当前在工作树未提交、但与 A9 无文件交集（A9 仅写 `logs/assist/`）。

---

## 1. 总目标与边界（M5-8 / M5-7）

把 `Artifact`/`ScriptMeta`/`Bookmark`/`TaskDef`/`SkillDef`/`RepoConfig` 等散落资产抽取为「实体—关系—属性」图谱，作为**派生索引层**（主数据不复制，实体 `ref_id` 回指既有领域对象）。赋能 M5-9 Agent 问答上下文注入、影响分析、关联浏览。

**不引重依赖**：首期 SQLite + 邻接表，禁 Neo4j 等（沿用 `详细设计与实施计划.md` §7.2 红线）。

---

## 2. 存储位置决策（G-D1，交 A0 裁定）

| 方案 | 描述 | 取舍 |
|---|---|---|
| A（推荐） | 独立 SQLite 文件 `data_dir/graph/graph.db`，由 M5 `graph.rs` 模块**直接** `rusqlite::Connection::open` 打开；不经 `DbPool`、不经 `DbConnectionConfig`、不经 Keyring | 图是 app 自有派生数据，无凭据、无需用户连接体系；但需自建取数限流/取消（复用 `QueryCancel` + 提升 `QueryLimiter`，§7 G-D2） |
| B | 复用用户 `db_connect` 连接池（把同一 SQLite 注册为一条 `conn_id`，走 `db_query`） | 自动获得 A3 限流/截断/审计；但**污染用户连接命名空间**、图库与用户库耦合、且 `DbPool::query` 当前 SQLite 实现已就绪但用户写闸门逻辑会套到图查询上（不合理） |

**建议**：A。理由：图查询是只读派生索引，不应进入"用户数据库连接 + fail-closed 写闸门"体系；但取数限流/取消**必须**与 A3 同源，避免口径漂移。G-D1 交 A0 在 M4 PASS 后裁定 A/B。

---

## 3. Schema 草案（DDL，挂同一 SQLite 实例，独立迁移文件）

> 文件命名建议 `src-tauri/src/graph/migrations/0001_graph.sql`（M5 实施期，A9 仅定契约）。
> 全部以 `ref_id` 回指既有领域对象；`ON DELETE CASCADE` 由**应用层**而非外键保证（既有 `ScriptMeta` 等无对应父表在此库）。

```sql
-- 节点
CREATE TABLE graph_node (
  id          TEXT PRIMARY KEY,         -- 稳定 id（§4）
  kind        TEXT NOT NULL,            -- artifact|script|bookmark|task|skill|repo|command|tag|person|url
  ref_id      TEXT,                      -- 回指既有领域对象 id（如 Artifact.id / ScriptMeta.id / TaskDef.id）
  label       TEXT NOT NULL,
  props       TEXT NOT NULL,            -- JSON，属性键值；隐私字段脱敏后入
  sources     TEXT NOT NULL DEFAULT '[]',-- JSON array，抽取来源集合，≤20（容量红线）
  degree      INTEGER NOT NULL DEFAULT 0,
  updated_at  TEXT NOT NULL
);
CREATE INDEX idx_node_kind ON graph_node(kind);
CREATE INDEX idx_node_ref ON graph_node(ref_id);

-- 边
CREATE TABLE graph_edge (
  id          TEXT PRIMARY KEY,         -- 稳定 id（§4）
  src         TEXT NOT NULL REFERENCES graph_node(id) ON DELETE CASCADE,
  dst         TEXT NOT NULL REFERENCES graph_node(id) ON DELETE CASCADE,
  relation    TEXT NOT NULL,            -- links_to|references|triggers|imports|mentions|depends_on|owned_by
  weight      REAL NOT NULL DEFAULT 1.0,
  source      TEXT NOT NULL,            -- rule | ai（阶段二语义抽取来源标记）
  created_at  TEXT NOT NULL
);
CREATE INDEX idx_edge_src ON graph_edge(src);
CREATE INDEX idx_edge_dst ON graph_edge(dst);
CREATE INDEX idx_edge_relation ON graph_edge(relation, src);

-- 事件（增量抽取队列）
CREATE TABLE graph_event (
  seq         INTEGER PRIMARY KEY AUTOINCREMENT,
  kind        TEXT NOT NULL,            -- node_upsert | edge_upsert | node_delete | edge_delete
  payload     TEXT NOT NULL,            -- JSON，待消费事件体
  created_at  TEXT NOT NULL
);

-- 元数据（schema 版本 + 抽取游标）
CREATE TABLE graph_meta (
  key         TEXT PRIMARY KEY,
  value       TEXT NOT NULL
);
-- 初始：schema_version=1；last_event_seq=0
```

**容量红线（硬约束，实施期断言）：**

| 维度 | 上限 | 超限行为 |
|---|---|---|
| 节点 | 50,000 | `graph_build` 拒绝继续 upsert，记 `over_cap` 且 catalog 标记 `degraded` |
| 边 | 200,000 | 同上 |
| 单节点度 | 500 | 超限边丢弃并记 `degree_cap` |
| `sources` 集合 | 20 | 截断并记 `too_many_sources` |
| 事件表 | 100,000 行 或 90 天 | 滚动裁剪最旧（FIFO） |

> 上限常量建议在 M5 落地时定在 `domain.rs` 图专有段（仿 `SCHED_MAX_*`/`DB_MAX_*`），避免散落（口径漂移红线）。

---

## 4. 稳定 id 规则（幂等，禁随机 uuid）

```
id = "{kind}:{sha256(natural_key)[:16]}"
natural_key:
  artifact  = artifact.id            (或 source_url 归一)
  script    = script.id
  bookmark  = bookmark.url
  task      = task.id
  skill     = skill.id
  repo      = repo.id
  command   = snippet.id
  tag       = "#" + tag_text
  person    = "@" + handle
  url       = url 归一（去尾斜杠/scheme 小写）
```

- 重抽/增量依赖稳定 id 做 upsert（同 id → 覆盖 props，追加 source 到 `sources`）。
- 边 id = `sha256(src|dst|relation)[:16]`（同关系去重）。

---

## 5. 事件与两阶段抽取

```
上游变更（Artifact/Repo/Script/Task/Skill 落库）→ 应用发 GraphEvent（app 内事件，不经 scheduler）
  → 阶段一(确定性 rule)：规则/正则抽取 URL/路径/#tag/@person/code-ref/front-matter → upsert 节点/边(source=rule)
  → 阶段二(语义 ai)：仅预留 trait SemanticExtractor，结果标 source=ai 可人工校正；触发 LLM 出网必须走 audit+确认闸门+Keyring（§7 红线）
```

- 阶段二触发 LLM 调用：属"出网"动作，必须经 `security_policy::check_invocation_source` 同款确认闸门 + `log_audit`（action=`graph_ai_extract`）；审计 detail 仅记 task/模型稳定标签，禁记抽取文本正文（隐私）。
- 增量消费：读取 `graph_event` 未消费行（`seq > last_event_seq`），逐条 upsert，更新 `graph_meta.last_event_seq`。**失败幂等**：同一 seq 可重放。

---

## 6. 查询契约（GraphQuery → GraphQueryResult）

复用 A3 `DbQueryResult` 字段语义（§0），新增图谱专属字段。

```rust
// 提案（M5 定在 domain.rs 图段；非现有类型）
pub struct GraphQuery {
    pub start: Vec<String>,     // 起始节点 id（1..N）
    pub relations: Vec<String>, // 空 = 任意 relation
    pub kinds: Vec<String>,     // 空 = 任意 kind
    pub max_depth: u8,          // 拒绝 >2（硬约束）
    pub limit: usize,           // 拒绝 >1000，默认 200
    pub privacy: String,        // 查询层隐私过滤层；Agent 默认 "public"
    pub cancel: QueryCancel,    // 复用 A3 公开取消句柄
}

pub struct GraphQueryResult {
    pub nodes: Vec<GraphNodeView>,
    pub edges: Vec<GraphEdgeView>,
    pub truncated: bool,        // 行/字节上限命中（复用 DB_MAX_ROWS/DB_MAX_RESULT_BYTES 口径）
    pub field_truncated: bool,
    pub elapsed_ms: u64,
    pub query_id: String,
}
```

**查询约束（硬）：**
- `max_depth > 2` → 拒绝（`GRAPH_DEPTH_EXCEEDED`），避免 N² 爆炸。
- `limit > 1000` → 拒绝（`GRAPH_LIMIT_EXCEEDED`）。
- 隐私过滤在**查询层**：`privacy` 低于节点 props 标记的最严格级别时剔除该节点；Agent 默认 `public`，凭据/私密配置节点永不进入 `public` 结果（K3）。
- `truncated` 标志沿用 M4-1.c §2 语义：超限**不是错误**，但禁止静默截断，UI 须显式提示。

---

## 7. 关键缺口与开放项（交 A0 / 实施期）

| ID | 缺口 | 依赖方 | 处置 |
|---|---|---|---|
| G-D1 | 图库存储位置 A vs B（§2） | A0 裁定 | M4 PASS 后裁定；A9 建议 A |
| G-D2 | `QueryLimiter`（`database.rs:579`）与 `query_sqlite_with_deadline`（`:635`）当前为私有/pub(crate)，M5 图存储要复用行/字节截断须 A3 提升为 `pub` 或提供 `pub fn limit_rows(rows) -> DbQueryResult` | A3 | 提 A0 指派 A3 在 M5-8 前做最小 API 暴露 |
| G-D3 | 图查询审计低频：仅 `graph_rebuild`/`graph_prune`/`restricted 越权访问` 记 `audit.json`；每次查询/事件消费**不记**（K5 1000 上限） | A17 | 实施期写 `log_audit` 调用，detail 不含 props 全文 |
| G-D4 | 图 schema 迁移框架：当前 A3 无迁移机制，图库需自建 `graph_meta.schema_version` + 版本文件（§3） | A17 | 实施期用 `atomic_write` 写迁移标记 |
| G-D5 | `check-graph-policy.py` 政策脚本（仿 `check-database-policy.py`/`check-scheduler-policy.py`）挂 `pre-merge.sh` 同位置 | A9 不写脚本，交 A0 指派 | 仅定义检查点：depth≤2 拒绝、limit≤1000、稳定 id 无 uuid、凭据不进图谱 |

---

## 8. 命令草案（均插 `list_artifact_images` 之前，进入 ACL）

| 命令 | 用途 | 备注红线 |
|---|---|---|
| `graph_build()` | 触发全量重建（增量扫描上游） | 低频审计；可超时（复用 `DB_MAX_QUERY_TIMEOUT_SECS`） |
| `graph_query(q: GraphQuery) -> GraphQueryResult` | 子图查询 | depth≤2/limit≤1000 硬拒 |
| `graph_neighbors(id) -> GraphQueryResult` | 一跳邻居（含 relation 过滤） | 限流复用 |
| `graph_stats() -> GraphStats` | 节点/边计数、容量状态、degraded 标记 | 只读 |
| `graph_rebuild()` | 重放 `graph_event` 重建 | 低频审计；超长 rebuild 不得被下一 tick 顶掉（§ scheduler feed 文档） |
| `graph_prune()` | 裁剪过期事件/超限降级恢复 | 低频审计；tick 内禁写审计 |

前端：`src/types.ts` 镜像 `GraphQuery`/`GraphQueryResult`/`GraphNodeView`/`GraphEdgeView`（逐字对齐 `serde rename_all="snake_case"`）；`src/bridge.ts` 包装；面板见 M5-9（A19 卡）。

---

## 9. 测试清单（契约测试点，供 A17 实施期落地）

- `T-graph-id1`：稳定 id 幂等（同 natural_key 两次生成 id 一致，非 uuid）。
- `T-graph-id2`：`sources` 超 20 截断并记 `too_many_sources`。
- `T-graph-cap1`：节点达 50,000 后 upsert 拒绝并标记 `degraded`。
- `T-graph-cap2`：单节点度 >500 边丢弃。
- `T-graph-q1`：`max_depth=3` 拒绝 `GRAPH_DEPTH_EXCEEDED`。
- `T-graph-q2`：`limit=1001` 拒绝 `GRAPH_LIMIT_EXCEEDED`。
- `T-graph-q3`：大结果 `truncated=true` 且 `row_count ≤ DB_MAX_ROWS(1000)`。
- `T-graph-priv1`：Agent(`privacy=public`) 查询不返回私密配置节点。
- `T-graph-audit1`：单次 `graph_query` **不**新增 `audit.json` 条目；`graph_rebuild` 新增且仅 1 条。
- `T-graph-cancel1`：复用 `QueryCancel`，取消命中返回 `Cancelled`。
- `T-graph-event1`：同 `seq` 重放幂等（upsert 覆盖不新增重复节点）。

---

## 10. FORBID 遵守记录

- 未写产品代码；未触 `src/`、`src-tauri/`、`scripts/pre-merge.sh` 及三份主文档。
- 未移动 `NEXT`（现 M4 batch implementation mode）。
- 未提交、未 push；本文件为新增独立文档，与工作树中 A3/A4/A6/A7/A10/A11 未提交产物无交集。
- 所有「已冻结」陈述均以来源 file:line 标注；「提案/开放项」明确区分。
