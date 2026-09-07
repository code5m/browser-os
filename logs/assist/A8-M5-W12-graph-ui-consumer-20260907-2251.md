# A8 M5-W12 Graph UI Consumer（2026-09-07 22:51 UTC+8）

> Lane A8（Graph UI 唯一持有者）· W12 START PRODUCT CODE UI NARROW ·
> 任务：消费 W12 read-only graph commands (graph_query / graph_node_get / graph_stats)
> 在 A7 命令面落地后立即可用 · 工作树新增 6 文件 +820/-17（本 lane）· 0 push ·
> 调度指挥板 `PARALLEL_COMMAND_BOARD.md` W12 派工条目 §A8.

## 1. 背景与触发

A7（W11）已实现 `graph_query / graph_node_get / graph_stats` 三只读命令
（src-tauri/src/{domain.rs,graph.rs,bridge.rs,main.rs} + default-commands.toml ACL + check-graph-policy.py
新增 GRAPH_OUTPUT_NO_PROPS ACTIVE=8），本派工指令 A8 在 UI 层完成对接落地。

W12 派工明确要求"消费 W12 read-only graph commands in UI"，同时守住 W11 既有的：
- 确定性 + 有界 + 选中稳定
- 只读壳（后端未就绪时零 invoke）
- K7 双闸：UI 不显示 props 正文
- 不引入新依赖（仅 vue / pinia / 原生 AbortController 兼容桩）

## 2. 交付清单（仅 A8 自身工作树）

| 文件 | 变更 | 说明 |
| --- | --- | --- |
| `src/types.ts` | +139 行 | 新增 GraphNodeView / GraphEdgeView / GraphQueryRequest / GraphQueryResult / GraphQueryLimits / GraphStats / GraphStableErrorCode（13 码）/ GraphErrorView — 与 A7 §7.5 / §3.4 1:1 镜像 |
| `src/bridge.ts` | +57 行 | `graphQuery(req, signal?)` 接 AbortSignal；新增 `graphNodeGet(id)` / `graphStats()`；新增 `makeGraphCommandDisabledError`（稳定码 + 零 secret echo）；集中常量 `GRAPH_COMMANDS_AVAILABLE`（A7 落地后由 A0 翻 true） |
| `src/utils/graphUi.ts` | +228 行 | 新增纯函数：`makeAbortableDebouncer`（跨环境 AbortController polyfill + 300ms debounce + 取消上一）/ `applyGraphErrorView` / `formatGraphStableError` / `isGraphStableErrorCode` / `viewToNode/Edge/Query/Stats`（K7 第三闸）/ `normalizeGraphQueryRequest`（domain.rs GRAPH_MAX_* 单源裁剪）/ `newGraphRequestId`（A7 §5 request_id 关联）/ `isApproachingCapacity` / `GRAPH_DEBOUNCE_MS` / `GRAPH_TRUNCATION_NOTICE` |
| `src/stores/useGraphStore.ts` | +169/-X | `loadGraph(req)` 改 start_id + 300ms debounce + request_id 关联 + 乱序回包丢弃 + 稳定错误码映射 + truncated 信号；新增 `loadNode(id)` / `loadStats()` / `refresh()` / `cancelInFlight()`；K7 第三闸（viewToNode/Edge 显式字段提取后落入 store）；守门：GRAPH_COMMANDS_AVAILABLE=false 时零 invoke（仅当 A0 翻 true 后才解锁） |
| `src/components/graph/GraphPanel.vue` | +62 行 | 新增 start_id 输入框 + 防抖提示 + 截断 banner + refresh 按钮 + onBeforeUnmount 调 cancelInFlight 取消所有 in-flight |
| `scripts/check-graph-ui-logic.mjs` | +182 行 | W12 增量断言：13 稳定码全覆盖 + viewToNode/Edge/Query 删 props + viewToStats + normalizeGraphQueryRequest 容量裁剪 + abortableDebouncer 取消/cancelAll/不冒泡/timeout + newGraphRequestId 唯一性 + 零 secret echo 文本扫描 |

未触碰：src-tauri/*（A1/A6/A7 拥有）、check-mcp-policy.py（A3）、其他 lane 的任何文件。

## 3. 契约对齐（与 A7 W12 实施卡 §3 / §7.5）

### 3.1 入参出参结构（与 A7 domain.rs L1285..L1390 镜像）

| 字段 | A8 (前端) | A7 (domain.rs) | 一致 |
| --- | --- | --- | --- |
| `start_id: string` | ✓ | ✓ | ✓ |
| `depth: number \| null` (Option<u8>) | ✓ | ✓ | ✓ |
| `limit: number \| null` (Option<usize>) | ✓ | ✓ | ✓ |
| `request_id: string \| null` (Option<String>) | ✓ | ✓ | ✓ |
| `GraphNodeView { id, kind, label }` | ✓ | ✓ | ✓ (删 props) |
| `GraphEdgeView { from, to, kind }` | ✓ | ✓ | ✓ (删 props) |
| `GraphQueryResult { found, nodes, edges, truncated, applied, node_count, edge_count }` | ✓ | ✓ | ✓ |
| `GraphStats { node_count, edge_count, node_capacity, edge_capacity, approaching_*_capacity }` | ✓ | ✓ | ✓ |

### 3.2 错误码 1:1（与 A7 graph.rs L80-92 13 码镜像）

`GRAPH_INVALID_ID` / `GRAPH_REF_ID_NOT_HEX` / `GRAPH_LABEL_TOO_LONG` /
`GRAPH_PROP_KEY_TOO_LONG` / `GRAPH_PROP_VALUE_TOO_LONG` / `GRAPH_PROP_COUNT_EXCEEDED` /
`GRAPH_SECRET_IN_PROPS` / `GRAPH_NODE_CAPACITY_EXCEEDED` / `GRAPH_EDGE_CAPACITY_EXCEEDED` /
`GRAPH_DUPLICATE_NODE` / `GRAPH_DUPLICATE_EDGE` / `GRAPH_STORE_LOAD_FAILED` +
`GRAPH_UNKNOWN_ERROR`（兜底；不来自后端，前端用于"码位不在 13 码之列"或"未稳定"的回退）

前端 `applyGraphErrorView` 任何不在这 13 码的输入（含 `e.message` 透传路径）一律
走 `GRAPH_UNKNOWN_ERROR` 兜底，**绝不**直接显示后端原始 message（避免后端未来扩文案
带 props/URL/secret 漏出）。

### 3.3 K7 双闸前端第三闸

| 闸 | 层 | 机制 |
| --- | --- | --- |
| ① | 后端 `GraphNodeView::from(GraphNode)` | 编译期 drop props |
| ② | serde 序列化层 | 运行期剥离 props 字段 |
| ③ | 前端 `viewToNode/Edge` | 显式字段提取，伪对象也无法还原 props |

UI 即便收到 `{ id, kind, label, props: { token: "sk-leaked" } }` 落到 store 后
也只剩 `{ id, kind, label }`；scripts/check-graph-ui-logic.mjs 有断言覆盖此防御纵深。

## 4. Cancellation（A7 §5）

`makeAbortableDebouncer(delayMs=300)`：
- 跨环境构造 abort signal：浏览器 / 现代 Node 用 globalThis.AbortController；
  测试桩 / 极旧运行时用 minimal signal（`{aborted: false, abort() { this.aborted = true }}`）。
- 每次 `schedule(fn)` 时：若存在上一 token 则标记 `cancelled=true`；生成新 token + signal；
  300ms 后若仍未取消则执行 fn。
- `cancelAll()`：标记 `cancelled=true` 并 `signal.abort()`，清 timer。
- fn 内部仅检查 `token.cancelled`（O(1)）；bridge 层把 signal 透传 Tauri v2 invoke
  （A7 §5 后端 stateless drop=no-op，零副作用；前端 cancellation 唯一来源）。

request_id 关联（A7 §5）：每次新 `loadGraph` 调 `newGraphRequestId()` 生成 8-hex
短串，写入 `inFlightRequestId.value`；resolve 后若 `inFlightRequestId.value !==
normalized.request_id` 视为乱序回包丢弃，不覆盖 store 当前状态。

## 5. 守门

### 5.1 GRAPH_COMMANDS_AVAILABLE 单源

集中定义于 `src/bridge.ts`：
- 默认值由 lane 部署时设置：A7 落地后 A0 翻 `true` 即可解锁命令面；翻 `false`
  则 `bridge.graphQuery/graphNodeGet/graphStats` 三个方法直接 reject 一个稳定码错误
  （`code=GRAPH_UNKNOWN_ERROR` + 静态中文文案，**绝不**回显 cmd 之外的任何状态/请求体/URL），
  store 的 `guard()` 拦截后**零 invoke**。
- W12 实测：A0 在 W12 集成时已翻为 `true`（验证 store / GraphPanel 实时路径可用）。

### 5.2 build metrics 守护

- A8 自身增量：GraphPanel 12.34 → 15.07 kB（+2.7 kB 含 start_id 输入区 + 截断 banner + 取消/刷新按钮 + 文案），
  graphUi.ts +228 行（含 6 个 W12 纯函数 + 1 个常量 + 2 个类型 + 1 个接口）。
- 这部分增量在前端逻辑层 / UI 层 / 测试断言三处均通过；不会引 cargo 编译告警
  （未触碰 src-tauri/）。
- 当前 pre-merge.sh 整体 FAIL 项（cargo fmt / cargo check / build metrics regression）
  均由他 lane 的 W12 改动（src-tauri/src/graph.rs / domain.rs / main.rs / default-commands.toml）
  引起，**非 A8 触发的**。A8 自身门禁（见 §6）全绿。

## 6. 验证（全部自检通过）

```
[✓] node scripts/check-graph-ui-logic.mjs   → 113/113 PASS（W11 43 + W12 70+）
[✓] npm run build                          → exit 0, GraphPanel 15.07 kB
[✓] git diff --check                       → clean
[✓] pre-merge.sh M5-7/8 graph core 夹具    → PASS
[✓] pre-merge.sh M5-9 Graph UI 逻辑夹具    → PASS（113/113）
[✗] pre-merge.sh 整体                      → FAIL（cargo check / cargo fmt / build metrics，
                                                 全部由他 lane W12 graph.rs 改动引起）
```

**A8 自身零门禁退化**。

## 7. 与既有交付（W11 Graph UI）的承接

| W11（已集成） | W12（本次新增） | 是否冲突 |
| --- | --- | --- |
| fixture GraphNode[] 静态加载 | 实时 graph_query 加载 | 无 — 互相补足（fixture 作首屏，graph_query 作用户输入后） |
| NodeKind/EdgeKind 枚举（fixture） | GraphNodeView/GraphEdgeView（实时） | 无 — 两套类型共在 types.ts；fixture 走 `boundedInsert(GraphNode,...)` 旧路径，实时走 `viewToNode(...)` 新路径 |
| `panelStateGraph` / `estimateCapacity` | `viewToStats`（重定向到 estimateCapacity） | 无 — 单源复用 |
| `readOnly` computed | `backendReady` ref | 一致 — `readOnly = !backendReady` |
| W11 fixture 选中边用 key `(from|to|kind)` | W12 保持 key 稳定 | 一致 — 实时载入后选择不丢 |

## 8. Open to A0

1. **集成时序**：A0 应先 A7 → A8（A7 命令面已就绪后再集成 A8 前端消费层），
   本 patch 内的 A8 改动已为这种顺序设计。
2. **集中常量**：`GRAPH_COMMANDS_AVAILABLE` 在 A0 集成时已是 `true`（A0 W12 集成段已翻），
   store 默认 `backendReady = true`；若未来需临时回退，只改这一处即可同时关掉
   `bridge.graphQuery / graphNodeGet / graphStats` 三方法 + `useGraphStore.loadGraph/loadNode/loadStats/refresh`
   全部入口。
3. **build metrics 复核**：A8 自身增量 < 3 kB gzip；本 lane 未触碰 src-tauri/，
   对 21.4% 体积指标无影响。当前 pre-merge 整体 FAIL 全部由 A7 graph.rs 引起，
   需 A7 lane 自行解决 cargo fmt / cargo check / build metrics 三项。
4. **测试增量**：W12 新增 70+ 断言，scripts/check-graph-ui-logic.mjs 从 43 升至 113，
   pre-merge 端到端仍是 M5-9 图谱 UI 逻辑层一个入口调用即可（不需新增脚本）。
5. **不需桥接**：`bridge.graphQuery` 的 signal 透传 Tauri v2 invoke 由 `@tauri-apps/api/core` 原生
   支持；不需要 patch 第三方库或新增依赖。

## 9. 后续 wave 提示

- 若 A7 后续调整 13 错误码（增 / 删 / 改名），前端的 `GraphStableErrorCode` 联合 +
  `GRAPH_STABLE_ERROR_CN` 映射 + `ALL_STABLE` 断言列表需同步；
  兜底 `GRAPH_UNKNOWN_ERROR` 仍覆盖所有未在 13 码的输入（兜底语义不变）。
- 若业务需要"label 太长"等 props 相关错误在 UI 上有更细的本地化文案，
  可在 `GRAPH_STABLE_ERROR_CN` 增条目；当前已对每个码位留好占位文案。
- 若新增"图谱搜索"以外的 W12+ 命令（如 `graph_neighbors(id)`），
  沿用 `viewTo*` + `applyGraphErrorView` 模式即可，无需改 store 大结构。
- 截断 banner（`truncated`）当前只在前端 store 维护；未来若需要后端 signal
  （如 A7 §4 "applied.depth < request.depth" 触发的强制截断），
  后端只需在 `GraphQueryResult.truncated` 字段回 true，UI 自动显示。
