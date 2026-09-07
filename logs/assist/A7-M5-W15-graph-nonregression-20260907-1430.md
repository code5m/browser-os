# A7 · M5-W15 · Graph 非回归评审说明（Release Readiness · Note）

> Lane: A7 ｜ Dispatch: `PARALLEL_COMMAND_BOARD.md` § M5-W15 Release Readiness Dispatch
> 行（2026-09-07，W15 关闭 M5 证据与 GUI 验收，不扩张运行时权限；仅 A0 push）
> 角色：**Graph non-regression review** —— 不写任何产品代码，仅产出 Graph（W12 落地、W13/W14 已合入）
> 的非回归评审说明（Note 形态）。
> 形态严守：本卡**未修改** `graph.rs` / `domain.rs` / `bridge.rs` / `main.rs` / `default-commands.toml` /
> `src/bridge.ts` / `src/types.ts` / `scripts/check-graph-policy.py` / `scripts/check-graph-ui-logic.mjs` /
> 三份主文档 / 任何 graph Vue 组件。唯一新增产物 = 本说明。
> BASE：W14 已合入，`HEAD=886ea29 feat(M5): integrate W14 plugin manager UI`。

## 1. 结论

**Graph（W12 落地、W13/W14 已合入）在 W15 Release Readiness 阶段零回归；符合全部 W15 Hard Stops，可进发布。**

支撑证据（均于 `HEAD=886ea29` 现树实跑，见 §2）：

1. **机器门禁全绿**：graph 策略脚本（后端）self-test + 默认均 PASS（ACTIVE=8）；graph UI 逻辑脚本 113/113 PASS；
   graph Rust 单测 15/15 PASS。
2. **锚点字节级未变**：W14 评审（W14 非回归说明 §2/§3）登记的全部 graph 锚点在现树中位置与形态完全一致（见 §3）。
3. **无运行时权限扩张**：graph 后端与 UI 均为**只读**——无 write/export/mutate/raw-invoke/process/网络/后台 worker；
   与 W15 Hard Stops（禁 graph write/export、raw Tauri invoke、后台 worker 等）完全不冲突。
4. **W14 Plugin Manager UI 不触碰 graph 数据路径**：W14 仅前端-only 窄 UI lane（A6），且其 `Allowed Files` 不含任何
   Rust 文件；现树 graph 后端/桥/类型/ACL 锚点未被改动（§3 + §4）。

## 2. 当前 Graph 集成证据（W15 实跑）

| 面 | 命令 / 结果 |
|---|---|
| 后端策略自检 | `python3 scripts/check-graph-policy.py --self-test` → `GRAPH_POLICY_SELF_TEST=PASS（ACTIVE=8）` |
| 后端策略默认扫描 | `python3 scripts/check-graph-policy.py` → `GRAPH_POLICY=PASS（无违规）` |
| 前端 UI 逻辑 | `node scripts/check-graph-ui-logic.mjs` → 通过 **113**，失败 0（含 `GRAPH_DEBOUNCE_MS=300`、文案不泄露 props/secret/token、截断文案非空） |
| Rust 单测（graph） | `cargo test --manifest-path src-tauri/Cargo.toml graph` → **15 passed / 0 failed**（含 `from_json_rejects_secret`、`validate_rejects_secret_in_props`、`graph_query_ready_view_omits_props`、`store_rejects_duplicate_node`、`store_enforces_node_capacity`、`graph_query_truncates_when_over_limit`、`graph_stats_approaching_capacity_flag`、`error_codes_are_stable_ascii` 等只读/脱敏/容量守卫） |

## 3. 共享锚点复验（W14 → W15 字节级一致）

| 面 | 锚点（现树位置） | W15 状态 |
|---|---|---|
| 后端模块声明 | `mod graph;` @ `src-tauri/src/main.rs:9` | 不变 |
| 后端托管 | `GraphState::default()` @ `src-tauri/src/main.rs:1347` | 不变 |
| 后端命令注册 | `bridge::graph_query` / `graph_node_get` / `graph_stats` @ `src-tauri/src/main.rs:1474-1476` | 不变 |
| 后端命令壳 | `graph_query`(6605)/`graph_node_get`(6624)/`graph_stats`(6641)，均过 `check_invocation_source` @ `src-tauri/src/bridge.rs` | 不变 |
| 后端 store/impl | `GraphStore`(185) + `graph_query_impl`(338)/`graph_node_get_impl`(371)/`graph_stats_impl`(380) @ `src-tauri/src/graph.rs` | 不变 |
| 后端域类型 | `GraphNodeKind`(2042)/`GraphNode`(2084)/`GraphEdge`(2109)/`GraphNodeView`/`GraphEdgeView` @ `src-tauri/src/domain.rs` | 不变 |
| 后端容量常量 | `GRAPH_PROPS_MAX_BYTES`(2188)/`GRAPH_LABEL_MAX_BYTES`(2190)/`GRAPH_NODE_ID_HEX_LEN`(2192)/`GRAPH_MAX_DEPTH`(2194) 等 7 个 `GRAPH_*` @ `src-tauri/src/domain.rs` | 不变 |
| 前端桥 | `graphQuery`/`graphNodeGet`/`graphStats` + `makeGraphCommandDisabledError` 守卫 @ `src/bridge.ts:439-450` | 不变 |
| 前端类型 | `GraphNodeView`(831)/`GraphEdgeView`(837)/`GraphQueryResult`(856-860) @ `src/types.ts` | 不变 |
| ACL | `graph_query`/`graph_node_get`/`graph_stats` 三行 + 末条 `list_artifact_images`(138) @ `src-tauri/permissions/default-commands.toml` | 不变 |

`git diff HEAD -- <上述文件>` 输出为空 → 自 W14 合入以来 graph 锚点文件**零改动**。

## 4. W15 Hard Stops 合规断言

逐条对照 `PARALLEL_COMMAND_BOARD.md` W15 Hard Stops（禁 plugin invoke/执行、动态加载、网络下载/监听、daemon、model call、
Agent/Skill 执行、MCP 扩张、**graph write/export**、后台 worker、raw Tauri invoke、敏感渲染/持久化）：

- [x] `src-tauri/src/graph.rs` 全文 grep `fn write|fn export|fn save|fn delete|mutate|raw_invoke|std::process|File::create|fs::write` → **0 命中**（无写/导出/变更/进程/裸写入路径）。
- [x] `src/components/graph/**` 全文 grep `invoke(|write|export|mutate|delete|POST|fetch(` → **0 命中**（UI 纯只读消费 `src/bridge.ts`，无 raw invoke、无网络、无写/导出）。
- [x] graph 三命令均经 `check_invocation_source` + ACL，无新命令、无 DTO 扩张；命令名与 DTO 冻结。
- [x] graph 后端为静态只读快照（`GraphStore` + 容量守卫 + secret 拒入），无后台 worker、无定时/轮询写入、无 graph write/export。
- [x] 无敏感渲染/持久化：graph 视图层 `graph_query_ready_view_omits_props` 已证明 props 不出现在返回 View；UI 文案经 `check-graph-ui-logic.mjs` 断言不泄露 props/secret/token。

**结论**：W15 Hard Stops 对 graph 全部满足，graph 不引入任何被锁运行时权限。

## 5. 与全量 pre-merge / 其他 lane 的关系（非 Graph 阻断项）

- A7 自身门禁（`check-graph-policy.py` + `check-graph-ui-logic.mjs` + graph cargo 单测）已全绿。
- 全量 `pre-merge.sh` 潜在的 FAIL 来自**其他并发 lane**（如 `cargo fmt`、`build metrics regression`、
  `check-plugin-policy.py` / `check-plugin-ui-logic.mjs` / `check-tools-policy.py` 等），与 Graph 无关，
  归 A0 跨 lane 集成统一处置。本卡不运行 `npm run build` / 全量 `pre-merge`（GUI 验收归 A8、最终矩阵归 A11；
  Graph 权威覆盖已由策略脚本 + Rust/node 单测提供）。
- 本 W15 评审**未改动任何产品代码**，仅新增本说明。

## 6. 工作树脏状态声明（透明）

`git status` 当前显示的未跟踪文件为**他 lane 的 W15 产物**：

- `logs/assist/A4-M5-W15-privacy-stable-error-review-20260908-0900.md`（A4）
- `logs/assist/A9-M5-W15-frozen-contract-review-20260907-1505.md`（A9）
- `logs/assist/Lane-A4-M5-W15-privacy-stable-error-review-20260908-0900.patch`（A4）

**均非 A7 引入、非 Graph 交付**。A7 不混入上述文件或任何他 lane 改动；仅新增本说明。

## 7. 交付物

- 本说明：`logs/assist/A7-M5-W15-graph-nonregression-20260907-1430.md`（唯一新增文件，docs-only Note）
- **未 push**（board：仅 A0 push）
- Graph 零回归结论：✅ 可进 M5-W15 Release
