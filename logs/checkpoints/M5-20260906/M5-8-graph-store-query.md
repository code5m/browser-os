# M5-8 图存储与查询（邻接表 + DDL + `graph_*` 命令）

> 子卡 ID：**M5-8** · 需求 #13 · `[S3|LEVERAGE:3|COMPLEX|AI:DEEP|R:xhigh]`
> 责任 Lane 候选：**A17**
> 父卡：`详细设计与实施计划.md` L571
> 主预研：`logs/assist/A9-M5-graph-store-contract-20260906-0700.md`（DDL + GraphQuery DTO）· `logs/assist/A9-M5-graph-scheduler-feed-20260906-0700.md`
> 配套：`M5-7-graph-model-extract.md`（抽取）· `M5-9-graph-ui-agent-consume.md`（UI 消费）

---

## 0. 编号与锚定

- 批次任务号 `M5-8`；需求号 #13；WBS L571 一致。
- 依赖：M5-7 ✅（schema 冻结）+ A3 DB API ✅
- 决策交 A0 拍：① 首期只读 vs 支持写 ② `graph_export("graphml")` 是否首期做

---

## 1. GOAL

实现图存储层（邻接表 + 索引 + 备份）、图查询 DTO（多跳邻居 / 路径 / 子图 / 标签过滤）与 `graph_*` 命令集合；首期**用户手建 + 自动抽取**（抽写），**不开第三方任意边写入**（防污染）。

---

## 2. READ

1. `logs/assist/A9-M5-graph-store-contract-20260906-0700.md`（**全读**）
2. `logs/assist/A9-M5-graph-scheduler-feed-20260906-0700.md`（**全读**）
3. `src-tauri/src/database.rs`（1272 行）
4. `M5-7-graph-model-extract.md` §4.1（schema）
5. `src-tauri/src/workspace.rs`（路径 + 备份目录）
6. `M5-1-core-workspace-split.md` §10

---

## 3. WRITE

| 文件 | 性质 | 说明 |
|---|---|---|
| `src-tauri/src/graph_store.rs` | **新增** | 邻接表读写 + 索引 + 备份 |
| `src-tauri/src/graph_query.rs` | **新增** | `GraphQuery` 多跳邻居 + 深度限制 + 路径查找 |
| `src-tauri/src/graph_maintenance.rs` | **新增** | 维护任务（vacuum / 备份 / FTS 重建） |
| `src-tauri/src/domain.rs` | 新增类型 | `GraphQuery` / `GraphResult` / `GraphSubgraph` / `GraphPath` / `GraphNodeUpsert` / `GraphEdgeUpsert` |
| `src-tauri/src/bridge.rs` | 修改 | `graph_query` / `graph_node_get/list/upsert/delete` / `graph_edge_list/upsert/delete` / `graph_export`（**全进 ACL**） |
| `src-tauri/permissions/default-commands.toml` | 修改 | 插 `graph_*` 共 9 条于 `list_artifact_images` 之前 |
| `src-tauri/src/scheduler.rs` | 修改 | 注册 `graph-maintenance` 周期任务（**`enabled=false` 默认**） |
| `src-tauri/src/shutdown.rs` | 修改 | 注册 `graph-store-shutdown`（**先于 `stop-scheduler` 关闭**） |
| `src-tauri/src/graph.db` `graph.db.bak` | 持久化 | SQLite + 备份 |
| `src/types.ts` | 新增 | TS 镜像 |

---

## 4. 关键契约

### 4.1 DDL（与 A9 graph-store-contract 一致；此处引用，**禁止漂移**）

```sql
CREATE TABLE graph_node (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL, label TEXT NOT NULL,
    props TEXT NOT NULL, source TEXT NOT NULL,
    source_ref TEXT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    extractor_version TEXT
);
CREATE INDEX idx_graph_node_kind ON graph_node(kind);
CREATE INDEX idx_graph_node_source ON graph_node(source);
CREATE VIRTUAL TABLE graph_node_fts USING fts5(label, content='graph_node', content_rowid='rowid');

CREATE TABLE graph_edge (
    id TEXT PRIMARY KEY,
    from_id TEXT NOT NULL REFERENCES graph_node(id),
    to_id TEXT NOT NULL REFERENCES graph_node(id),
    kind TEXT NOT NULL, props TEXT NOT NULL, source TEXT NOT NULL,
    source_ref TEXT, weight REAL NOT NULL DEFAULT 1.0,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE INDEX idx_graph_edge_from ON graph_edge(from_id);
CREATE INDEX idx_graph_edge_to ON graph_edge(to_id);
CREATE INDEX idx_graph_edge_kind ON graph_edge(kind);

CREATE TABLE graph_schema (version TEXT PRIMARY KEY, node_kinds TEXT NOT NULL, edge_kinds TEXT NOT NULL, installed_at TEXT NOT NULL);
CREATE TABLE graph_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
```

**schema_version 走 `atomic_write`**（沿用 M3 范式）。

### 4.2 `GraphQuery` DTO

```rust
pub struct GraphQuery {
    pub start: Vec<String>,
    pub direction: Direction,    // Out / In / Both
    pub edge_kinds: Vec<String>, // 空 = 不限
    pub max_depth: u8,           // 默认 2，**上限 4**
    pub limit: usize,            // 默认 100，**上限 1000**
    pub cursor: Option<String>,  // 翻页
}
```

**首期不开**路径查找；子图查询**不返回 props 正文**（K7）。

### 4.3 `graph_*` 命令清单（9 条）

| 命令 | 用途 | ACL 风险 |
|---|---|---|
| `graph_query` | 多跳邻居 | Low |
| `graph_node_get` | 单节点详情 | Low |
| `graph_node_list` | 节点列表（按 kind/label 过滤） | Low |
| `graph_node_upsert` | 用户手建/编辑（**禁覆盖 source=Manual**） | Medium |
| `graph_node_delete` | 用户删除（级联删边） | Medium |
| `graph_edge_list` | 边列表 | Low |
| `graph_edge_upsert` | 用户手建/编辑边 | Medium |
| `graph_edge_delete` | 用户删除边 | Medium |
| `graph_export` | graphml（**首期仅返回"暂不支持"**） | — |

### 4.4 维护任务

- `graph-vacuum` / `graph-backup` / `graph-fts-rebuild`（均 `enabled=false` 默认）
- 复用 `TaskKind::{Script, Command}`（**禁**新增 `TaskKind::GraphMaintenance`）
- 复用 `script_runner` 通道

### 4.5 收口

- `graph-store-shutdown` **先于** `stop-scheduler` 注册（A7 冻结：scheduler 关 → 维护任务结束 → graph store flush）
- 关闭流程：① stop maintenance tasks ② flush WAL ③ close db

---

## 5. FORBID

- **不**让用户通过 `graph_node_upsert` 覆盖 `source=Manual` 节点
- **不**让查询返回 `props` 正文（仅摘要，K7）
- **不**让 `max_depth > 4`（防爆栈）
- **不**让 `limit > 1000`（防响应体爆）
- **不**为维护任务新增 `TaskKind::GraphMaintenance`（A6 冻结）
- **不**让 `graph-store-shutdown` 改 `stop-scheduler` 索引序（A7 冻结）
- **不**让 `graph_export("graphml")` 首期实现（返回"暂不支持"）
- **不**破 K1/K3/K5
- **不**移动 `NEXT`、不 push

---

## 6. COMMANDS

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# A. 现状复核
ls src-tauri/src/graph_*.rs 2>&1
grep -c "graph_query\|graph_node\|graph_edge" src-tauri/src/bridge.rs

# B. 反向用例
# N1: max_depth=10 → 截断到 4
# N2: limit=10000 → 截断到 1000
# N3: 用户 upsert source=Manual 节点 → 阻断
# N4: graph_query 返回 props 正文 → 阻断
# N5: graph_export("graphml") → 返回 "暂不支持"
# N6: graph.db 损坏 → 自动从 graph.db.bak 恢复 + 提示
# N7: 关闭流程中 graph-store-shutdown 在 stop-scheduler 之后 → 单测断言

# C. 性能基线
# 1000 节点查询 depth=2 < 500ms
# 10000 节点查询 depth=2 < 2s

# D. 编译与基线
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

# E. 门禁
bash scripts/pre-merge.sh
```

---

## 7. PASS_CRITERIA

| # | 判据 | 验证 |
|---|---|---|
| 1 | DDL 与 A9 graph-store-contract 一致 | 静态断言 |
| 2 | `max_depth` / `limit` 双重上限生效 | 单测 N1/N2 |
| 3 | `source=Manual` 节点不可被 upsert 覆盖 | 单测 N3 |
| 4 | 子图查询不返回 props 正文 | 单测 N4 |
| 5 | `graph_export("graphml")` 返回"暂不支持" | 单测 N5 |
| 6 | `graph.db` 损坏自动从 `graph.db.bak` 恢复 | 单测 N6 |
| 7 | `graph-store-shutdown` 注册在 `stop-scheduler` 之前 | 单测 N7 + 命令 E |
| 8 | 性能基线达标 | 命令 C |
| 9 | ACL 末条仍为 `list_artifact_images` | 命令 E |
| 10 | `cargo test` 全绿 | 命令 D |
| 11 | `pre-merge.sh` ALL_PASS | 命令 E |

---

## 8. FAIL_ACTION

| 失败 | 动作 |
|---|---|
| DDL 漂移（与 A9 契约不一致） | 阻断（A9 契约是 A0 签发的真相源） |
| `source=Manual` 被覆盖 | 阻断（用户数据红线） |
| `max_depth > 4` | 阻断（爆栈） |
| `graph-store-shutdown` 顺序错 | 阻断（A7 冻结） |
| `cargo clippy` warning > 13 + 本卡新增 | 按基线清零再合入 |

---

## 9. DOC_BACKWRITE

1. `详细设计与实施计划.md` L571 `[ ]` → `[x]`
2. `后续需求TODO.md` §13 状态 `PARTIAL`（留 `M5-9` UI）
3. `AI-模型切换与接手清单.md` NEXT 移至 `M5-9`
4. `logs/checkpoints/M5-8.a-2026MMDD-HHMM.md`
5. `M5-14-debt-ledger.md` 增项：路径查找 / graphml 导出 / props 宽容版本

---

## 10. COMMIT / NEXT

- **COMMIT**：A0 拣入后由 A17 实施填
- **NEXT**：M5-9（Graph UI + Agent 消费）

---

## 11. FORBID 遵守记录

- 本卡为 A1 M5-W0 文档展开，**未写任何产品代码**
- 未触 `src-tauri/src/`、`src-tauri/Cargo.toml`、`scripts/pre-merge.sh`、三份主文档、ACL/Capability
- 未移动 `NEXT`（仍 M5-W0）
- 未提交、未 push
