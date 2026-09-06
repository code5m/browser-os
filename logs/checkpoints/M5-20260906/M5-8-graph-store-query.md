# M5-8 图存储与查询（邻接表 + DDL + `graph_*` 命令）

> 子卡 ID：**M5-8** · 需求 #13 · `[S3|LEVERAGE:3|COMPLEX|AI:DEEP|R:xhigh]`
> 责任 Lane 候选：**A17**
> 父卡：`详细设计与实施计划.md` L571
> 主预研：`logs/assist/A9-M5-graph-store-contract-20260906-0700.md`（DDL + GraphQuery DTO）· `logs/assist/A9-M5-graph-scheduler-feed-20260906-0700.md`
> 配套：`M5-7-graph-model-extract.md`（抽取）· `M5-9-graph-ui-agent-consume.md`（UI 消费）
>
> **W3** BLOCKED（待 A17 = A7 W5 实施期承接）· **W4** ACTIVE（**A7 graph store/query policy slice**；详见本卡顶部 `[W5 next-card acceptance criteria]` 段）

---

## [W5 next-card acceptance criteria · 2026-09-06 18:35 CST] A7 M5-8 W5 实施期 acceptance criteria（pure graph query + policy script · 不接 live UI / agent 消费）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L157（**A7 M5-W5** *"Implement M5-7/M5-8 graph model/store policy slice: ... pure graph store/query helpers, policy script. No graph UI and no agent consumption yet."*）+ L134-170 硬约束 + A7 W4 graph core delta（`1610939` 拣入）锁定的 query 边界。
> **本卡 W5 与 M5-7 W5 的关系**：A7 W5 dispatch L157 派发**双卡**——M5-7 DTO + 容量 + redaction + pure store helpers；M5-8 pure graph query helpers + policy script + 不注册 `graph_*` 命令。**M5-8 W5 不接 live UI（9 条命令保持 LOCKED 直到 M5-8 收口后由 A0 决定）**。
> **A1 W5 角色**：A1 W5 **不**改 §1~§11 决策史；仅在头部加本 `[W5 next-card acceptance criteria]` 段，**明确 A7 W5 实施期 4 项 AC + 5 项 hard stops**，供 A7 / A10 / A11 / A0 验收。

### W5 A7 M5-8 实施期 acceptance criteria（4 项）

| AC | 描述 | 验收证据 |
|----|------|----------|
| AC-1 **pure graph query helpers 冻结** | `pure_query_neighborhood(graph, node_id, hop, edge_kinds, label_filter) -> Result<SubgraphView>` / `pure_query_shortest_path(graph, from, to, weight_kind) -> Result<PathView>` / `pure_query_subgraph(graph, root_set, max_nodes, max_edges) -> Result<SubgraphView>` 纯函数冻结在 `core::graph_query` 模块；返回类型为视图（只读），不持有 graph 写锁；返回节点/边数量受 `GRAPH_QUERY_MAX_NODES` / `GRAPH_QUERY_MAX_EDGES` 上界限制（防 DoS）| `cargo test core::graph_query` 或 `cargo test graph::query::*` |
| AC-2 **查询结果截断/超时/取消** | 复用 M4-1.c 的 row/byte/field 截断约定：`GRAPH_QUERY_MAX_RESULT_BYTES` / `GRAPH_QUERY_MAX_PATH_LEN` / `GRAPH_QUERY_TIMEOUT_MS`（参数化时钟）三常量在 `domain.rs` 冻结；超限返回 `GraphQueryTruncated { reason, partial }` 而**不**直接 dump 全部；超时由注入的 clock 判定（无 thread::sleep）| focused Rust 单测 + `scripts/check-graph-policy.py` self-test |
| AC-3 **policy script + pre-merge wire（覆盖本卡 + M5-7）** | `scripts/check-graph-policy.py` 三模式 PASS；policy 守门项至少 8 条：① 无新 Tauri 命令 ② DTO 字段与 A9 prework 0 drift ③ 容量上界常量在 `domain.rs` 单一真源 ④ redaction 覆盖 `source_ref` + `props` 双路径 ⑤ query helpers 是 pure（无 Tauri import）⑥ query helpers 复用 M4-1.c 截断约定 ⑦ 无 npm / 无 new Cargo 依赖 ⑧ 不在 `core::graph*` 引 `crate::bridge` 或 `tauri::AppHandle` | `scripts/check-graph-policy.py` self-test + default + `--expect-pending` 三模式 PASS |
| AC-4 **不注册 9 条 graph_* Tauri 命令** | `grep -E "graph_query\|graph_insert\|graph_delete\|graph_export\|graph_*" src-tauri/src/bridge.rs` 仅出现 `// not-implemented-yet` 占位 / 注释；`src-tauri/permissions/default-commands.toml` 末条仍恒为 `list_artifact_images`，**未**新增 graph 命令 | `git diff src-tauri/permissions/default-commands.toml` 0 新增命令行 + A10 抽查 `bridge.rs` |

### W5 A7 M5-8 实施期 hard stops（5 项）

| HS | 约束 | 来源 |
|----|------|------|
| W5-HS1 | **无 live UI / 无 agent 消费** —— `graph_*` 命令保持 LOCKED；9 条命令的具体语义延后 W6+/A0 决定 | PARALLEL_COMMAND_BOARD L157 + L167 |
| W5-HS2 | **无 background graph rebuild workers / 无 network access** —— W5 不接后台 graph 重建 | PARALLEL_COMMAND_BOARD L167 |
| W5-HS3 | **无新 Tauri 命令 / 无新 Cargo 依赖 / 无 npm** | PARALLEL_COMMAND_BOARD L168 + L157 |
| W5-HS4 | **无 second execution path / 无 installer / 无 model provider / 无 download path** | PARALLEL_COMMAND_BOARD L165 + L169 |
| W5-HS5 | **所有 lane 必须从 `origin/master` pull，不 push** | PARALLEL_COMMAND_BOARD L170 |

### W5 验证清单（供 A11 收口）

- `cargo test --manifest-path src-tauri/Cargo.toml graph_query` PASS
- `python3 scripts/check-graph-policy.py --self-test` PASS
- `python3 scripts/check-graph-policy.py` PASS
- `python3 scripts/check-graph-policy.py --expect-pending` PASS（如有 PENDING）
- `git diff src-tauri/permissions/default-commands.toml` 0 新增命令行
- `grep -c "graph_query\|graph_insert\|graph_delete" src-tauri/src/bridge.rs` 仅匹配占位/注释
- `bash scripts/pre-merge.sh` ALL_PASS
- `git diff --check` CLEAN
- **A10 复审 PASS**（no unbounded maps / no network / no agent consumption / capability 真源单点 / redaction 双路径 / 0 新 Tauri 命令）
- **A11 verification delta** 产出 `logs/checkpoints/M5-A11-W5-*.md`

### W5 A1 不修订范围（本卡）

- **§1 GOAL / §2 READ / §3 WRITE / §4 关键契约 / §5 FORBID / §6 COMMANDS / §7 PASS_CRITERIA / §8 FAIL_ACTION / §9 DOC_BACKWRITE / §10 COMMIT / §11 FORBID 遵守记录**：A1 W5 **不动**（决策史保持 W0 原文；W5 AC 在本顶部段单列）。
- **三份主文档 / ACL / Capability / pre-merge.sh / scripts/**：A1 W5 不动（policy 脚本由 A7 落地）。
- **`NEXT` 标记**：A0 调度权；A1 不改字面值。
- **本卡与 M5-7 关系**（A7 实施期双卡并行）：M5-7 DTO/容量/redaction/pure store；M5-8 pure query/policy/无命令。两条 AC 互补、不重叠。

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
