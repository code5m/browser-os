# Capability Native Module: `graph` (Rust)

> 迁移自 `src-tauri/src/graph.rs`（Native Physical Boundary Matrix Pilot 6）。
> 分类：**CAPABILITY_NATIVE(graph)**（矩阵 §2.3；target `src-tauri/src/capabilities/graph/`）。
> 对照矩阵：`docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §2.3 / §8.8。
> 同级 TS 能力：`src/capabilities/graph/`（成熟度 MEDIUM，命令接线仍经 `bridge.ts`）。
> 注意：本模块是 **core 纯逻辑切片**（M5-7/M5-8，Lane A7），**无 `#[tauri::command]`**；图谱命令体在 `bridge.rs`。

## 1. DDD 职责（Domain Responsibility）

知识图谱的纯逻辑层（与 `domain.rs` 的 `GraphNode`/`GraphEdge`/`GraphProps` DTO + 7 个容量常量协同）：

- `GraphStore`：内存 bounded 存储（`RwLock`），容量上限 `GRAPH_MAX_NODES`/`GRAPH_MAX_EDGES`。
- `validate_graph_node` / `validate_graph_edge`：容量（label/id/prop 字节上限）+ 脱敏（凭据双扫 `SENSITIVE_KEY_NAMES`/`SENSITIVE_VALUE_PATTERNS`）+ Skill/Agent 节点 `id` 须 64-hex（`GRAPH_NODE_ID_HEX_LEN`，AGRAPH-10）。
- `bounded_neighbors` / `bounded_subgraph`：有界遍历（`GRAPH_MAX_DEPTH`≤4 / `GRAPH_QUERY_LIMIT`≤1000）。
- `to_json` / `from_json`：`GraphStore` 与 graph.json 快照的 serde 壳（`from_json` 拒绝含 `api_key` 等凭据的 props）。
- `GraphError` + `.code()`：稳定错误码（不 echo 内部串）。
- 命令内核（被 `bridge.rs` 调用，非本模块命令）：`graph_query_impl` / `graph_node_get_impl` / `graph_stats_impl` / `validate_id_public` / `GraphState` / `load_snapshot`。

## 2. 边界（Boundary / Non-Responsibility）

- **不含任何 `#[tauri::command]`**：命令体在 `bridge.rs`（`bridge::graph_query` 等），本模块只提供纯函数与 `GraphState`。
- **不引入 `tauri` / `AppHandle` / `crate::bridge` / 网络 / 后台 worker / 第二执行路径**（`GRAPH_NO_SECOND_PATH`）。
- **不写盘 / 不导出 / 不构建 / 不索引**：`graph.json` 仅做**只读快照**加载（main.rs 启动时若有则载入 `GraphState`）；持久化（SQLite 单连接）留待 M5-8（A3/A4 稳定后）。
- **不存 `agent_kv` 值**：Skill/Agent 节点仅按 `id` 引用 A5 的 `SkillDef.id`/`AgentDef.id`，图谱与 agent memory 边界清晰（仅 `Memorizes` 关系边）。

## 3. Commands

**本模块无命令。** 三个图谱只读命令归属 graph 能力，但命令体当前注册在 `bridge.rs`（经 `generate_handler!`），命令体调用本模块纯函数：

| Tauri command | 命令体位置 | 委托本模块 |
|---|---|---|
| `graph_query` | `bridge.rs` (`bridge::graph_query`) | `validate_id_public` + `GraphState` + `graph_query_impl` |
| `graph_node_get` | `bridge.rs` (`bridge::graph_node_get`) | `validate_id_public` + `GraphState` + `graph_node_get_impl` |
| `graph_stats` | `bridge.rs` (`bridge::graph_stats`) | `GraphState` + `graph_stats_impl` |

命令体（含 `check_invocation_source`、ACL 接线、`GraphNodeView`/`GraphEdgeView` 出参脱敏）在 **bridge.rs 逐 command 分解阶段**（矩阵 §8 末段 / 协议 §8）迁移至 `capabilities/graph/commands.rs`。届时本目录补 `commands.rs` 并解除 `bridge.rs` 耦合。

## 4. Resources

- **有界内存 store**：`GraphState { store: RwLock<GraphStore> }`，由 main.rs `.manage(GraphState::default())` 注册为 AppState，图谱命令体经 `app.state::<crate::graph::GraphState>()` 读取。
- **只读快照**：`workspace/graph.json`（main.rs 启动时 `load_snapshot` 若有则载入 `GraphState`）；无写回。
- 无 WebView / PTY / socket / 子进程 / 定时器 / watcher。

## 5. 生命周期（Lifecycle）

- `GraphState` 在应用启动由 main.rs `.manage(GraphState::default())` 创建；启动时若 `workspace/graph.json` 存在则 `load_snapshot` 载入。
- 图谱命令体在运行期只读访问 `GraphState`（RwLock 读锁），瞬时、无长生命周期资源。
- 当前无写路径（只读切片）；M5-8 接 SQLite 持久化时才会引入写生命周期。

## 6. 依赖（Dependencies）

- `crate::domain`：`GraphNode`/`GraphEdge`/`GraphProps`/`GraphNodeKind`/`GraphEdgeKind` + 7 个容量常量真源（`GRAPH_PROPS_MAX_BYTES == MAX_TEXT_FIELD_BYTES` 恒等、`GRAPH_LABEL_MAX_BYTES`、`GRAPH_NODE_ID_HEX_LEN`、`GRAPH_MAX_DEPTH`、`GRAPH_QUERY_LIMIT`、`GRAPH_MAX_NODES`、`GRAPH_MAX_EDGES`）—— **SHARED_NATIVE_INFRASTRUCTURE，类型单一真源**。
- 外部 crate：`serde`（Serialize/Deserialize）；标准库 `std::collections::{BTreeMap,BTreeSet,VecDeque}`、`std::sync::RwLock`。

## 7. 禁止依赖（Forbidden Dependencies）

- 禁止 `crate::bridge`（AppState 共享态枢纽 / 命令 hub）。
- 禁止 `tauri` / `AppHandle` / `State` 引用（本模块不持有命令壳）。
- 禁止 `std::process` / `Command::new` / `tokio` / 网络 / 后台重建 worker（`GRAPH_NO_SECOND_PATH`）。
- 禁止引入 `agent_kv` 值或读取 agent memory（Skill/Agent 节点只引用 `id`）。

## 8. Public / Native Contract

- 对外纯函数契约：`GraphStore` / `validate_graph_node` / `validate_graph_edge` / `bounded_neighbors` / `bounded_subgraph` / `to_json` / `from_json` / `GraphError`(+`.code()`) / `graph_query_impl` / `graph_node_get_impl` / `graph_stats_impl` / `validate_id_public` / `GraphState` / `load_snapshot`。
- 领域类型真源在 `crate::domain`（禁止第二份定义）。
- 既有 `crate::graph::*` 调用点（bridge.rs ×10 + main.rs ×3）经 `main.rs` 顶部 `pub use crate::capabilities::graph::graph;` re-export shim 解析，**未逐处改写**。
- TS 侧经 `bridge.ts` 命令调用，不直接 `import` 本 crate。

## 9. Security / ACL

- `GRAPH_NO_SECOND_PATH`：本模块无第二执行路径 / 网络 / 反向依赖（守 `check-graph-policy.py`）。
- 出参脱敏：图谱命令出参经 `GraphNodeView`/`GraphEdgeView` 删除 `props`（K7 + 防御纵深第三闸），`props` 不外泄（防 secret echo）。
- ACL：三个图谱命令在 `permissions/default-commands.toml` 放行（`graph_query` / `graph_node_get` / `graph_stats`，位于末条 `list_artifact_images` 之前），与 `bridge.ts`/`src/types.ts` 镜像一致。

## 10. Tests

- `graph.rs` 内 `#[cfg(test)] mod`（验证 / 容量 / 脱敏 / bounded / json 往返 / secret 拒绝等，多例）。
- 运行：`cd src-tauri && cargo test capabilities::graph`（或 `cargo test` 全量）。

## 11. Source of Truth

- 领域类型 + 7 容量常量：`src-tauri/src/domain.rs`。
- 纯 store/query/校验：`src-tauri/src/capabilities/graph/graph.rs`（本模块）。
- 命令体（暂留）：`src-tauri/src/bridge.rs`（`graph_query`/`graph_node_get`/`graph_stats`）。
- 前端接线：`src/bridge.ts`（graphQuery 包装）+ `src/types.ts`（`GraphNodeView`/`GraphEdgeView`/`GraphQueryResult`）+ `components/graph/`（5 组件）。
- 门禁真源：`scripts/check-graph-policy.py`（7 ACTIVE 码 `GRAPH_*`）。

## 12. Known Debt

- `#![allow(dead_code)]` 已于 M5-W12 移除（helper 已被 bridge.rs 命令体消费）。
- 持久化（SQLite 单连接）未接（M5-8 规划，A3/A4 稳定后）；当前仅 graph.json 只读快照。
- 图谱命令体仍在 `bridge.rs`（LEGACY_MIXED_MODULE 残核的一部分）；属矩阵 §8 末段 bridge 分解计划。

## 13. Extraction Readiness

- **高。** 纯模块、零命令、零 `crate::bridge` 耦合、跨模块依赖仅 SHARED（`domain`）+ 标准库。
- 唯一 AppState 关联 `GraphState` 由 graph 能力自持（main.rs manage + 命令体读取），owner 明确，非 God Object。
- 达到阈值后可随 `graph` 能力提级为 crate `mvp-graph-rust`（不改领域语义）。
- 同能力下一个低风险同批候选：W12/W8 命令体 `commands.rs`（需先解除对 `bridge` hub 的耦合，见矩阵 §4 / 协议 §8）。
