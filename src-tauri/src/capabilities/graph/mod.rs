//! Knowledge Graph 能力原生后端（core 纯逻辑切片，M5-7/M5-8，Lane A7）。
//!
//! 承载 `graph.rs`（图谱 DTO 在 `domain.rs`；本模块提供校验 / 容量 / 脱敏 /
//! bounded store / bounded query 助手）。**本模块不含任何 `#[tauri::command]`**——
//! 三个图谱只读命令 `graph_query` / `graph_node_get` / `graph_stats` 的命令体
//! 注册在 `bridge.rs`（`bridge::graph_query` 等，经 `generate_handler!`），命令体
//! 内部委托本模块的纯函数（`graph_query_impl` / `graph_node_get_impl` /
//! `graph_stats_impl` / `validate_id_public` / `GraphState` / `load_snapshot`）。
//!
//! 设计红线（承 W4 A7 delta + A4 W5 评审 + `check-graph-policy.py`）：
//! 纯逻辑、无副作用、无 `tauri` / `AppHandle` / `crate::bridge` / 网络 / 命令 /
//! 第二执行路径（`GRAPH_NO_SECOND_PATH`）；出参经 `GraphNodeView`/`GraphEdgeView`
//! 删除 `props`（K7 防御纵深）；Skill/Agent 节点按 64-hex `id` 引用 A5，不存
//! `agent_kv` 值（仅 `Memorizes` 关系边）；持久化（SQLite 单连接）留待 M5-8。
//!
//! 迁移自 `src-tauri/src/graph.rs`
//! （Native Physical Boundary Matrix Pilot 6，见
//! `docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §8.8）。
//! 既有 `crate::graph::` 调用点（bridge.rs 图谱命令体 ×10 + main.rs 启动加载 ×3）
//! 经 `main.rs` 顶部 re-export shim 解析，无需逐处改写。
pub mod graph;
