# Lane A7 · M5-W8 图谱命令桥卡片（DOCS/GRAPH BRIDGE PLAN ONLY · 不依赖 A3/MCP）

> 续 W5（图谱 core 切片，`4b438ef`）+ W6（图谱契约评审，`A7-M5-W6-graph-contract-review-20260906-2239.md`）+ W7（图谱桥卡片，`A7-M5-W7-graph-bridge-card-20260907-0725.md`）。
> 本文件是 **M5-W8 Excluding-A3 Dispatch（board `6c1f30e docs(M5): dispatch W8 excluding A3`）** 下 Lane A7 的 docs-only 交付。
> 依据 board §M5-W8（line 196）：`A7 = START DOCS/GRAPH BRIDGE PLAN ONLY`，任务 = “Advance graph bridge plan **without MCP/A3 dependency**: define read-only graph query command contract, capacity/error states, and how GraphPanel consumes existing graph store. Do not implement backend commands.” 交付物 = 本卡片（blocked-by-A3/MCP 项与可独立交付 UI/store 项分离）。
> **本波 W8 不写任何图谱命令代码**，仅推进契约面。

## 0. 调度匹配自检

| 项 | 实测 | 结论 |
|---|---|---|
| `WORKSPACE_IDENTITY` / `pwd` / branch | `BACKV3_MAIN` / 匹配 / `master` | ✅ |
| `git pull --ff-only` | 已是最新（HEAD=`6c1f30e`，W8 dispatch 已含；A3 W7 MCP 命令 `6c1f30e` 已在树但**本波 HOLD**） | ✅ |
| NEXT/M5 | board §M5-W8：A3 **HOLD/NO ASSIGNMENT**；A7 为 **DOCS/GRAPH BRIDGE PLAN ONLY** | ✅ 本包合规（仅卡片，无命令代码） |
| A3 排除遵守 | 不触碰 `src-tauri/src/mcp.rs` / A3 MCP 文件；图谱桥不引用 MCP registry/policy | ✅ |
| 是否越界 | 仅产出本卡片（`logs/assist/`）；**未**改 `graph.rs`/`domain.rs`/`main.rs`/`bridge.ts`/`default-commands.toml`/`src/types.ts` | ✅ |
| 工作树脏文件 | W8 其他 lane（A1/A2/A4/A5/A6/A8/A9/A10/A11）并发产物非本 Lane，不纳入、不触碰 | — |

## 1. 当前已集成状态（事实基线）

| 组件 | 状态 | 来源 |
|---|---|---|
| `src-tauri/src/graph.rs`（W5） | `GraphStore` 内存 bounded 存储 + `validate_graph_node/edge` + `bounded_neighbors`/`bounded_subgraph` + `to_json`/`from_json` + 13 个 `GraphError` 变体；`#![allow(dead_code)]`（无消费方） | `4b438ef` |
| `src-tauri/src/domain.rs`（W5） | `GraphNode`/`GraphEdge` + 7 容量常量单源 | `4b438ef` |
| `scripts/check-graph-policy.py`（W5） | 7 ACTIVE 码（GRAPH_CONSTANTS_PRESENT / PROPS_EQ_MAX_TEXT_FIELD / PRIVACY_DOUBLE_SCAN / BOUNDED_STORE / TRAVERSAL_BOUNDED / REF_NODE_INTEGRITY / NO_SECOND_PATH） | `4b438ef` |
| `src/types.ts`（A8 W6） | `GraphNode`/`GraphEdge` + 7 容量常量；**尚无 `GraphNodeView`** | `5f92ece` |
| `scripts/check-graph-ui-logic.mjs`（A8 W6/W8） | 图谱 UI 纯逻辑 headless 测试已落 | `5f92ece` |
| `graph.rs` 命令 | **0**（守 docs-only） | — |
| A3 W7 MCP 命令（`mcp_policy_get`/`mcp_registry_list`/`mcp_capability_preview`） | 在树（`6c1f30e`）但 **W8 HOLD**，本波不依赖、不扩展 | board §M5-W8 L192 |

> **A1 W7 已按本 Lane W6 评审 F1 修订 M5-9 卡**：`summarizeNode` 白名单 8→4 字段（`{id,kind,label,neighborCount}`），并明确 `graph_query` 真实命令落地由 A0 W8 dispatch 决定（当前 W8 仍给 A7 docs-only）。本卡片即该契约的落地蓝图。

## 2. 核心判定：图谱桥**零 A3/MCP 依赖**

- W8 goal（board L178）：*“continue M5 without A3 … No MCP product-code work in this wave.”*
- 图谱桥只读命令**只消费 A7 自有 `GraphStore`**（`graph.rs`/`domain.rs`），经 Tauri invoke 直达 `GraphPanel`；**不**读取 MCP registry/policy、**不**经 rmcp server、**不**引用 `mcp.rs` 任何符号。
- 故整个图谱桥属于 **Independently shippable**，可由 A7/A8 在任一不含 A3 的 wave 落地，无需等 A3 恢复。

---

## 3. INDEPENDENTLY SHIPPABLE（可独立交付 · 无 A3/MCP 依赖）

### 3.1 只读命令契约（W8+ 落地，非本波实现）

所有命令复用 `graph.rs` 既有 helper，**禁** `std::process`/`Command::new`/`tokio`/网络/后台重建（守 `GRAPH_NO_SECOND_PATH`）；输出一律删 `props`（K7 + 防御纵深）。

#### `graph_query`（主查询）
- 入参：`{ start_id: String, depth: usize = 1, limit: usize = 1000 }`
- 行为：`depth = depth.min(GRAPH_MAX_DEPTH=4)`；`limit = limit.min(GRAPH_QUERY_LIMIT=1000)` → `GraphStore::bounded_subgraph(start_id, depth)`（内部 `bounded_neighbors`，无向 BFS，返回 ≤ limit 节点 + 其**间**边）。
- 出参：`{ nodes: GraphNodeView[], edges: GraphEdgeView[], truncated: bool, applied: { depth: usize, limit: usize } }`
  - `truncated=true` 表示触 `GRAPH_QUERY_LIMIT`；`applied` 回显实际生效的 depth/limit（静默 cap）。
- 错误：`GRAPH_INVALID_ID` / `GRAPH_REF_ID_NOT_HEX` / `GRAPH_STORE_LOAD_FAILED`（store 载入失败，见 §3.3）。

#### `graph_node_get`（单点）
- 入参：`{ id: String }`
- 行为：store 精确查找；Skill/Agent 节点 id 须 `GRAPH_NODE_ID_HEX_LEN=64` 十六进制（`is_hex` 校验 → `GRAPH_REF_NODE_INTEGRITY`）。
- 出参：`GraphNodeView | null`（缺失返回 `null`，**非错误**）。
- 错误：`GRAPH_INVALID_ID` / `GRAPH_REF_ID_NOT_HEX`。

#### `graph_stats`（容量概览）
- 入参：无
- 出参：`{ node_count, edge_count, max_nodes=GRAPH_MAX_NODES, max_edges=GRAPH_MAX_EDGES, approaching_capacity: bool }`
  - `approaching_capacity = node_count >= 0.8 * GRAPH_MAX_NODES`（供 UI 黄牌）。
- 错误：`GRAPH_STORE_LOAD_FAILED`。

#### 脱敏视图类型（`src/types.ts` 新增，不引 props）
```ts
export interface GraphNodeView { id: string; kind: GraphNodeKind; label: string; }
export interface GraphEdgeView { from: string; to: string; kind: GraphEdgeKind; }
```

### 3.2 容量 / 错误态映射（对照 `graph.rs` 13 个 `GraphError`）

| 阶段 | `GraphError` 变体 | 命令错误码 | UI 状态（A8 `errorStateFor`/`emptyStateFor` 对齐） |
|---|---|---|---|
| 入参校验 | `EmptyId` / `IdTooLong` | `GRAPH_INVALID_ID` | `ERROR_INVALID_ID` |
| 入参校验 | `RefIdNotHex` | `GRAPH_REF_ID_NOT_HEX` | `ERROR_REF_ID_NOT_HEX` |
| store 载入 | `Serde` / `SecretInProps` / `NodeCapacityExceeded` / `EdgeCapacityExceeded` / `LabelTooLong` / `PropKeyTooLong` / `PropValueTooLong` / `PropCountExceeded` | `GRAPH_STORE_LOAD_FAILED`（附原因） | `ERROR_LOAD` |
| 查询返回空 | （无错） | — | `EMPTY`（list 无节点）/ `NO_RESULT`（search 无果） |
| 触上限 | `bounded` 静默 cap | — | `TRUNCATED`（命令 `truncated` 字段）/ `OVERSIZE`（返回数 ≥ `GRAPH_QUERY_LIMIT`） |
| 接近容量 | （无错，`graph_stats.approaching_capacity`） | — | `APPROACHING_CAPACITY`（黄牌） |

> 错误码约定：`graph.rs` 命令返回 `Result<T, String>`，`Err` 为稳定码串（如 `"GRAPH_ERR:REF_ID_NOT_HEX"`），前端 `bridge.ts` 解析后映射到 §3.2 UI 状态；**不**记录 token/cookie/body（守 W8 硬停 L207）。

### 3.3 GraphPanel 如何消费「现有 graph store」（MCP 无关）

- **当前（W8）**：`GraphPanel` 经 `useGraphStore.ts` 静态 fixture（5 节点 + 7 边，A8 W6）消费 → 纯 helper（`filterGraphNodes`/`searchGraphNodes`/`summarizeNode`）对 `GraphNode[]` 运算。**该层完全 MCP 无关**。
- **store 来源**：后端 `GraphStore` 现仅由 `from_json` 在测试/fixture 载入（无 live extractor、无持久化，属后续 wave 的 scope 决策，亦**非 A3 依赖**）。W8+ 命令读该 store（快照/AppState）即返回，命令**不构建** store（守“no graph rebuild worker”）。
- **切换缝**：`graph_query` 落地后，`useGraphQuery` 把静态源替换为 `invoke('graph_query', …)` 返回 `GraphNodeView[]`，下游纯 helper **不变** → 零重构、零 A3 耦合。
- **K7 双闸**：前端 helper 永不渲染 `props`（W6 评审 F5）；命令输出亦删 `props`（§3.1）→ 双重保险。

### 3.4 原子同包清单（W8+ 落地时，守 W7/W8「每条命令原子化：source check + ACL + 前端 bridge/types + 策略覆盖 + 测试」）

| 文件 | 改动 |
|---|---|
| `src-tauri/src/graph.rs` | 新增 3 个 `#[tauri::command] pub fn`（复用 helper；输出 `GraphNodeView`/`GraphEdgeView` 脱敏）；移除 `#![allow(dead_code)]` |
| `src-tauri/src/main.rs` | `invoke_handler!` 注册 `graph::graph_query`/`graph::graph_node_get`/`graph::graph_stats` |
| `src-tauri/permissions/default-commands.toml` | `commands.allow` 在末条 `list_artifact_images` **之前**插入 3 条 `graph_*` |
| `src/bridge.ts` | 新增 `graphQuery()`/`graphNodeGet()`/`graphStats()` 包装 `invoke`，类型用 `GraphNodeView`/`GraphEdgeView` |
| `src/types.ts` | 新增 `GraphNodeView`/`GraphEdgeView`（仅 `{id,kind,label}` / `{from,to,kind}`） |
| `scripts/check-graph-policy.py` | 加 `GRAPH_OUTPUT_NO_PROPS` 码（扫描 `graph.rs` 命令输出不含 `props`）+ 自测样本（ACTIVE=8） |
| `scripts/pre-merge.sh` | 既有 `GRAPH_*` 门禁自动覆盖新码，无需改接入 |
| 测试 | `graph.rs` 单测：小 store→`graph_query` 断言 depth/limit 截断 + `props` 不出现于输出 JSON + `graph_node_get` 64-hex 校验；`cargo test graph` 全绿；`check-graph-policy.py --self-test` PASS(ACTIVE=8) |

> **source check**：沿用本仓库 Tauri ACL + capability 主窗闸门（与 A5 W7 命令同款）——命令入 `default-commands.toml` allow 即受 capability 约束，仅主窗可调用；**不**涉及 `mcp.rs`。

### 3.5 与既有 7 GRAPH 策略码对齐

| 码 | 如何满足 |
|---|---|
| `GRAPH_CONSTANTS_PRESENT` | 复用 `domain.rs` 7 常量，命令不重定义 |
| `GRAPH_PROPS_EQ_MAX_TEXT_FIELD` | 复用，命令不触碰 |
| `GRAPH_PRIVACY_DOUBLE_SCAN` | 载入经 `from_json`→`validate_*` 双扫；命令**输出再脱敏**删 props |
| `GRAPH_BOUNDED_STORE` | 仅查询不插入；store 容量由 W5 插入逻辑守 |
| `GRAPH_TRAVERSAL_BOUNDED` | `bounded_subgraph`/`bounded_neighbors` 已 `depth.min(MAX_DEPTH)`/`limit.min(QUERY_LIMIT)` |
| `GRAPH_REF_NODE_INTEGRITY` | `graph_node_get` 对 Skill/Agent id 走 `is_hex(64)` 校验 |
| `GRAPH_NO_SECOND_PATH` | 命令置于 `graph.rs`，复用 helper；禁 `std::process`/`Command::new`/`tokio`/网络/后台 |
| （W8+ 新增）`GRAPH_OUTPUT_NO_PROPS` | 扫描命令输出不含 `props` 字段 |

---

## 4. BLOCKED-BY-A3/MCP（显式分离 · 非本波 · 待 A3 恢复）

> 以下均**不**阻塞 §3 独立交付；仅在未来需把图谱接入 MCP 生态时涉及，且需 A3 恢复后才能做。

| 项 | 说明 | 阻塞原因 |
|---|---|---|
| 经 MCP/rmcp 暴露 `graph_query` | 把图谱查询作为 MCP capability（`mcp_graph_query`）经 rmcp server 暴露给外部 agent | 需 A3 的 `mcp.rs`/rmcp runtime（W8 HOLD） |
| 图谱 DTO 引用 MCP registry tool id | 新增 `GraphNodeKind::McpTool` / `GraphEdgeKind::UsesMcpTool` 边，节点 id 引用 `mcp_registry_list` 结果 | 需 A3 MCP registry 数据 + 类型耦合 |
| 图谱 ↔ MCP 策略交叉校验 | `graph_query` 返回前调用 `mcp_capability_preview` 做 capability 门禁 | 需 A3 `mcp_capability_preview` 命令 |

**结论**：上述 3 项均为「图谱能力 → MCP 生态」的可选增强，**不影响** `GraphPanel` 经 Tauri invoke 直连 `graph_query` 的核心链路。W8 本波图谱桥可独立于 A3 全量交付。

---

## 5. 硬停 / 禁止（W8 实施时仍须守）

- **A3 排除**：不触碰 `mcp.rs`/A3 文件；图谱桥不 import 任何 MCP 符号。
- **只读**：无插入/删除/更新节点边；无 skill 执行、无 plugin 安装、无 MCP server、无 graph rebuild worker。
- **无第二执行路径**：命令必须复用 `graph.rs` helper，不得 `std::process`/`Command::new`/`tokio`/网络。
- **K7 输出脱敏**：任何命令响应不得含 `props` key/value。
- **容量真源单点**：命令内不写 `5000`/`256`/`1000` 字面量，全部引用 `domain.rs` 常量。
- 不引入新 npm 依赖；不记录 token/cookie/body/prompt-secret。

## 6. 给各 Lane 的 NEXT

- **给 A0**：本卡片即 W8+ 图谱桥落地蓝图；A0 后续 wave 开启 `graph_query` 真实命令时（由 A7 或 A8 实施）按 §3 原子同包落地即可，无需等 A3。
- **给 A8**：W8 继续打磨 `GraphPanel` 纯逻辑（a11y/empty/error/oversize/deterministic），并可在 `graph_query` 落地时把静态 fixture 换成 `graphQuery()`；`summarizeNode` 白名单已对齐 §3.1。
- **给 A2/A4/A10**：W8 复审重点 = 图谱桥零 A3/MCP 泄漏（§4 全 blocked 项未混入）、只读、ACL 末条前插入、输出零 props、无第二路径、无敏感审计。
- **给 A11**：W8 收口核对 `src/types.ts` 与 `GraphNodeView` 契约 + `grep` 0 命中 `props` 于命令输出 + `check-graph-policy.py --self-test` PASS(ACTIVE=8)（命令落地后）。

## 7. LANE 输出模板

```
LANE=A7
STATUS=PASS（DOCS/GRAPH BRIDGE PLAN ONLY，无产品代码）
WAVE=M5-W8 Excluding-A3
BASE=6c1f30e
HEAD=logs/assist/A7-M5-W8-graph-bridge-card-20260907-0756.md
FILES=logs/assist/A7-M5-W8-graph-bridge-card-20260907-0756.md
VERIFY=docs-only；推进 W7 图谱桥计划，定义只读 graph_query/graph_node_get/graph_stats 契约+容量/错误态(映射 graph.rs 13 GraphError)+GraphPanel 消费现有 store(MCP 无关)；A3 排除遵守(0 触碰 mcp.rs)；未改任何产品代码；git diff --check 未涉及本 Lane 文件
CHECKPOINT=logs/assist/A7-M5-W8-graph-bridge-card-20260907-0756.md
MERGE_NOTES=W8 A7 为 DOCS/GRAPH BRIDGE PLAN ONLY，产出图谱桥卡片(推进 W7)：① 核心判定=图谱桥零 A3/MCP 依赖(只读命令只消费 A7 自有 GraphStore，不经 mcp.rs/rmcp)，整体可独立交付；② INDEPENDENTLY SHIPPABLE=graph_query(depth≤4/limit≤1000 经 bounded_subgraph,输出删 props,truncated/applied 字段)/graph_node_get(64-hex 校验,缺失返 null)/graph_stats(approaching_capacity 黄牌)+GraphNodeView/GraphEdgeView 类型+原子同包清单(graph.rs 命令+main.rs+ACL 插 list_artifact_images 前+bridge.ts+types.ts+check-graph-policy.py 新增 GRAPH_OUTPUT_NO_PROPS ACTIVE=8+测试)+容量/错误态映射 graph.rs 13 GraphError→稳定码(GRAPH_INVALID_ID/REF_ID_NOT_HEX/STORE_LOAD_FAILED)+UI 状态(EMPTY/NO_RESULT/ERROR_*/TRUNCATED/OVERSIZE/APPROACHING_CAPACITY)；③ BLOCKED-BY-A3/MCP 显式分离=经 MCP 暴露 graph_query/MCP tool 节点 kind/图谱↔MCP 策略交叉校验(均不阻塞核心链路,待 A3 恢复)；④ 复用 W5 7 GRAPH 策略码,新增 GRAPH_OUTPUT_NO_PROPS；⑤ GraphPanel 消费=静态 fixture→invoke(graph_query) 切换缝零重构、MCP 无关；⑥ A1 W7 已按 W6 F1 把 summarizeNode 白名单 8→4 字段,本卡片契约对齐
NEXT=A0 后续 wave 开启 graph_query 真实命令时按 §3 落地(由 A7/A8 实施,无需等 A3)；A8 W8 继续打磨 GraphPanel 纯逻辑
```
