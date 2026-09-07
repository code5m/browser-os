# Lane A7 · M5-W12 图谱只读 live-query 桥 — 实施卡片

> 续 `A7-M5-W11-graph-w12-impl-plan-20260907-1019.md`（W11 仅计划）。本卡是 W12 的**落地实现**，对应 board §M5-W12 A7 = START PRODUCT CODE NARROW。
> 验证：`cargo check` PASS；`cargo test` 全量 414 passed（含 6 新增图谱单测）；`check-graph-policy.py --self-test` PASS（ACTIVE=8）；默认扫描 PASS。补丁：`logs/checkpoints/Lane-A7-M5-W12-graph-readonly-bridge-20260907-1104.patch`。

## 0. 关键设计裁定（与 W11 计划一致，落地点）

1. **命令位置**：3 命令定义在 `bridge.rs`（非 `graph.rs`），调 `crate::graph::*` 纯内核 + `check_invocation_source`（同在 `bridge.rs`）；`graph.rs` 仍零 `crate::bridge`（守 `GRAPH_NO_SECOND_PATH`）。W10 §2.5 的“graph.rs 三命令”含糊处已纠正。
2. **`GraphState` newtype 独立托管**：`pub struct GraphState { pub store: RwLock<GraphStore> }` + `Default`。不碰 `AppState`（其 `#[derive(Default)]` 不 impl `RwLock`），避免他 Lane 冲突。
3. **错误码 1:1**：`GraphError::code()` 把 13 变体映射为 `GRAPH_*` ASCII 稳定串（W10 §2.6 曾折叠多类，本 Lane 改为 1:1 便于前端映射/测试断言）。零 secret/label/路径 echo。
4. **范围收窄为 3 只读命令**：剔除 M5-8 的 9 命令 SQLite 写/列/导出/抽取器/后台 worker；以冻结 `graph.rs`/`domain.rs` 为真相源。

## 1. 命令契约（已落地签名）

```rust
// bridge.rs
#[tauri::command] pub async fn graph_query(app, webview, req: GraphQueryRequest)
    -> Result<GraphQueryResult, String>;            // found=false 非错误；nodes/edges 为 View(无 props)；truncated+applied 信号
#[tauri::command] pub async fn graph_node_get(app, webview, id: String)
    -> Result<Option<GraphNodeView>, String>;       // 缺失 None(非错误)；空/超长 → GRAPH_INVALID_ID
#[tauri::command] pub async fn graph_stats(app, webview)
    -> Result<GraphStats, String>;                  // 计数+容量+≥90% 黄牌
```

- `graph_query_impl(store, start, depth?, limit?)`：`depth=min(req??GRAPH_DEFAULT_QUERY_DEPTH(2), GRAPH_MAX_DEPTH=4)`；`limit=min(req??GRAPH_QUERY_LIMIT, GRAPH_QUERY_LIMIT=1000)`；`total=bounded_neighbors(...,GRAPH_QUERY_LIMIT).len()`；`truncated = total>limit`；返回 `GraphNodeView`/`GraphEdgeView`（`From` 丢弃 `props`）。
- 入参 id 经 `validate_id_public`（空/超长）→ `GRAPH_INVALID_ID`；Skill/Agent 的 64-hex 完整性由 store 载入期 `from_json→validate_graph_node` 强制（查询期无需重复）。
- 错误统一 `Err(code.to_string())`，`code()` 来自 `GraphError::code()`。

## 2. 启动载入（main.rs setup）

```rust
.manage(crate::graph::GraphState::default())
// 在 .setup(|app| { ... } 末尾：
{
    let path = workspace::data_dir(app.handle()).join("graph.json");
    let store = crate::graph::load_snapshot(&path);   // 缺文件/损坏/双扫命中 → 空 store
    if let Ok(mut g) = app.state::<crate::graph::GraphState>().store.write() { *g = store; }
}
```
Tauri 在 build 期先注入 managed state 再跑 setup（全仓命令在 setup 内已用 `app.state()` 证实），故此处可安全写入。

## 3. 前端镜像（A8 W11 已写封装，本 Lane 仅修正守卫）

- `src/types.ts`：`GraphNodeView`/`GraphEdgeView`/`GraphQueryRequest`/`GraphQueryLimits`/`GraphQueryResult`/`GraphStats`（无 `props`）。
- `src/bridge.ts`：`graphQuery(req, signal?)`/`graphNodeGet(id)`/`graphStats()` 已支援 `AbortSignal`；本 Lane 修复并发引入的 `GRAPH_COMMANDS_AVAILABLE` **重复声明**（2–3 处 `export const` 会令 `npm build` 失败）并翻 `true`。`useGraphStore` 经同一常量 `backendReady` 解锁。

## 4. 策略门禁（ACTIVE 7→8）

`GRAPH_OUTPUT_NO_PROPS`：`domain.rs` 的 `GraphNodeView`/`GraphEdgeView` 结构体不得含 `props` 字段；`graph.rs` 出参须经 `GraphNodeView::from`/`GraphEdgeView::from` 且 `GraphError::code` 必须存在。扩充 GOOD_DOMAIN/GOOD_GRAPH 好样本 + 2 坏样本（View 含 props / 出参未经 View 转换），self-test 双向通过。

## 5. 并发提示

`src/bridge.ts`、`src/types.ts`、`scripts/check-graph-policy.py`、`src-tauri/permissions/default-commands.toml` 与 A8/A3 同文件并发修改。A7 具体 hunks：graph.rs（+GraphState/code/纯内核/测试）、domain.rs（+View DTO/常量）、bridge.rs（末尾 3 命令）、main.rs（manage+setup+handler）、default-commands.toml（ACL 3 行）、bridge.ts（去重+翻 true）、types.ts（+View 接口）、check-graph-policy.py（+码位+样本）。A0 据 checkpoint 隔离集成。
