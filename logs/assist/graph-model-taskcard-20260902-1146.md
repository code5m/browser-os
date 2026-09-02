# graph-model-taskcard（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 锚定：WBS §7.2 **M5-7 ~ M5-9**（需求 #13 知识图谱）
> 状态：⏸ **任务卡 / 未实现存储与查询 / 未执行验收 / 不宣称 PASS**
> 关联：`M5-13.a-prework-20260902-1055.md`（GraphNode/GraphEdge/邻接表/两阶段抽取）· `database-schema-taskcard-20260902-1146.md`（**共享存储选型决策**）

---

## 1. 目标

整理 M5 图谱数据模型卡：**节点/边/事件/来源、增量更新、查询接口、隐私过滤、容量限制、验收标准**。

---

## 2. 现状证据（2026-09-02 实测）

| 项 | 现状 |
|---|---|
| 图谱存储 | ❌ 无（`Cargo.toml` 无 SQLite / 图数据库依赖） |
| 图谱命令 | ❌ 59 个白名单命令中无 `graph_*` |
| 领域模型 | `domain.rs` 只有 `Artifact` / `AuditEntry` / `RepoConfig` |
| 上游数据来源 | ⚠️ `Artifact`（笔记/成果）已有；`ScriptMeta` / `ScheduledTask` / `SkillMeta` / `Agent` **均未落地** |
| 现有持久化 | JSON 文件（`workspace.rs`），非原子写 |
| 审计 | ✅ `log_audit`，1000 条上限 |

**关键依赖**：图谱是**派生索引层**——上游（Artifact / Script / Task / Skill / Agent）未落地时，图谱只能围绕 `Artifact` 建图（`M5-13.a` §依赖图已指出）。

---

## 3. 必改文件候选

| 文件 | 改动 | 必要性 |
|---|---|---|
| **新增** `src-tauri/src/graph/mod.rs` | 图谱模块入口 | 必须 |
| **新增** `src-tauri/src/graph/model.rs` | `GraphNode` / `GraphEdge` / `GraphEvent` / `GraphSource` | 必须 |
| **新增** `src-tauri/src/graph/store.rs` | 存储（SQLite 邻接表，见 §7 决策点） | 必须 |
| **新增** `src-tauri/src/graph/extract.rs` | 抽取器（两阶段） | 必须 |
| **新增** `src-tauri/src/graph/query.rs` | 查询接口 + 隐私过滤 | 必须 |
| `src-tauri/src/bridge.rs` | 新增 `graph_query` / `graph_neighbors` / `graph_stats` / `graph_rebuild` / `graph_prune` | 必须 |
| `src-tauri/permissions/default-commands.toml` | 新增命令（**K1**） | 必须 |
| `src-tauri/src/shutdown.rs` | 注册 `GraphShutdown`（关闭连接 / flush 待写） | 必须（依赖 TASK-10） |

---

## 4. 契约 / 数据结构

### 4.1 核心模型

```rust
/// 节点类型（枚举封闭，新增需发版）
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Artifact,      // 笔记/成果
    Repo,          // 仓库
    File,          // 文件
    Script,        // 脚本
    Task,          // 定时任务
    Skill,         // Skill
    Agent,         // Agent
    Person,        // 人（作者/联系人）
    Tag,           // 标签
    Url,           // 网址
    Term,          // 术语/实体（抽取得到）
}

/// 边类型
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Mentions,      // A 提到 B
    References,    // A 引用 B（链接）
    DerivesFrom,   // A 派生自 B
    AuthoredBy,    // A 由 B 创作
    BelongsTo,     // A 属于 B（如文件属于仓库）
    TriggeredBy,   // A 被 B 触发（Task → Script）
    Invokes,       // Agent/Skill → Script
    TaggedWith,    // A 打了标签 B
    SimilarTo,     // 相似度（带权重）
    CoOccurs,      // 共现（带权重）
}

/// 实体来源（可追溯是图谱的命脉）
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GraphSource {
    /// 来源类型
    pub kind: SourceKind,                 // Artifact|RepoFile|ScriptMeta|TaskMeta|SkillMeta|AgentLog|UserInput
    /// 来源实体 id（如 artifact id）
    pub ref_id: String,
    /// 原文片段定位（便于回到原文）
    pub locator: Option<String>,          // 如 "line:12-15" / "char:340-356"
    /// 抽取器版本（用于重抽与失效）
    pub extractor_version: String,
    /// 置信度 0.0~1.0
    pub confidence: f32,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,                       // 稳定 id：见 §4.2
    pub kind: NodeKind,
    pub label: String,                    // 展示名
    /// 类型专属属性（JSON）
    pub props: serde_json::Value,
    /// 来源（可多个：同一实体被多处提及）
    pub sources: Vec<GraphSource>,
    pub created_at: String,               // UTC ISO8601
    pub updated_at: String,
    /// 隐私分级，见 §4.6
    pub privacy: PrivacyLevel,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub id: String,
    pub kind: EdgeKind,
    pub from_id: String,
    pub to_id: String,
    /// 权重 0.0~1.0（SimilarTo/CoOccurs 用）
    pub weight: f32,
    pub props: serde_json::Value,
    pub sources: Vec<GraphSource>,
    pub created_at: String,
    pub updated_at: String,
    pub privacy: PrivacyLevel,
}

/// 图谱事件（用于增量更新与审计）
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GraphEvent {
    pub seq: u64,                         // 单调递增，增量消费的游标
    pub op: GraphOp,                      // UpsertNode|DeleteNode|UpsertEdge|DeleteEdge
    pub payload: serde_json::Value,
    pub at: String,                       // UTC ISO8601
    /// 触发来源（谁引起的变更）
    pub origin: String,                   // "artifact:saved" / "task:created" / "user:edit" / "rebuild"
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphOp { UpsertNode, DeleteNode, UpsertEdge, DeleteEdge }

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyLevel { Public, Internal, Private, Restricted }
```

### 4.2 稳定 id 规则（**必须遵守，否则增量更新会重复建点**）

```
node_id = "{kind}:{natural_key_sha256_16}"
```

| kind | natural_key |
|---|---|
| `artifact` | artifact.id |
| `repo` | repo_config.id |
| `file` | canonicalize(path) |
| `script` | script_id |
| `task` | task_id |
| `skill` | skill_id（**不含版本号**，避免升级即断裂） |
| `agent` | agent_id |
| `person` | 规范化后的名字（小写、去空格） |
| `tag` | 规范化后的标签（小写） |
| `url` | 规范化 URL（去 fragment、去 utm 参数、去尾斜杠） |
| `term` | 规范化术语文本 |

```rust
fn node_id(kind: &NodeKind, natural_key: &str) -> String {
    let mut h = Sha256::new();
    h.update(format!("{kind:?}:{natural_key}").as_bytes());
    format!("{:?}:{}", kind, hex::encode(&h.finalize()[..8])).to_lowercase()
}
```

### 4.3 增量更新

```
上游变更（artifact saved / task created / skill installed ...）
   → 产生 GraphEvent（append 到 graph_events 表，seq 自增）
   → 抽取器消费新事件（两阶段，见下）
   → Upsert 节点/边（按 node_id 幂等）
   → 更新 last_consumed_seq
```

| 项 | 规则 |
|---|---|
| 幂等 | 全部 `Upsert`（`INSERT ... ON CONFLICT DO UPDATE`），**不预删** |
| 断点续传 | 持久化 `last_consumed_seq`；重启从该点继续 |
| 删除传播 | 上游删除 → `DeleteNode` + 级联 `DeleteEdge`（FK `ON DELETE CASCADE`） |
| 重抽 | `extractor_version` 变化时，可对该来源的所有事件重抽（**不删旧结果，先 upsert 新结果**） |
| 批量 | 单次 rebuild 采用批量事务（每 500 条一事务） |

**两阶段抽取**（沿用 `M5-13.a` §两阶段）：

| 阶段 | 内容 | 是否需要模型 |
|---|---|---|
| **阶段一（确定性）** | 正则/解析：URL、文件路径、`#tag`、`@person`、代码引用、front-matter | ❌ 不需要，可离线 |
| **阶段二（语义）** | 实体识别、相似度、主题聚类 | ✅ 需要模型（**本卡只预留接口，不实现**） |

### 4.4 查询接口

```rust
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphQuery {
    /// 中心节点（邻居查询时必填）
    pub center: Option<String>,
    /// 跳数，1 或 2（>2 一律拒绝，见 §4.7）
    pub depth: Option<u8>,
    /// 节点类型过滤
    pub kind_filter: Option<Vec<NodeKind>>,
    /// 边类型过滤
    pub edge_filter: Option<Vec<EdgeKind>>,
    /// 关键字（label 模糊匹配）
    pub keyword: Option<String>,
    /// 最小权重
    pub min_weight: Option<f32>,
    /// 隐私上限（调用方权限决定，见 §4.6）
    pub max_privacy: Option<PrivacyLevel>,
    /// 分页
    pub limit: Option<u32>,      // 默认 100，上限 1000
    pub offset: Option<u32>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphResult {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    /// 是否因容量限制被截断
    pub truncated: bool,
    /// 因隐私被过滤掉的条目数（**告知而不展示**）
    pub filtered_by_privacy: u32,
}
```

| 命令 | 用途 |
|---|---|
| `graph_query` | 通用查询（§4.4） |
| `graph_neighbors` | 邻居查询（center + depth 必填） |
| `graph_stats` | 节点/边计数、按 kind 分布、最后更新 seq |
| `graph_rebuild` | 全量重建（**异步 + 进度事件**） |
| `graph_prune` | 按容量/时间裁剪（§4.7） |

### 4.5 存储（SQLite 邻接表，草案）

```sql
-- 在 database-schema-taskcard 的迁移体系中，图谱为独立迁移文件（如 003_graph.sql）
CREATE TABLE IF NOT EXISTS graph_node (
    id          TEXT PRIMARY KEY,
    kind        TEXT NOT NULL,
    label       TEXT NOT NULL,
    props       TEXT NOT NULL DEFAULT '{}',
    sources     TEXT NOT NULL DEFAULT '[]',   -- JSON 数组
    privacy     TEXT NOT NULL DEFAULT 'internal',
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_graph_node_kind ON graph_node(kind);
CREATE INDEX IF NOT EXISTS idx_graph_node_updated ON graph_node(updated_at DESC);

CREATE TABLE IF NOT EXISTS graph_edge (
    id          TEXT PRIMARY KEY,
    kind        TEXT NOT NULL,
    from_id     TEXT NOT NULL REFERENCES graph_node(id) ON DELETE CASCADE,
    to_id       TEXT NOT NULL REFERENCES graph_node(id) ON DELETE CASCADE,
    weight      REAL NOT NULL DEFAULT 1.0,
    props       TEXT NOT NULL DEFAULT '{}',
    sources     TEXT NOT NULL DEFAULT '[]',
    privacy     TEXT NOT NULL DEFAULT 'internal',
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    UNIQUE (kind, from_id, to_id)            -- 幂等的物理保证
);
CREATE INDEX IF NOT EXISTS idx_graph_edge_from ON graph_edge(from_id);
CREATE INDEX IF NOT EXISTS idx_graph_edge_to   ON graph_edge(to_id);

CREATE TABLE IF NOT EXISTS graph_event (
    seq         INTEGER PRIMARY KEY AUTOINCREMENT,
    op          TEXT NOT NULL,
    payload     TEXT NOT NULL,
    at          TEXT NOT NULL,
    origin      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS graph_meta (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL
);
-- graph_meta: ('last_consumed_seq', N), ('extractor_version', '1'), ('schema_version', '1')
```

**为什么邻接表而非图数据库**：规模小（见 §4.7 容量上限）、零额外依赖、与 M4 数据库共用同一 SQLite 实例。

### 4.6 隐私过滤

| 级别 | 含义 | 默认谁能看 |
|---|---|---|
| `public` | 可对外分享 | 全部 |
| `internal` | 本应用内可见（默认） | 本地用户 |
| `private` | 需显式解锁 | 本地用户 + 解锁 |
| `restricted` | 永不出应用、不进导出/同步/图谱可视化 | 仅原文查看 |

| 规则 | 说明 |
|---|---|
| 过滤层级 | **查询层强制过滤**，不依赖调用方自觉 |
| 上限判定 | 调用方声明 `max_privacy`，查询时 `privacy <= max_privacy` |
| 默认 | 未声明时 `max_privacy = internal` |
| 计数告知 | 被过滤的条数以 `filtered_by_privacy` 返回（**只给数量，不给内容**） |
| 导出 | 导出功能**必须**排除 `private` / `restricted` |
| Agent 可见性 | Agent 默认 `max_privacy = public`；需提升要显式授权（走 A2P/A2A 能力模型） |
| 密码/凭据 | ❌ 绝不进图谱（**K3**）；`restricted` 也只存引用 |
| 生效时机 | 节点/边的 `privacy` 取**两端与自身的最高者**（保守） |

### 4.7 容量限制（硬约束）

| 项 | 上限 | 超出行为 |
|---|---|---|
| 节点总数 | **50,000** | 拒绝新建并提示先 prune |
| 边总数 | **200,000** | 同上 |
| 单节点边数（度） | **500** | 新建边时拒绝并提示 |
| 查询返回节点 | 1,000（`limit` 上限） | `truncated = true` |
| 查询跳数 `depth` | **≤ 2** | >2 一律拒绝（避免全图遍历） |
| 单次 `rebuild` | 批量 500 / 事务 | 进度事件上报 |
| 事件表保留 | 90 天 或 100,000 条 | 老的归档到文件后删除 |
| `sources` 数组长度 | 20 / 实体 | 超出合并为「+N 处来源」 |

**红线**：`depth > 2` 直接拒绝，**不做**「尽力返回」（会导致 UI 卡死与内存爆炸）。

### 4.8 审计（**K5**）

| 事件 | 落哪里 |
|---|---|
| `graph_rebuild` 开始/结束 | `audit.json`（低频） |
| `graph_prune` | `audit.json` |
| 隐私过滤命中（`restricted` 被访问尝试） | `audit.json`（**安全事件，必须记**） |
| 每次查询 | ❌ **不记**（高频，会刷爆） |
| 每次事件消费 | ❌ 不记 |

---

## 5. 实现要点（步骤化）

1. **先拍板存储选型**（与 `database-schema-taskcard` 的 D1 是同一个决策）。
2. **依赖 M4 数据库**（`graph_*` 表作为独立迁移文件挂进同一 SQLite）。
3. `graph/model.rs`：§4.1 全部结构 + `node_id()` 稳定 id 函数。
4. `graph/store.rs`：邻接表 CRUD + 幂等 upsert + `last_consumed_seq` + 批量事务。
5. `graph/extract.rs`：**只实现阶段一**（确定性抽取）；阶段二留接口 `trait SemanticExtractor` 且不实现。
6. `graph/query.rs`：§4.4 查询 + **强制隐私过滤**（§4.6）+ 容量硬约束（§4.7）。
7. `bridge.rs` 加 5 个命令 → `default-commands.toml`（**K1**）。
8. 注册 `GraphShutdown`（flush 待写事件 + 关闭连接）。
9. 前端可视化（**本卡不涉及**）。

---

## 6. 禁止事项

| # | 禁止 | 原因 |
|---|---|---|
| 1 | ❌ 在存储选型未拍板前引入图数据库/SQLite | 与 M4 共用同一决策 |
| 2 | ❌ 节点 id 用随机 uuid | 增量更新会重复建点 |
| 3 | ❌ 允许 `depth > 2` 的查询 | 全图遍历卡死 |
| 4 | ❌ 查询层不做隐私过滤（只靠 UI） | 可被直接调命令绕过 |
| 5 | ❌ 把凭据/密码写进 `props` | K3 |
| 6 | ❌ 每次查询/事件消费写 `audit.json` | 刷爆 1000 条上限（K5） |
| 7 | ❌ 删除上游实体时只删节点不删边 | 悬挂边 → 查询崩溃 |
| 8 | ❌ 阶段二（语义抽取）在本卡实现 | 需要模型能力，先只留接口 |
| 9 | ❌ 新增命令忘进 ACL | K1 |

---

## 7. 风险

| 级别 | 风险 | 缓解 |
|---|---|---|
| **高** | **存储选型未决策**（与 M4 D1 同一决策） | 未拍板前不得开工；两处共用结论 |
| **高** | 上游数据模型（Script/Task/Skill/Agent）均未落地 → 图谱只能围绕 Artifact 建图 | 接受「先 Artifact-only 建图」，但**必须**在文档中标注图谱当前是不完整的（`M5-13.a` 已指出） |
| 中 | 孤儿节点 / 悬挂边 | FK `ON DELETE CASCADE` + 定期 `graph_prune` 巡检 |
| 中 | 抽取质量差（阶段一正则误报） | `confidence` 字段 + 来源可追溯（`locator` 回到原文） |
| 中 | 隐私级别误判导致泄露 | 保守取最高；导出强制排除；Agent 默认 `public` |
| 中 | 容量增长失控 | §4.7 硬上限 + prune |
| 低 | 邻接表在 2 跳查询上的性能 | 节点上限 5 万 + 边索引，足够；超规模再评估 |

---

## 8. 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 同一 artifact 保存 3 次 | 只产生 1 个节点（幂等 upsert） |
| R2 | 删除一个 artifact | 其节点 + 相关边全部消失（无悬挂边） |
| R3 | 重启后继续消费事件 | 从 `last_consumed_seq` 继续，不重复、不遗漏 |
| R4 | 查询 `depth = 5` | 拒绝，给出可读错误 |
| R5 | 查询命中 `restricted` 节点 | 不返回内容；`filtered_by_privacy` 计数 +1 + 审计 |
| R6 | 以 Agent 身份查询 | 默认只看 `public` |
| R7 | 导出图谱 | 不含 `private` / `restricted` |
| R8 | 构造 60000 个节点 | 达到 50000 上限后拒绝新建并提示 prune |
| R9 | 单节点构造 600 条边 | 超过度上限 500 后拒绝 |
| R10 | `graph_rebuild` 中途取消 | 已提交批次保留，可续跑；无半截数据 |
| R11 | `sources` 超过 20 条 | 合并为「+N 处来源」，不无限增长 |
| R12 | 高频查询 1000 次 | `audit.json` 不爆（查询不记审计） |
| R13 | 阶段二未实现时调用 | 返回明确的「未实现」错误，不静默返回空 |
| R14 | URL 规范化 | `a.com/x?utm=1#f` 与 `a.com/x` 视为同一节点 |
| R15 | 宿主退出时有待写事件 | 走 `GraphShutdown` flush，不丢 |

---

## 9. 验收命令（**均未执行**）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# 9.1 模块存在
ls src-tauri/src/graph/                      # 期望 model.rs / store.rs / extract.rs / query.rs

# 9.2 稳定 id（非随机 uuid）
grep -n "fn node_id" src-tauri/src/graph/model.rs          # 期望命中
grep -n "Uuid::new_v4" src-tauri/src/graph/model.rs | wc -l  # 期望 0

# 9.3 depth 硬约束
grep -n "MAX_DEPTH\|depth > 2\|depth > MAX" src-tauri/src/graph/query.rs   # 期望命中

# 9.4 隐私过滤在查询层
grep -n "max_privacy\|privacy <=" src-tauri/src/graph/query.rs             # 期望命中
grep -n "filtered_by_privacy" src-tauri/src/graph/query.rs                 # 期望命中

# 9.5 容量上限
grep -n "50000\|MAX_NODES\|200000\|MAX_EDGES\|MAX_DEGREE" src-tauri/src/graph/store.rs  # 期望命中

# 9.6 幂等 upsert + 级联删除
grep -n "ON CONFLICT\|ON DELETE CASCADE" src-tauri/src/graph/store.rs      # 期望命中

# 9.7 审计不刷爆（K5）
grep -rn "log_audit" src-tauri/src/graph/*.rs | wc -l                      # 期望：极少量

# 9.8 命令已进 ACL（K1）
grep -c "graph_query\|graph_neighbors\|graph_stats\|graph_rebuild\|graph_prune" \
  src-tauri/permissions/default-commands.toml                              # 期望 5

# 9.9 退出收口
grep -n "GraphShutdown" src-tauri/src/shutdown.rs                          # 期望命中

# 9.10 编译门槛
cargo clippy --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20
# 对照 baseline 13 warning
```

---

## 10. 失败动作

| 失败 | 动作 |
|---|---|
| 存储选型未拍板就开工 | 停止，等人工决策（与 M4 D1 同一决策） |
| 节点 id 不幂等（重复建点） | 修 `node_id()`；不得靠「rebuild 时先清空」 |
| 查询绕过隐私过滤 | 视为 P0 隐私缺陷，立即修复（过滤必须在查询层） |
| 出现悬挂边 | 补 FK 级联 + prune 巡检 |
| `depth>2` 未拒绝 | 补硬约束；不得「先跑跑看」 |
| `audit.json` 被刷爆 | 移除查询审计（K5） |
| 容量失控 | 补硬上限 + prune；不得移除上限 |
| clippy warning 增加 | 对照 baseline 回退 |

---

## 11. 推荐模型

- 数据模型与存储：`AI:DEEP`
- 阶段一确定性抽取：`AI:BALANCED`
- 阶段二语义抽取（未来）：`AI:DEEP-xhigh` + 人工评估
- **人工验收必做**：R5/R6/R7（隐私）、R8/R9（容量）、R10（rebuild 取消）
