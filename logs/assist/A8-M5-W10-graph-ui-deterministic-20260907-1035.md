# A8 M5-W10 Deliverable · 图谱 UI 确定性选择 + 有界渲染（GRAPH UI SMALL ONLY）

> LANE=A8 · WAVE=M5-W10 Controlled Runtime Prep · ROLE=START GRAPH UI SMALL ONLY（board line 199）
> 范围：Continue Graph UI polish only if it does not require backend commands: **no-backend empty state, deterministic selection, bounded rendering**.
> BASE=latest master（W9 已集成：含 A8 W9 只读态修正）。当前时间 2026-09-07；未 push（仅 A0 可 push）。
> 不触碰后端 / 不依赖 A3 MCP（W10 仅 A3 可动 MCP runtime-prep）。

---

## 1. 本次改动（W10 SMALL，作用域严格收窄）

| 文件 | 改动 | 对应 W10 项 |
|---|---|---|
| `src/utils/graphUi.ts` | 新增 `edgeKey(e)`（边稳定标识 `from\|to\|kind`）+ `clampRender<T>(items,cap)` 渲染有界护栏 + `RENDER_NODE_CAP`/`RENDER_EDGE_CAP`（=数据上限 5000/20000，正常数据不触发截断，纯 UI 安全网） | 确定性选择 / 有界渲染（纯逻辑、可测） |
| `src/stores/useGraphStore.ts` | 边选择由 `selectedEdgeIdx`（可见列表索引）改为 `selectedEdgeKey`（稳定标识）；`selectEdge(key)` 按标识 toggle；`selectedEdge` 按标识在可见集合中还原；`selectNode` 清 edgeKey | 确定性选择（修复索引随过滤漂移） |
| `src/components/graph/GraphViewer.vue` | 边渲染/选择改用 `edgeKey`；渲染经 `clampRender` 有界（renderNodes/renderEdges）；超界时显示 `render-note` 提示；svg aria-label 用 `renderNodes.total` | 确定性选择 + 有界渲染（UI） |
| `scripts/check-graph-ui-logic.mjs` | +4 断言：`edgeKey` 格式/确定性、`clampRender` 未超限不截断/超限封顶 | 确定性 + 有界渲染验证 |

**未改动（符合 W10 不写后端命令、作用域）**：`graphUi.ts` 其他纯逻辑（capacityState/filter/boundedInsert 等）保持；`GraphFilter/NodeDetail/EdgeDetail.vue` 沿用 W8/W9；`src-tauri/**`、`bridge.ts`、ActivityBar/MainArea（属更早波次）。

## 2. 验证结果

| 项 | 结果 |
|---|---|
| `node scripts/check-graph-ui-logic.mjs` | **通过 47，失败 0**（W10 +4：edgeKey 格式/确定性、clampRender 不截断/封顶） |
| `npm run build`（含 vue-tsc） | **✓ built**；GraphPanel 懒加载独立 chunk；主 chunk `index` 164.82 kB（未变，远低于 IF-2 阈值 22%） |
| 类型/编译 | vue-tsc 0 错误，build 0 新增 warning |
| 残留引用 | 全仓 grep `selectedEdgeIdx` = 0（索引方案已彻底移除） |

## 3. 关键约束自检（守 W10 Hard Stop）

- ✅ **未写后端 graph 命令**：`src-tauri/**` 零改动；`bridge.graphQuery` 仅保留在 guard 后的预留分支，未解锁。
- ✅ **未触碰 A3 MCP 代码**：图谱 UI 与 MCP 无耦合，未引用/修改 `mcp_*`。
- ✅ **作用域正确**：仅 `src/components/graph/**`、`src/stores/useGraphStore.ts`、`src/utils/graphUi.ts`、`scripts/check-graph-ui-logic.mjs` 有 diff；补丁 4 文件 +71/−19。
- ✅ **K7 延续**：详情不渲染 props 正文。
- ✅ **无无界数组 / 无二次执行路径**：`boundedInsert` 数据有界（≤5000/20000）；`clampRender` 渲染有界护栏与数据上限同口径；`guard()` 仍零 invoke。

## 4. 主要价值（确定性选择修复）

W10 前边选择用 `selectedEdgeIdx`（可见列表索引）。切换过滤/排序时索引漂移，导致"选中的边"指向错误边——非确定性。W10 改为按 `edgeKey(e)=from|to|kind` 稳定标识选择与高亮：
- 过滤变化后，只要该边仍在可见集合即可还原高亮，选择不再漂移；
- 选中边被过滤掉时 `selectedEdge` 自然为 null（播报清空），过滤恢复后自动重现；
- `edgeKey` 为纯函数，已单测固化确定性。
辅以 `clampRender` 渲染有界护栏，使"渲染数组无界增长"在单点被封死（与数据上限同口径，正常不触发截断）。

## 5. 上游依赖（非本波）

- 图谱转 live 仍待 A7 落 `graph_query` 并 `GRAPH_COMMANDS_AVAILABLE=true`；届时 `guard()` 放行、`loadGraph()` 接 `bridge.graphQuery()`（W8 已预留 boundedInsert 接入点）。本波不解锁。
- W10 仅 A3 可动 MCP runtime-prep（stdio-only / feature-gated），其余 runtime 面（plugin/agent 执行）保持锁定，与本 Lane 无交集。

## 6. 输出模板回填

```text
LANE=A8
STATUS=PASS (GRAPH UI SMALL ONLY, 无后端代码)
WAVE=M5-W10 Controlled Runtime Prep
BASE=latest master (W9 已集成)
HEAD=logs/assist/A8-M5-W10-graph-ui-deterministic-20260907-1035.md
FILES=src/utils/graphUi.ts, src/stores/useGraphStore.ts, src/components/graph/GraphViewer.vue, scripts/check-graph-ui-logic.mjs
PATCH=logs/checkpoints/Lane-A8-M5-W10-graph-ui-deterministic-20260907-1035.patch
VERIFY=node scripts/check-graph-ui-logic.mjs → 47/47 PASS; npm run build → PASS(GraphPanel 懒加载, 主 chunk 164.82kB 未超 IF-2); vue-tsc 0 错; 全仓 selectedEdgeIdx=0
CHECKPOINT=logs/assist/A8-M5-W10-graph-ui-deterministic-20260907-1035.md
MERGE_NOTES=A8 W10 小步打磨图谱 UI(不写后端命令): ①graphUi 新增 edgeKey(边稳定标识) + clampRender 渲染有界护栏(RENDER_NODE_CAP/EDGE_CAP=数据上限) ②store 边选择由 selectedEdgeIdx 改为 selectedEdgeKey(按 edgeKey), 修复过滤时选中边漂移(确定性) ③GraphViewer 边渲染/选择改用 edgeKey, 渲染经 clampRender 有界, 超界显 render-note ④测试+4(edgeKey 格式/确定性, clampRender 不截断/封顶). 补丁 4 文件 +71/-19, 未碰 A3/MCP, 未碰他 lane 文件
NEXT=待 A7 落 graph_query + GRAPH_COMMANDS_AVAILABLE=true 后由 graph-live lane 接 bridge.graphQuery 并解锁 NodeDetail agent/skill 子视图(复用 A5 PermissionPreview)
```
