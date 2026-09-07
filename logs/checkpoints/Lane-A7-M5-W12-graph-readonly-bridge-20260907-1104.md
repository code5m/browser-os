# Lane A7 · M5-W12 Graph Live-Query Readonly Bridge — 实施交付与验证

> Wave：**M5-W12 Graph Live-Query Readonly Dispatch**（board `269269a` 之后，W11 已集成）
> Base：`269269a`（W11 MCP stdio dry-run hardening 已集成，工作树被多 Lane 并发修改）
> Lane 状态：**START PRODUCT CODE NARROW → 交付完成**
> 本 Lane 范围（board §M5-W12 L198 / L208）：实现图谱只读 live-query 桥——`GraphState` 托管状态、启动载入快照、`graph_query`/`graph_node_get`/`graph_stats` 三命令（带 `check_invocation_source`、有界 depth/limit、稳定错误码、出参无 `props`）、ACL/bridge.ts/types 镜像、graph 策略自测扩展。**无** graph build/index/write/export/background worker。

## 1. 实现内容（逐文件）

| 文件 | 改动 |
|---|---|
| `src-tauri/src/graph.rs` | 移除遗留 `#![allow(dead_code)]`；新增 `GraphState{RwLock<GraphStore>}+Default`、`GraphError::code()`（13→稳定 `GRAPH_*` ASCII 码）、`validate_id_public`、`load_snapshot`（失败回退空 store 不 panic）；新增纯内核 `graph_query_impl`/`graph_node_get_impl`/`graph_stats_impl`（无 AppHandle/IO，可直接单测）；测试模块增补 6 个聚焦单测。`GraphNodeView`/`GraphEdgeView` 的 `From` 转换在 `domain.rs`。 |
| `src-tauri/src/domain.rs` | 新增只读出参 DTO：`GraphNodeView`/`GraphEdgeView`（无 `props`）+ `From<&GraphNode>`/`From<&GraphEdge>`；`GraphQueryRequest`/`GraphQueryLimits`/`GraphQueryResult`/`GraphStats`；常量 `GRAPH_DEFAULT_QUERY_DEPTH=2`。 |
| `src-tauri/src/bridge.rs` | 末尾新增 3 个 `#[tauri::command]`（`graph_query`/`graph_node_get`/`graph_stats`）：先 `check_invocation_source`（与全仓命令同源），再读 `app.state::<GraphState>()` 调纯内核；出参经 View 删 `props`，错误仅稳定码。命令不反向依赖 `crate::bridge`，`graph.rs` 仍零 `crate::bridge`（守 `GRAPH_NO_SECOND_PATH`）。 |
| `src-tauri/src/main.rs` | `.manage(crate::graph::GraphState::default())`；`.setup` 内 `workspace::data_dir(...).join("graph.json")` 经 `load_snapshot` 载入托管状态（失败回退空 store，不 panic）；`invoke_handler!` 注册 3 命令。 |
| `src-tauri/permissions/default-commands.toml` | ACL 在 `mcp_registry_list` 后、`list_artifact_images` 前插入 `graph_query`/`graph_node_get`/`graph_stats`。 |
| `src/types.ts` | 新增 `GraphNodeView`/`GraphEdgeView`/`GraphQueryRequest`/`GraphQueryLimits`/`GraphQueryResult`/`GraphStats`（无 `props`）。 |
| `src/bridge.ts` | 修复 `GRAPH_COMMANDS_AVAILABLE` 重复声明（并发编辑致出现 2–3 处 `export const`，会破坏 `npm build`），并翻为 `true`（后端已落地）。前端 `graphQuery(req,signal?)`/`graphNodeGet(id)`/`graphStats()` 封装（A8 W11 已写，本 Lane 仅修正守卫）。 |
| `scripts/check-graph-policy.py` | 新增 `GRAPH_OUTPUT_NO_PROPS`（ACTIVE 7→8）：`domain.rs` View 不得含 `props`；`graph.rs` 出参须经 `GraphNodeView/EdgeView::from` 且 `GraphError::code` 必须存在。扩充 GOOD_DOMAIN/GOOD_GRAPH 好样本 + 2 个坏样本。 |

## 2. 验证（已在本机执行）

- `cargo check`（src-tauri 默认构建）：**PASS**（仅 3 条预存在 warning，均非本 Lane 引入）。
- `cargo test`（全量）：**414 passed; 0 failed**（含本 Lane 新增 6 个图谱聚焦单测：`graph_query_ready_view_omits_props`/`graph_query_truncates_when_over_limit`/`graph_query_missing_start_is_found_false_not_error`/`graph_node_get_missing_is_none`/`graph_stats_approaching_capacity_flag`/`error_codes_are_stable_ascii`）。
- `python3 scripts/check-graph-policy.py --self-test`：**GRAPH_POLICY_SELF_TEST=PASS（ACTIVE=8）**。
- `python3 scripts/check-graph-policy.py`（默认扫描真实仓库）：**GRAPH_POLICY=PASS（无违规）**。
- `pre-merge.sh` 的图谱夹具门禁（L443–446）直接调用上述脚本，无需改阈值（ACTIVE 数由 `ALL_CODES` 动态派生，已从 7→8）。

## 3. W12 硬停合规（board L204–209）

- 只读查询：**无** graph build/index/write/export/background worker；命令经纯内核读 `GraphStore`，运行期零写。
- 出参无 `props`：`GraphNodeView`/`GraphEdgeView` 结构体无 `props` 字段（K7 双闸，经 `From` 丢弃）；`GRAPH_OUTPUT_NO_PROPS` 静态守门。
- 错误稳定码：所有 `Err` 为 `GRAPH_*` ASCII 串，绝不 echo props/label/路径/secret/query body（W9/W10/W11/W12 隐私纪律）。
- 三同步：3 命令过 `check_invocation_source` + ACL（`default-commands.toml`）+ `main.rs` handler + `bridge.ts`/`types.ts` 镜像 + `check-graph-policy.py` + Rust 单测，**同包**完成。
- 零依赖新增：不引 `tokio`/`rmcp`，不需 `mcp` feature gate，默认构建即含；`graph.rs` 仍零 `crate::bridge`（无第二执行路径）。
- 仅 A0 push（本 Lane 不 commit 不 push）。

## 4. 并发说明（重要）

当前工作树被多 Lane 并发修改（A8 的 `useGraphStore.ts`/`GraphPanel.vue`/`graphUi.ts`/`check-graph-ui-logic.mjs`、A1/A3/A4/A5/A6/A9/A10/A11 文档、A3 `check-mcp-policy.py` 等）。本 Lane 的补丁（`Lane-A7-M5-W12-graph-readonly-bridge-20260907-1104.patch`）按 8 个 A7 自有文件生成，其中 `src/bridge.ts`、`src/types.ts`、`scripts/check-graph-policy.py`、`src-tauri/permissions/default-commands.toml` 与 A8/A3 存在**同一文件并发改动**；A7 在本文件中的具体 hunks 见 §1 与下方 LANE 块，A0 集成时据此隔离/排序。

## 5. LANE 输出

```
LANE=A7
STATUS=PASS（PRODUCT CODE NARROW，已交付并验证）
WAVE=M5-W12 Graph Live-Query Readonly Dispatch
BASE=269269a
HEAD=logs/assist/A7-M5-W12-graph-readonly-bridge-20260907-1104.md
FILES=scripts/check-graph-policy.py, src-tauri/permissions/default-commands.toml, src-tauri/src/bridge.rs, src-tauri/src/domain.rs, src-tauri/src/graph.rs, src-tauri/src/main.rs, src/bridge.ts, src/types.ts
VERIFY=cargo check PASS(仅预存warning); cargo test 全量 414 passed 0 failed(含 A7 新增 6 图谱聚焦单测); check-graph-policy.py --self-test PASS(ACTIVE=8); 默认扫描 PASS; ACL 3 命令已登记; 3 命令过 check_invocation_source; 出参 View 无 props(K7); 错误稳定 GRAPH_* 码零 secret; 默认构建无新依赖(不引 tokio/rmcp); 补丁 8 文件 35KB
CHECKPOINT=logs/checkpoints/Lane-A7-M5-W12-graph-readonly-bridge-20260907-1104.patch
MERGE_NOTES=A7 W12 实现图谱只读 live-query 桥(3 命令 graph_query/graph_node_get/graph_stats):① graph.rs 新增 GraphState{RwLock<GraphStore>}+Default、GraphError::code()稳定 GRAPH_* ASCII 码、validate_id_public、load_snapshot(失败回退空 store 不 panic)、纯内核 graph_query_impl/graph_node_get_impl/graph_stats_impl(无 AppHandle/IO,可单测);② domain.rs 新增 GraphNodeView/GraphEdgeView(无 props)+From、GraphQueryRequest/Limits/Result/Stats、GRAPH_DEFAULT_QUERY_DEPTH=2;③ bridge.rs 末尾 3 命令:check_invocation_source 同源+读 GraphState 调纯内核+出参经 View 删 props+错误仅稳定码;graph.rs 仍零 crate::bridge(守 GRAPH_NO_SECOND_PATH);④ main.rs manage GraphState+setup 载入 graph.json 快照+invoke_handler 注册 3 命令;⑤ ACL 在 mcp_registry_list 后插 3 条;⑥ types.ts 新增 View/Request/Result/Stats 镜像(无 props);⑦ bridge.ts 修复 GRAPH_COMMANDS_AVAILABLE 重复声明(并发致 2-3 处会坏 npm build)并翻 true;⑧ check-graph-policy.py 新增 GRAPH_OUTPUT_NO_PROPS(ACTIVE 7→8)含好/坏样本自测;⑨ 验证:cargo check PASS、cargo test 414 passed(6 新)、graph 策略 self-test PASS(ACTIVE=8)、默认 PASS;⑩ 并发:bridge.ts/types.ts/check-graph-policy.py/default-commands.toml 与 A8/A3 同文件并发,A0 据本 checkpoint 隔离
NEXT=A0 集成本 Lane 补丁(W12 只读桥已可落地,无需等 A3);A8 继续 W12 UI 消费(useGraphStore 已随 GRAPH_COMMANDS_AVAILABLE=true 解锁,graphUi/GraphPanel 实时载入/查询/统计,AbortController 取消);A3 复核图谱命令未暴露为 MCP 可执行工具(守 dry-run/read-only);A4 复核 GraphError::code 零 secret echo 与出参无 props;A10 安全复审(W12 图桥无写/后台 worker/props 外泄/source-check 缺口);A11 重测 cargo test/图策略 self+default/MCP 策略仍 PASS/Agent-Skill 锁仍 PASS/图 UI logic/npm build/pre-merge 与 build metrics≤22%/warning 不增
```
