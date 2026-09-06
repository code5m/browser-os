# A7 · M5-W3 图谱实现就绪卡（M5-7/M5-8 落地规范）

> LANE=A7 · WAVE=M5-W3（SUPPORT DOCS ONLY）· 责任实现 Lane=**A17**（M5-7/8 卡指定）
> 性质：把 A1 的 `M5-7-graph-model-extract.md` / `M5-8-graph-store-query.md` 预研卡升级为**实现就绪规范**；A7 不写产品代码（W3 硬停：仅 A2/A3 可动产品代码）
> 配套真相源：`logs/assist/A9-M5-graph-store-contract-20260906-0700.md`（DDL）、A2 `M5-1.b` 卡（seam）、A3 `check-mcp-policy.py`（政策脚本范式）、A7 M4-7 冻结（shutdown 序）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W3（行 133-168）→ A7=`SUPPORT DOCS ONLY`；Must Deliver=精确 DTO/schema + 阻塞项；无图谱产品代码

---

## 0. 本卡定位

- 本卡是 A7 在 M5-W3 的**整包交付**：图谱模型/存储的"实现就绪规范"。实施由 A17 在 A0 后续 dispatch 承接。
- 不修改 A1 的 `M5-7`/`M5-8` 卡（避免与 A1 冲突）；本卡为 A7 自有、自包含的实现就绪输入。
- 给出：精确 DTO/schema（对齐 M5-W2 常量）、`core/` 纯函数拆分、AGRAPH_1 政策脚本规范、精确 shutdown 序、阻塞项与状态。

---

## 1. 仓库实况（@ `98a3b01`，已 `git pull --ff-only`）

| 项 | 状态 | 证据 |
|---|---|---|
| M5-W1 core 边界门 | 已集成（854bc40） | `check-core-boundary.py` 自测+默认均 PASS（ACTIVE=7，core 文件=2：`keyring_store.rs`+`mod.rs`） |
| M5-W2 常量集中 | 已落 `domain.rs` | `MAX_TIMEOUT_SECS=600`（1015）、`HARD_GRACE_SECS=5`（1017）、`MAX_TEXT_FIELD_BYTES=64*1024`（1019）、`DB_MAX_TEXT_FIELD_BYTES=64*1024`（1036）；字面值被单测 T-db-c6 锁定（1695-1719） |
| A3 M5-2 政策门 | 已集成 | `scripts/check-mcp-policy.py`（22KB）；`capability.rs` 由 A2/A3 W3 落地中 |
| M5-1.b（A2）seam + B 类搬入 | W3 `START`，**未 PASS** | `core/` 仅 `keyring_store.rs`+`mod.rs`，`seam.rs` 未落地；M5-1.b §9 行 237 明示"M5-7/8 必须 M5-1.b PASS 后才能做" |
| `database.rs` | 已落地（48KB） | 暴露 `Connection::open` / `pub fn connect` / `pub fn query` → `graph_store` 复用单连接路径**现已可用（非阻塞）** |
| 图谱产品代码 | 零 | `domain.rs` 无 `Graph*` 类型；`bridge.rs` 0 条 `graph_` 命令；无 `graph*.rs`；无 `check-graph-policy.py` → 与 W3 docs-only 一致 |

---

## 2. 精确 DTO / schema（实现期落 `domain.rs`）

### 2.1 模型类型（复用 M5-W2 常量）

```rust
// 图谱局部常量：须与 M5-W2 集中常量保持相等不变量（仿 DB_MAX_TEXT_FIELD_BYTES == MAX_TEXT_FIELD_BYTES）
pub const GRAPH_PROPS_MAX_BYTES: usize = MAX_TEXT_FIELD_BYTES; // 64*1024；单测守"须 == MAX_TEXT_FIELD_BYTES"
pub const GRAPH_LABEL_MAX_BYTES: usize = 256;
pub const GRAPH_MAX_DEPTH: u8 = 4;            // 查询深度硬上限（防爆栈）
pub const GRAPH_QUERY_LIMIT: usize = 1000;   // 返回条数硬上限（防响应体爆）
pub const GRAPH_EXTRACTOR_VERSION: &str = env!("CARGO_PKG_VERSION"); // semver，触发再抽取判定

pub enum GraphSource { Manual, Extract, Ai, Imported } // Ai 首期仅留 trait，不实现
pub struct GraphNode {
    pub id: String,                    // sha256(source_ref || kind || stable_props_hash)
    pub kind: String,                  // 节点类型（注册表必填）
    pub label: String,                 // ≤ GRAPH_LABEL_MAX_BYTES
    pub props: serde_json::Value,      // ≤ GRAPH_PROPS_MAX_BYTES；禁存主数据正文（K7）
    pub source: GraphSource,
    pub source_ref: Option<String>,   // 仅引用原文路径，不复制
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub extractor_version: String,
}
pub struct GraphEdge {
    pub id: String,
    pub from_id: String, pub to_id: String,
    pub kind: String,                  // 关系类型（注册表必填）
    pub props: serde_json::Value,
    pub source: GraphSource,
    pub source_ref: Option<String>,
    pub weight: f64,                   // 0.0~1.0（首期 1.0 精确 / 0.0 否定）
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
pub struct GraphSchema {
    pub version: String,
    pub node_kinds: Vec<String>,
    pub edge_kinds: Vec<String>,
    pub installed_at: DateTime<Utc>,
}
```

### 2.2 `core/` 纯函数拆分（关键：满足 `COREBOUND_*`）

| 模块 | 位置 | 性质 | 注入 seam | 进 `core/`？ |
|---|---|---|---|---|
| `graph_model` | `core/graph_model.rs` | 类型 + `id_hash` + `schema_validate` | 无（纯） | ✅ A 类 |
| `graph_normalize` | `core/graph_normalize.rs` | 阶段② 同 hash 合并 + 关系归一 | 无（纯） | ✅ A 类 |
| `graph_extract_doc` | `core/graph_extract_doc.rs` | 阶段① md/txt/json 解析 | 取 `&[u8]`/`&str` 内容 | ✅ A 类 |
| `graph_extract_html` | `core/graph_extract_html.rs` | 阶段① HTML 结构化扫描 | 取 `&Path` + `&dyn RootsProvider`（仅扫 `allowed_roots` 内） | ✅ B 类（用 seam） |
| `graph_store` | `core/graph_store.rs` | 邻接表读写 + 索引 + 备份 | 取 `&Path` + 路径校验 | ✅ B 类 |
| `graph_query` | `core/graph_query.rs` | 多跳/深度/过滤（纯计算） | 无 | ✅ A 类 |
| `graph.rs`（编排） | `src-tauri/src/graph.rs` | `graph_extract` 命令 handler：接 seam + 调 core 纯函数 + 写 store + emit | 用 `AppHandle`/`emit`/source-check | ❌ bin 侧 |

原则：`core/` 内模块**绝不** `use tauri` / `use crate::bridge` / `use AppHandle` / 起进程；只接收 `PathBuf`/`&str`/`&dyn RootsProvider`/`&dyn PathResolver`/`&dyn ProgressSink` 入参。→ 复用 `check-core-boundary.py`（AGRAPH_1 并入 `COREBOUND_*`）双守门。这是 W1 delta「图谱纯函数入 core」的**具体落点**。

### 2.3 查询/存储 DTO（与 M5-8 §4.1/§4.2 对齐，硬上限改常量）

```rust
pub enum GraphDirection { Out, In, Both }
pub struct GraphQuery {
    pub start: Vec<String>,
    pub direction: GraphDirection,
    pub edge_kinds: Vec<String>,        // 空 = 不限
    pub max_depth: u8,                   // 实施期截断到 GRAPH_MAX_DEPTH(4)
    pub limit: usize,                   // 实施期截断到 GRAPH_QUERY_LIMIT(1000)
    pub cursor: Option<String>,
}
pub struct GraphResult { pub nodes: Vec<GraphNode>, pub edges: Vec<GraphEdge>, pub truncated: bool }
pub struct GraphSubgraph { pub nodes: Vec<GraphNode>, pub edges: Vec<GraphEdge> } // 不返回 props 正文（K7）
pub struct GraphNodeUpsert { pub node: GraphNode, pub force: bool }  // force=false 时不得覆盖 source=Manual
pub struct GraphEdgeUpsert { pub edge: GraphEdge }
```

DDL **严格引用** A9 `graph-store-contract`（M5-8 §4.1），`schema_version` 走 `session::atomic_write`（M3 范式）；FTS5 虚拟表同 DDL。

### 2.4 `graph_store` 复用 `database.rs` 单连接

- 打开 `graph.db`：复用 `database.rs::DbPool::Sqlite(conn)` + `Connection::open(path)`，路径经同一 root 策略校验。
- **禁止**另起 `rusqlite` 句柄 / 第二 sqlite 访问原语 → 与 M4「无第二执行路径」一致。
- `vacuum` / `backup` / `fts-rebuild` 维护任务走 `script_runner`（复用 `TaskKind::{Script,Command}`，`enabled=false` 默认，**禁**新增 `TaskKind::GraphMaintenance`）。

---

## 3. AGRAPH_1 政策脚本规范（`scripts/check-graph-policy.py`）

仿 `check-mcp-policy.py` / `check-core-boundary.py`：三模式（`--self-test` / 默认 / `--expect-pending`）。A17 实施期创建并挂 `pre-merge.sh`。

| 码 | 断言 | 来源 |
|---|---|---|
| `AGRAPH_1` | `core/graph_*` 不 import `tauri`/`AppHandle`/`crate::bridge`/起第二执行路径（复用 `check-core-boundary` 判据） | `COREBOUND_*` |
| `AGRAPH_2` | `GraphNode.props`/`label` 字节 ≤ `GRAPH_PROPS_MAX_BYTES`/`GRAPH_LABEL_MAX_BYTES`（= `MAX_TEXT_FIELD_BYTES`） | K7/体量 |
| `AGRAPH_3` | `max_depth` ≤ `GRAPH_MAX_DEPTH`(4) | 防爆栈 |
| `AGRAPH_4` | `limit` ≤ `GRAPH_QUERY_LIMIT`(1000) | 防响应爆 |
| `AGRAPH_5` | `graph_*` 9 命令均在 ACL 且插 `list_artifact_images` 之前（K1） | ACL 序 |
| `AGRAPH_6` | 无 `TaskKind::GraphExtract`/`GraphMaintenance`（A6 冻结） | 调度冻结 |
| `AGRAPH_7` | `graph-store-shutdown` 注册在 `stop-scheduler` 之前（§4 序） | 关闭序冻结 |
| `AGRAPH_8` | `graph_node_upsert` 不得覆盖 `source=Manual`（用户数据红线） | FORBID |

pending→default：W3 期 `AGRAPH_*` 全 `PENDING`（图谱未实现）；A17 实现后转 `DEFAULT`（同 `check-database-policy.py` 模式）。

---

## 4. 精确 shutdown 序（与 A7 冻结一致）

`ShutdownCoordinator` 7 任务序（M5-1.b §5#12）基础上，`graph-store-shutdown` 必须插在 `stop-scheduler` **之前**、`kill-running-scripts` 之前；`mcp-server-shutdown` 在 `stop-scheduler` 之后（M5-2 §4.5）。合并精确序：

```
stop-background-workers → flush-sessions → close-tabs → kill-terminals → shutdown-grid
  → graph-store-shutdown → stop-scheduler → mcp-server-shutdown → kill-running-scripts → a2a-shutdown
```

断言：`graph-store-shutdown < stop-scheduler < kill-running-scripts`（AGRAPH_7 单测）。与 A7 M4-7 冻结（`stop-scheduler` 注册在 `stop-background-workers` 之后）**无冲突**。

---

## 5. 命令清单（9 条，全 source-check + ACL 前 `list_artifact_images`）

`graph_query` / `graph_node_get` / `graph_node_list` / `graph_node_upsert` / `graph_node_delete` / `graph_edge_list` / `graph_edge_upsert` / `graph_edge_delete` / `graph_export`（首期返回"暂不支持"）。

四件套同落（M4 护栏）：`bridge.rs` handler + `main.rs` 注册 + `types.ts`/`bridge.ts` 镜像 + `default-commands.toml` 插 `list_artifact_images` 之前。

---

## 6. 阻塞项与状态（B1..B5）

| ID | 阻塞项 | 状态 | 责任 | 解锁条件 |
|---|---|---|---|---|
| **B1** | M5-1.b seam（`PathResolver`/`RootsProvider`/`ProgressSink`）+ B 类搬入 PASS | W3 进行中（A2 `START`），**未 PASS** | A2 + A10 复审 | A2 落 M5-1.b 且 `check-core-boundary.py` 仍 PASS、A10 复审转 PASS → 图谱路径解析/seam 注入可用 |
| B2 | `domain.rs` `Graph*` 类型 + `GRAPH_*` 常量新增 | 待 A17 实施期加 | A17 | 实施期直接加（**非前驱阻塞**） |
| B3 | `check-graph-policy.py`（`AGRAPH_1..8`）创建 | 待 A17 | A17 | 实施期按 §3 建 |
| B4 | ACL 9 条目 + `bridge`/`types` 镜像 | 待 A17 | A17 | 实施期按 §5 加 |
| B5 | `graph-store-shutdown` 序注册 + 单测 | 待 A17 | A17 | 实施期按 §4 加 |

**结论**：A7 不写产品代码；唯一外部前驱阻塞 = **B1（M5-1.b）**，其余皆 A17 实施期内自闭环。

---

## 7. 自测（本准备卡实测，@ `98a3b01`）

| # | 验证 | 结果 |
|---|---|---|
| T1 | `check-core-boundary.py --self-test` | PASS（ACTIVE=7，坏样本全检） |
| T2 | `check-core-boundary.py` 默认 | PASS（core 文件=2） |
| T3 | `grep Graph` `domain.rs` | 0（无图谱类型，待 A17） |
| T4 | `grep graph_` `bridge.rs` | 0（无图谱命令） |
| T5 | `ls graph*.rs` | 无（W3 docs-only 正确） |
| T6 | M5-W2 常量命中 | `MAX_TIMEOUT_SECS`/`HARD_GRACE_SECS`/`MAX_TEXT_FIELD_BYTES` 命中（1015/1017/1019） |
| T7 | M5-7/M5-8 卡存在 | 是（A1 W0） |

→ 本准备卡与仓库状态一致，无冲突，零产品代码。

---

## 8. 输出模板回填

```text
LANE=A7
STATUS=PASS_WITH_DOCS
WAVE=M5-W3 (SUPPORT DOCS ONLY)
BASE=98a3b01
HEAD=docs only（A7-M5-graph-core-W3-impl-card-20260906-1412.md + A7-M5-graph-core-W3-checkpoint-20260906-1412.md）
FILES=logs/assist/A7-M5-graph-core-W3-impl-card-20260906-1412.md, logs/assist/A7-M5-graph-core-W3-checkpoint-20260906-1412.md
VERIFY=T1-T7 全 PASS/命中；无产品代码
CHECKPOINT=logs/assist/A7-M5-graph-core-W3-checkpoint-20260906-1412.md
MERGE_NOTES=W3 docs-only；未改 M5-7/8 卡（避免与 A1 冲突），本卡为 A7 自有实现就绪规范；唯一前驱阻塞 B1=M5-1.b（A2 W3 进行中）；
           与 A2 M5-1.b §9 行237、A3 check-mcp-policy.py 范式、M5-8 §4.5、A7 M4-7 冻结（stop-scheduler 序）一致
NEXT=A0 待 M5-1.b PASS 后发 M5-7/8 实施 dispatch 给 A17；本卡作为实现就绪输入
```
