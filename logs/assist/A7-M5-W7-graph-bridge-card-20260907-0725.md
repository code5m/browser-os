# Lane A7 · M5-W7 图谱命令桥卡片（SUPPORT DOCS ONLY · 规划 W8 只读 graph 命令）

> 续 W5（图谱 core 切片，`4b438ef`）+ W6（图谱契约评审，`logs/assist/A7-M5-W6-graph-contract-review-20260906-2239.md`）。
> 本文件是 **M5-W7 Integration Dispatch（board `a26fbaf docs(M5): dispatch W7 command bridges`）** 下 Lane A7 的 docs-only 交付。
> 依据 board §M5-W7：`A7 = SUPPORT DOCS ONLY`，任务 = “Prepare graph command bridge plan for later W8; no graph command code in W7.” 交付物 = 本卡片（`Graph bridge card`）。
> **本波 W7 不写任何图谱命令代码**，仅为 W8 落地的只读 graph 命令桥规划契约与原子同包清单。

## 0. 调度匹配自检

| 项 | 实测 | 结论 |
|---|---|---|
| `WORKSPACE_IDENTITY` / `pwd` / branch | `BACKV3_MAIN` / 匹配 / `master` | ✅ |
| `git pull --ff-only` | 已是最新（HEAD=`a26fbaf`，W7 dispatch 已含） | ✅ |
| NEXT/M5 | board §M5-W7：仅 A3 + A5 可写产品代码；A7 为 **DOCS ONLY** | ✅ 本包合规（仅卡片，无命令代码） |
| 是否越界 | 仅产出本卡片（`logs/assist/`）；**未**改 `graph.rs`/`domain.rs`/`main.rs`/`bridge.ts`/`default-commands.toml`/`src/types.ts` | ✅ |
| 工作树脏文件 | W7 其他 lane（A1/A2/A3/A4/A5/A6/A8/A9/A10/A11）并发产物非本 Lane，不纳入、不触碰 | — |

## 1. 背景与目的

- A8 在 **W6** 已实现图谱 UI 纯逻辑/面板壳（`filterGraphNodes`/`searchGraphNodes`/`summarizeNode` 等），但受 W6-HS1 约束**无后端 `graph_*` 命令**，故 W6 吃**静态 `GraphNode[]` 快照/fixture**（见 A7 W6 评审 F2）。
- A7 在 **W5** 已落地图谱 core 切片（`src-tauri/src/graph.rs` + `domain.rs` DTO/常量 + `check-graph-policy.py` 7 ACTIVE 码），含 bounded store/query helper（`bounded_neighbors`/`bounded_subgraph`/`from_json`/`validate_*`），**但尚无命令暴露**。
- **W8 目标**：把 A7 的 graph 能力以**只读命令桥**暴露给前端，让 A8 把静态快照替换为 `invoke('graph_query', ...)` 的实时只读查询。**W8 命令必须只读、无重建 worker、无执行、无第二路径**，并复用 W5 既有 helper（天然满足 7 个 GRAPH 策略码）。

## 2. 规划命令集（W8 只读，原子同包）

> 全部调用 `graph.rs` 既有 helper；不引入 `std::process`/`Command::new`/`tokio`/网络/后台重建（守 `GRAPH_NO_SECOND_PATH`）。
> 输出**一律脱敏**：节点/边不含 `props`（K7 + 防御纵深），仅返回 `{id, kind, label}`（+ 边 `from/to/kind`）。

### 2.1 `graph_query`（主查询）
- 入参：`{ start_id: String, depth: usize = 1, limit: usize = 1000 }`
- 行为：`depth = depth.min(GRAPH_MAX_DEPTH=4)`，`limit = limit.min(GRAPH_QUERY_LIMIT=1000)` → `GraphStore::bounded_subgraph(start_id, depth)`（内部走 `bounded_neighbors`，无向 BFS，返回 ≤ limit 节点 + 其**间**边）。
- 出参：`{ nodes: GraphNodeView[], edges: GraphEdgeView[], truncated: bool }`（`truncated=true` 表示触上限）。
- 脱敏：映射时丢弃 `props`/`weight`（或 weight 可保留为只读元信息，但 props 必丢）。

### 2.2 `graph_node_get`（单点）
- 入参：`{ id: String }`
- 行为：store 精确查找（Skill/Agent 节点 id 须 `GRAPH_NODE_ID_HEX_LEN=64` 十六进制，复用 `validate_id` 的 `is_hex` 校验 → 满足 `GRAPH_REF_NODE_INTEGRITY`）。
- 出参：`GraphNodeView | null`。

### 2.3 `graph_stats`（容量概览）
- 入参：无
- 行为：返回 `{ node_count, edge_count, max_nodes=GRAPH_MAX_NODES, max_edges=GRAPH_MAX_EDGES }`，供 A8 容量提示（≥0.8×max 显示「接近上限」）。
- 出参：上述小结构。

## 3. 存储来源（W8 不构建，仅读取）

- W8 命令**只读**一个已存在的 `GraphStore`：来源 = 启动时从快照文件（如 workspace 下 `graph_store.json`）经 `GraphStore::from_json` 载入 `AppState` 的 `Arc<Mutex<GraphStore>>`，**或**每次查询时读快照文件（≤5000 节点，JSON 解析可接受）。
- **图谱提取/重建（extractor / rebuild worker）不在 W8 范围**（W7/W8 硬停：no graph rebuild worker）。store 由后续 wave 的提取器填充；W8 命令仅消费。
- `from_json` 已逐条 `validate_graph_node/edge`（容量/脱敏/完整性），故载入即已守 `GRAPH_BOUNDED_STORE`/`GRAPH_PRIVACY_DOUBLE_SCAN`/`GRAPH_REF_NODE_INTEGRITY`。

## 4. 隐私与策略对齐（逐码映射）

| 既有 GRAPH 码（W5 已落 `check-graph-policy.py`） | W8 命令如何满足 |
|---|---|
| `GRAPH_CONSTANTS_PRESENT` | 复用 `domain.rs` 7 常量，命令不重定义 |
| `GRAPH_PROPS_EQ_MAX_TEXT_FIELD` | 复用，命令不触碰 |
| `GRAPH_PRIVACY_DOUBLE_SCAN` | 载入经 `from_json`→`validate_*`（已双扫）；命令**输出再脱敏**删 props |
| `GRAPH_BOUNDED_STORE` | 仅查询，不插入；store 容量由 W5 插入逻辑守 |
| `GRAPH_TRAVERSAL_BOUNDED` | `bounded_subgraph`/`bounded_neighbors` 已 `depth.min(MAX_DEPTH)`、`limit.min(QUERY_LIMIT)` |
| `GRAPH_REF_NODE_INTEGRITY` | `graph_node_get` 对 Skill/Agent id 走 `is_hex(64)` 校验 |
| `GRAPH_NO_SECOND_PATH` | 命令置于 `graph.rs`，复用 helper；**禁** `std::process`/`Command::new`/`tokio`/网络/后台线程 |

**新增建议码（W8 落地时补）**：`GRAPH_OUTPUT_NO_PROPS` —— 扫描 `graph.rs` 命令实现，断言输出结构**不包含** `props` 字段（防回归把 props 序列回前端）。纳入 `check-graph-policy.py` 后 `ACTIVE=8`，含自测好/坏样本。

## 5. W8 原子同包清单（按 W7 硬停「每条新命令原子化：source check + ACL + 前端 bridge/types + 策略覆盖 + 测试」）

| 文件 | 改动 |
|---|---|
| `src-tauri/src/graph.rs` | 新增 `graph_query`/`graph_node_get`/`graph_stats` 三个 `#[tauri::command] pub fn`（复用 helper；输出 `GraphNodeView`/`GraphEdgeView` 脱敏结构）；移除 `#![allow(dead_code)]`（已有消费方） |
| `src-tauri/src/main.rs` | `invoke_handler!` 注册 `graph::graph_query`/`graph::graph_node_get`/`graph::graph_stats` |
| `src-tauri/permissions/default-commands.toml` | `commands.allow` 在末条 `list_artifact_images` **之前**插入 `"graph_query"`/`"graph_node_get"`/`"graph_stats"`（守 ACL 末条规则） |
| `src/bridge.ts` | 新增 `graphQuery()`/`graphNodeGet()`/`graphStats()` 包装 `invoke`，类型用 `GraphNodeView`/`GraphEdgeView` |
| `src/types.ts` | 新增 `GraphNodeView`/`GraphEdgeView`（仅 `{id,kind,label}` / `{from,to,kind}`），不引入 `props` |
| `scripts/check-graph-policy.py` | 加 `GRAPH_OUTPUT_NO_PROPS` 码 + 自测样本（ACTIVE=8） |
| `scripts/pre-merge.sh` | 已有 `GRAPH_*` 门禁，自测/默认扫描自动覆盖新码（无需改接入） |
| 测试 | `graph.rs` 单测：构建小 store→`graph_query` 断言深度/limit 截断 + `props` 不出现在输出 JSON + `graph_node_get` 64-hex 校验；`cargo test graph` 全绿；`check-graph-policy.py --self-test` PASS(ACTIVE=8) |

> **source check**：沿用本仓库 Tauri ACL + capability 主窗闸门（与 A3/A5 W7 命令同款）——命令入 `default-commands.toml` allow 即受 capability 约束，仅主窗可调用。

## 6. 与 A8 W6 集成缝

- A8 W6 的 `useGraphQuery`/search 逻辑已对 `GraphNode[]` 运算；W8 落地后，A8 把静态快照源替换为 `graphQuery()` 返回（类型对齐 `GraphNodeView`）。
- A8 `summarizeNode` 白名单（W6 评审 F1 已修正为 `{id,kind,label,neighborCount}`）与 `GraphNodeView` 完全对齐 → **零漂移**。
- 前端 `src/bridge.ts` 已 `import { GraphNode, GraphEdge } from "./types"`（A8 W6 落地），W8 仅需补充 `GraphNodeView` 并加 `graphQuery` 包装。

## 7. 硬停 / 禁止（W8 实施时仍须守）

- **只读**：无插入/删除/更新节点边；无 skill 执行、无 plugin 安装、无 MCP server、无 graph rebuild worker。
- **无第二执行路径**：命令必须复用 `graph.rs` helper，不得 `std::process`/`Command::new`/`tokio`/网络。
- **K7 输出脱敏**：任何命令响应不得含 `props` key/value（含 `secret`/`token`/`api_key`/`Bearer ` 等）。
- **容量真源单点**：命令内不写 `5000`/`256`/`1000` 字面量，全部引用 `domain.rs` 常量（经 `graph.rs` 透传）。
- 不引入新 npm 依赖。

## 8. 给各 Lane 的 NEXT

- **给 A0**：本卡片为 W8 实施蓝图，可直接 drop；W8 由 A0 调度开启时按 §5 原子同包落地。
- **给 A8**：W8 命令落地后，将 W6 静态快照替换为 `graphQuery()`（`GraphNodeView` 类型对齐 `summarizeNode`）；W6 评审 F1 已就位，无需再改。
- **给 A3/A5**：source check / ACL / 前端 bridge 范式按本卡片 §5 同款（A3/A5 W7 命令即样板）。
- **给 A10**：W8 复审重点 = 命令只读 + ACL 末条前插入 + 输出零 props + 无第二路径 + 无敏感审计。
- **给 A11**：W8 收口核对 `src/types.ts` 与 `GraphNodeView` 契约 + `grep` 0 命中 `props` 于命令输出 + `check-graph-policy.py --self-test` PASS(ACTIVE=8)。

## 9. LANE 输出模板

```
LANE=A7
STATUS=PASS（DOCS ONLY，无产品代码）
WAVE=M5-W7
BASE=a26fbaf
HEAD=logs/assist/A7-M5-W7-graph-bridge-card-20260907-0725.md
FILES=logs/assist/A7-M5-W7-graph-bridge-card-20260907-0725.md
VERIFY=docs-only；规划 W8 只读 graph 命令桥（graph_query/graph_node_get/graph_stats），复用 W5 graph.rs helper；未改任何产品代码；git diff --check 未涉及本 Lane 文件
CHECKPOINT=logs/assist/A7-M5-W7-graph-bridge-card-20260907-0725.md
MERGE_NOTES=W7 A7 为 DOCS ONLY，产出图谱命令桥卡片规划 W8 只读 graph 命令：graph_query(有界子图,depth≤4/limit≤1000)/graph_node_get(64-hex 校验)/graph_stats(容量概览)；全部复用 W5 graph.rs bounded_subgraph/bounded_neighbors/from_json/validate_*，天然满足 7 GRAPH 策略码(GRAPH_NO_SECOND_PATH 禁第二路径)；输出脱敏删 props(K7+防御纵深)，建议新增 GRAPH_OUTPUT_NO_PROPS 码(ACTIVE=8)；原子同包清单=graph.rs 命令+main.rs handler+default-commands.toml ACL(插 list_artifact_images 前)+bridge.ts+types.ts(GraphNodeView)+check-graph-policy.py+测试；存储来源=快照/AppState 只读，W8 不构建重建 worker；前端 GraphNode/GraphEdge 类型 A8 W6 已落地，桥接缝对齐 summarizeNode 白名单(F1)
NEXT=A0 在后续 wave 开启 W8 时按本卡片实施；A8 将静态快照替换为 graphQuery() 实时只读
```
