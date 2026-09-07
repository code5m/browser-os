# A8 M5-W8 Deliverable · 图谱 UI 纯逻辑打磨（UI POLISH/TEST ONLY）

> LANE=A8 · WAVE=M5-W8 Excluding-A3 · ROLE=START UI POLISH/TEST ONLY（board line 197）
> 范围：Review and polish Graph UI pure logic already landed — accessibility labels, empty/error/oversize states, deterministic filters/search, no unbounded arrays. **Do not add backend graph commands.** No MCP/A3 work.
> BASE=6c1f30e（mainline 已含 A3 W7 MCP 桥，但本 Lane 不触碰）；本交付仅作用于前端图谱 UI 壳（W6 已集成）。
> 当前时间 2026-09-07；未 push（仅 A0 可 push）。

---

## 1. 本次改动清单（均不触碰后端 / 不依赖 A3）

| 文件 | 改动 | 对应 W8 项 |
|---|---|---|
| `src/utils/graphUi.ts` | 新增 `capacityState(cap)` 纯逻辑（ok/near/over 超量态，确定性） | 空/错误/**超量**态 |
| `src/stores/useGraphStore.ts` | 暴露 `capState`（容量健康度）、`selectionText`（选择播报） | 超量态 + aria 播报 |
| `src/components/graph/GraphPanel.vue` | `role=region`+aria-label；刷新按钮 aria-label；横幅加 `role=status/alert`+`aria-live`；**新增超量横幅**；独立 empty/error 区块 | 无障碍 + 空/错误/超量态 |
| `src/components/graph/GraphViewer.vue` | 节点加 `aria-pressed`/`aria-roledescription`；边加 `tabindex`/`role=button`/`aria-label`/`@keydown.enter`；新增可视隐藏 `aria-live` 选择播报区 | 无障碍 |
| `src/components/graph/GraphFilter.vue` | `.filter` 加 `role=search`；chips 容器 `role=group`+aria-label；chip/clear 补 aria-label | 无障碍 |
| `src/components/graph/NodeDetail.vue` / `EdgeDetail.vue` | 详情区 `role=region`+`aria-live=polite`；空态 `role=status` | 无障碍 |
| `src/components/layout/ActivityBar.vue` | 顶栏 + ☰菜单（含**图谱 tab**）按钮补 `aria-label`/`aria-current=page` | 无障碍（图谱入口） |
| `src/components/layout/MainArea.vue` | 图谱视图容器 `role=region`+`aria-label="知识图谱"` | 无障碍 |
| `scripts/check-graph-ui-logic.mjs` | 新增 7 条断言：capacityState ok/near/over、确定性搜索（不放大数组/顺序稳定）、boundedInsert 6000→封顶 5000 | 超量态 + 确定性 + 无界防护 |

## 2. 验证结果（W8 交付门槛：`npm run build` 或聚焦 UI 逻辑 PASS）

| 项 | 结果 |
|---|---|
| `node scripts/check-graph-ui-logic.mjs` | **通过 41，失败 0**（原 34 → +7） |
| `npm run build`（含 vue-tsc 类型检查） | **✓ built** |
| GraphPanel chunk | 10.60 kB（懒加载独立 chunk，较 W6 9.12 kB +1.48 kB，仍异步） |
| 主 chunk `index` | 164.82 kB（仅因 ActivityBar/MainArea 加 aria 微增，远低于 IF-2 阈值） |
| 类型/编译 | vue-tsc 0 错误；build 0 warning 新增 |

结论：W8 图谱 UI 打磨在 6c1f30e 上完整、无回归、无新类型/构建告警。

## 3. 关键约束自检（守 W8 Hard Stop）

- ✅ **未写后端 graph 命令**：`src-tauri/**`（`graph.rs`/`main.rs`/`domain.rs`/ACL）零改动。
- ✅ **未触碰 A3 MCP 代码**：MCP 桥（`mcp_policy_get` 等）未被本 Lane 引用或修改（图谱 UI 与 MCP 无耦合）。
- ✅ **未触碰其他 lane 脏文件**：`src-tauri/src/plugin.rs`(MM)、`scripts/check-agent-skill-policy.py`(M)、文档与 board 等均未编辑；补丁按文件作用域裁剪，仅含本 Lane 10 文件（+105/−16）。
- ✅ **K7 延续**：`NodeDetail`/`EdgeDetail` 仍不渲染 `props` 正文；`summarizeNode/Edge` 摘要不含 props（W7 评审 F4/F5 已确认）。
- ✅ **无二次执行路径 / 无无界数组**：`boundedInsert` 仍封顶 `GRAPH_MAX_NODES/EDGES`；`filterNodes/filterEdges` 返回子集（新增 6000 节点压测断言）。

## 4. 上游依赖（非本波范畴，记录供 W8/W9）

- 图谱转 live 仍待 A7 落地 `graph_query` 命令 + `GRAPH_COMMANDS_AVAILABLE=true`；agent/skill 节点详情届时复用 A5 只读 `PermissionPreview`（见 W7 评审笔记 F3/F6）。本 W8 仅打磨壳层，不解锁 live。

## 5. 输出模板回填

```text
LANE=A8
STATUS=PASS (UI POLISH/TEST ONLY, 无后端代码)
WAVE=M5-W8 Excluding-A3
BASE=6c1f30e
HEAD=logs/assist/A8-M5-W8-graph-ui-polish-20260907-0945.md
FILES=src/utils/graphUi.ts, src/stores/useGraphStore.ts, src/components/graph/{GraphPanel,GraphViewer,GraphFilter,NodeDetail,EdgeDetail}.vue, src/components/layout/{ActivityBar,MainArea}.vue, scripts/check-graph-ui-logic.mjs
PATCH=logs/checkpoints/Lane-A8-M5-W8-graph-ui-polish-20260907-0945.patch
VERIFY=node scripts/check-graph-ui-logic.mjs → 41/41 PASS; npm run build → PASS(GraphPanel 10.60kB 懒加载, 主 chunk 164.82kB 未超 IF-2); vue-tsc 0 错
CHECKPOINT=logs/assist/A8-M5-W8-graph-ui-polish-20260907-0945.md
MERGE_NOTES=A8 W8 仅打磨前端图谱 UI 壳: ①graphUi.ts 增 capacityState 超量态纯逻辑 ②store 暴露 capState/selectionText ③GraphPanel 补 aria 语义+超量横幅+独立 empty/error 块 ④GraphViewer 节点 aria-pressed/边键盘可达+选择播报 ⑤GraphFilter role=search/group+aria ⑥Node/EdgeDetail role=region+aria-live ⑦ActivityBar/MainArea 图谱入口 aria-label ⑧测试 +7 断言(超量态/确定性/无界防护). 未写后端 graph 命令, 未碰 A3 MCP, 未碰他 lane 脏文件; 补丁按文件裁剪仅本 Lane 10 文件
NEXT=待 A7 落 graph_query 命令且 GRAPH_COMMANDS_AVAILABLE=true 后, 由 graph-live lane 将 loadGraph 接 bridge.graphQuery 并解锁 NodeDetail agent/skill 子视图(复用 A5 PermissionPreview)
```
