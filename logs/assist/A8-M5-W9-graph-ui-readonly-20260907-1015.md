# A8 M5-W9 Deliverable · 图谱 UI 只读/无后端态修正（GRAPH UI SMALL）

> LANE=A8 · WAVE=M5-W9 Runtime-Free Polish · ROLE=START GRAPH UI SMALL（board line 198）
> 范围：Graph UI polish only — keep filters/search/layout deterministic, preserve bounded arrays, **improve no-backend/read-only states**. **No backend graph commands.**
> BASE=97118d6（W8 已集成：含 A8 W8 图谱 UI 打磨与 capacityState；本波在其上做小步修正）。
> 当前时间 2026-09-07；未 push（仅 A0 可 push）。

---

## 1. 本次改动（W9 SMALL，作用域严格收窄）

| 文件 | 改动 | 对应 W9 项 |
|---|---|---|
| `src/stores/useGraphStore.ts` | ① `guard()` **不再写入 `error`**——"后端未就绪"是预期态（只读壳）而非失败，修复 W8 遗留的"误把未实现渲染成红色错误横幅"；② 新增 `readOnly` 计算属性（`!backendReady`）并暴露 | 改进 no-backend/只读态 |
| `src/components/graph/GraphPanel.vue` | empty 区块按 `store.readOnly` 分流：只读壳用专属 `readonly-state` 样式（warn 色），真实错误仍走 `error-state`；错误横幅仅对真实 `error` 触发 | 改进 no-backend/只读态（UI 可视化分流） |
| `scripts/check-graph-ui-logic.mjs` | 新增 2 条断言：就绪空态文案、就绪空态≠未就绪只读态（两态可区分） | 确定性/状态可区分 |

**未改动（保持 W8 已交付、符合 W9 不写后端命令）**：`src/utils/graphUi.ts`（capacityState 等已就位，本波无需改）、`src/components/graph/{GraphViewer,GraphFilter,NodeDetail,EdgeDetail}.vue`（W8 a11y 已就位）、`src-tauri/**`、`bridge.ts`、ActivityBar/MainArea（属 W8，W9 作用域不含）。

## 2. 验证结果

| 项 | 结果 |
|---|---|
| `node scripts/check-graph-ui-logic.mjs` | **通过 43，失败 0**（W9 +2：就绪空态文案、两态可区分） |
| `npm run build`（含 vue-tsc） | **✓ built**；GraphPanel 仍为懒加载独立 chunk；主 chunk `index` 164.82 kB（未变，远低于 IF-2 阈值） |
| 类型/编译 | vue-tsc 0 错误，build 0 新增 warning |
| 确定性/有界数组 | W8 断言（filterNodes 顺序稳定、boundedInsert 6000→封顶 5000、capacityState 超量态）全部保持绿，本波未破坏 |

## 3. 关键约束自检（守 W9 Hard Stop）

- ✅ **未写后端 graph 命令**：`src-tauri/**`（`graph.rs`/`main.rs`/`domain.rs`/ACL）零改动；未新增 invoke。
- ✅ **未触碰 A3 MCP 代码**：图谱 UI 与 MCP 无耦合，未引用/修改。
- ✅ **作用域收窄正确**：仅 `src/components/graph/**`、`src/stores/useGraphStore.ts`、`scripts/check-graph-ui-logic.mjs` 有 diff；`graphUi.ts` 与其他 graph 组件零改动。
- ✅ **K7 延续**：`NodeDetail`/`EdgeDetail` 仍不渲染 props 正文。
- ✅ **无无界数组 / 无二次执行路径**：`boundedInsert` 与 `filterNodes/filterEdges` 行为未变；`guard()` 仅做布尔拦截、零 invoke。

## 4. 修复说明（W9 主要价值）

W8 集成后 `guard()` 会把"后端 graph 命令尚未就绪"写入 `error`，导致：挂载或点击刷新时，红色错误横幅 + error-state 区块出现，把"尚未实现"误报为失败。W9 修正为：
- `guard()` 仅返回 `false`，不污染 `error`；
- 新增 `readOnly` 标志，UI 用 `readonly-state`（warn 色）明确表达"只读壳"，与真实错误（红色 error-state）分流；
- `panelStateGraph` 的空态文案本就区分"就绪空态（点击刷新载入）"与"未就绪只读壳"，新增断言固化该可区分性。

## 5. 上游依赖（非本波）

- 图谱转 live 仍待 A7 落 `graph_query` 并 `GRAPH_COMMANDS_AVAILABLE=true`；届时 `guard()` 放行、`loadGraph()` 接 `bridge.graphQuery()`（W8 已预留 boundedInsert 接入点）。本波不解锁。

## 6. 输出模板回填

```text
LANE=A8
STATUS=PASS (GRAPH UI SMALL, 无后端代码)
WAVE=M5-W9 Runtime-Free Polish
BASE=97118d6
HEAD=logs/assist/A8-M5-W9-graph-ui-readonly-20260907-1015.md
FILES=src/stores/useGraphStore.ts, src/components/graph/GraphPanel.vue, scripts/check-graph-ui-logic.mjs
PATCH=logs/checkpoints/Lane-A8-M5-W9-graph-ui-readonly-20260907-1015.patch
VERIFY=node scripts/check-graph-ui-logic.mjs → 43/43 PASS; npm run build → PASS(GraphPanel 懒加载, 主 chunk 164.82kB 未超 IF-2); vue-tsc 0 错
CHECKPOINT=logs/assist/A8-M5-W9-graph-ui-readonly-20260907-1015.md
MERGE_NOTES=A8 W9 小步修正图谱 UI 只读/无后端态: ①store.guard() 不再写 error(只读壳是预期态非失败) ②store 新增 readOnly 计算属性并暴露 ③GraphPanel empty 区块按 readOnly 分流为 readonly-state(warn 色), 错误横幅仅真实 error 触发 ④测试+2(就绪空态文案/两态可区分). graphUi.ts 与其他 graph 组件零改动, 未写后端命令, 未碰 A3/MCP. 补丁作用域严格收窄(src/components/graph, useGraphStore.ts, check-graph-ui-logic.mjs)
NEXT=待 A7 落 graph_query + GRAPH_COMMANDS_AVAILABLE=true 后由 graph-live lane 接 bridge.graphQuery 并解锁 NodeDetail agent/skill 子视图(复用 A5 PermissionPreview)
```
