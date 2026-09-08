# A3 · M5-W18-R2B：图谱/笔记联合体验与导航（Research & Design）

> 派发：`M5-W18-R2B-TASKS-20260908.md` §4「Lane A3：图谱/笔记联合体验与导航」。
> 基线：`origin/master = 434e63f`（rebase 后）。本波仅写研究/设计文档，不改产品代码、不改其他 lane、不 push。
> 证据标签遵循 R2B §1.3：`CURRENT_PRODUCT / REFERENCE_SOURCE / OFFICIAL_DOC / OBSERVED_BEHAVIOR / EXECUTED_SYNTHETIC_TEST / INFERENCE / DESIGN_DECISION`。

## 0. CONSUMED_PEERS（已消费的对等成果，固定 SHA）

| Peer | SHA | 文件 | 用于本报告的结论 |
|---|---|---|---|
| A2 R2B | `bcdfc3b` | `logs/research/M5-W18/A2-obsidian-vault-semantics.md` | §3.3 标题/块解析规则（F-block 已执行断言）；§3.7 unresolved 占位+边标志；§3.8 反链=派生逆索引；§4 库统计 8370 md / 7888 附件；§5 合成夹具已执行；§6 W19 精确目标映射（新增 `vault_graph.rs`、`Note`/`Attachment` 节点类型、`graph_import_vault` 只读命令、容量债 B1） |
| A1 R2B | `a54eed1` | `logs/research/M5-W18/A1-product-baseline-R2-20260908.md` | G1：图谱后端 `GraphStore`+前端 `GraphPanel` **已存在**，A2/A3 须叠加其上（非从零）；右栏 inline 预览模式存在（`useWorkspaceStore.openFileInline`） |
| 蓝图 | 主仓 `WORKBENCH_BLUEPRINT-20260908.md` | — | 右侧按需显示属性/结构/**反向链接/局部图谱**（L31）；笔记闭环流程「打开 md→点链接→看反链/局部图→返回原位」（L66）；S1 壳层（A1）/S3 笔记关联闭环（A2/A3 行为证据） |
| 调度抽查 | 主仓 `logs/checkpoints/A0-M5-W18-R2B-dispatch-20260908.md` | — | 第 48 条：旧文把引擎采纳裁决给了 A3，已在任务板更正——**A3 不持有 zvec 引擎采纳裁决权**；第 44 条：A2 行为证据混淆点已由 A2 R2B 修正 |

> 本 lane 既有 `A3-obsidian-graph-search-ux-20260908-0759.md`（R1+R2 附录）全部保留，本文为 R2B 补充包，不重写旧篇。

## 1. R1/R2 修正日志（承接并补齐）

- 旧 A3 R2 已把证据分级从 `[OBSERVED]` 重分类为 `REFERENCE_SOURCE / OBSERVED_BEHAVIOR / INFERENCE / CURRENT_PRODUCT / EXECUTED_SYNTHETIC_TEST`，本篇沿用并补充：
- **标题/块解析**（原 A3 标 `INFERENCE`）：A2 §3.3 + §5 夹具 `F-block` 已用执行断言证明 `[[Note#Heading]]→目标 note 锚点`、`[[Note#^id]]→块`、缺失标题**不算** unresolved → 升级为 `EXECUTED_SYNTHETIC_TEST`（消费 A2 `bcdfc3b`）。
- **节点类型**：当前产品 `GraphNodeKind = file|dir|tab|script|skill|agent|tag|topic`（`src/types.ts:798`），**尚无 `note`/`attachment`**。A2 §6.2 拟在 W19 增加 `Note`/`Attachment`。本篇设计面向「未来 `Note` 类型」与「当前 `file` 类型」两条路径，二者统一由 `node.id` 映射到 `useWorkspaceStore.openMd/openFile`。
- **容量债**：A2 B1（8370 md > `GRAPH_MAX_NODES=5000`）与旧 A3 预警一致 → 维持为跨 lane 待决项，归 A1 容量决策，非 A3 越权。

## 2. 逐行为来源登记表（R2B §4 完成包要求 2）

| 行为 | 来源标签 | 与 A2 一致性 | 独立复核 |
|---|---|---|---|
| filter（query 子串 + kind 芯片） | `CURRENT_PRODUCT`（`graphUi.filterNodes/filterEdges`、`GraphFilter.vue`） | 不涉及 A2 语义 | 已复核 |
| group（按 kind 着色） | `CURRENT_PRODUCT`（`graphUi.NODE_KIND_COLOR`）；`colorGroups` 本库为空 → `UNKNOWN`（DEFER） | A2 未定义 colorGroups schema | 待 A2 R2B+W19 |
| depth（遍历深度） | `CURRENT_PRODUCT`（`GRAPH_MAX_DEPTH=4`；`normalizeGraphQueryRequest` 默认 2） | A2 派生用同 `graphQuery{depth}` 契约 | 已复核 |
| orphan（无入出边） | `EXECUTED_SYNTHETIC_TEST`（A2 §5 `orphan-D` PASS）+ `INFERENCE`（本环境无 Obsidian 运行） | A2 §3.8/§3.9 一致：无入出边节点；反链派生不存储 | 已复核；**强化**：局部子图内孤儿须标注「当前子图内无连接」 |
| unresolved（指向不存在目标） | `EXECUTED_SYNTHETIC_TEST`（A2 §5 `unresolved:[[Missing Note]]` PASS） | A2 §3.7 一致：占位节点 + `unresolved` 边标志 | 已复核 |
| 反链（backlink） | `EXECUTED_SYNTHETIC_TEST`（A2 §5 `backlinks-A` PASS，8 入边、排除 `sub/Note` 的 `[[Alpha]]`） | A2 §3.8 一致：派生逆索引，不存储 | 已复核 |
| 标题/块链接 | `EXECUTED_SYNTHETIC_TEST`（A2 §5 `F-block`） | A2 §3.3 一致 | 已复核 |
| 附件/embed | `EXECUTED_SYNTHETIC_TEST`（A2 §5 `A embed img.png` PASS，非误判 broken） | A2 §3.6 一致：附件为引用目标，非 unresolved note | 已复核 |
| 库级忽略（userIgnoreFilters） | `REFERENCE_SOURCE`（读 `.obsidian/app.json` 11 条） | A2 §3.9 一致：两句法（库忽略 + 插件忽略） | 已复核 |
| 重命名/删除级联 | `INFERENCE`（DEFER 写路径，A2 §3.10） | A2 同 DEFER | 待 W19 事务 |

> 与 A2 不一致处：无事实性冲突；唯一差异是**节奏**——`Note`/`Attachment` 类型与 `graph_import_vault` 命令属 A2 的 W19 蓝图，本篇 UX 设计以「若 id 可解析为笔记路径则走笔记打开契约」为前提，待 A2 落地后零改动接入。

## 3. 全局图 / 局部图作为资源模型（DESIGN_DECISION）

- **全局图 = 中央文档视图**：即现有 `mainView==='graph'` 的 `GraphPanel`（`MainArea.vue:237` 挂载，`defineAsyncComponent` 懒加载）。在 A2 `graph_import_vault` 落地后，全局图 = 整库 `GraphState.store` 经 `graphQuery` 的视图。
- **局部图 = 右侧上下文工具窗口**（对应蓝图 L31）：当 `FileEditor` 打开某笔记时，右侧栏渲染以该笔记为 `start_id`、深度 2 的局部子图。复用现有 `GraphPanel` 的 `startId` 查询机制（`useGraphStore.loadGraph({start_id})`，已有 300ms 防抖+AbortController），仅换挂载容器与尺寸约束。
- **反链面板 = 派生逆索引**：对同一 `start_id`，由 `visibleEdges` 计算 `inbound = edges.where(to===id)`，纯前端 `graphUi` 函数即可（不新增后端命令），与 A2 §3.8 一致。
- **保留的现有能力**（不得破坏）：只读壳零 invoke（`backendReady=false`）、容量/截断横幅、稳定错误码映射、边稳定键选择（`edgeKey`/`resolveEdgeByKey`）、确定性布局（`layoutPositions`）、渲染有界（`clampRender`）。

## 4. 导航 / 返回 / 焦点资源契约（DESIGN_DECISION）

资源身份 `GraphFocusResource`：`{ kind:'note'|'attachment'|'other', id, path?, anchor? }`
- **单一真源**：`useGraphStore.selectedNodeId` ↔ `useWorkspaceStore.filePath` 双向绑定；选中即聚焦，聚焦即选中，避免双份状态漂移（沿用现有 `selectedNodeId`/`selectedEdgeKey` 稳定键范式）。
- **返回栈**：`useWorkspaceStore` 新增 `noteHistory: {path, anchor, scrollTop}[]`；点击编辑器内 `[[wikilink]]`→`openTarget(resource, {pushHistory:true})`；「返回」弹栈恢复 `filePath`+`anchor`+滚动位置（对应蓝图 L66 闭环）。
- **焦点规则**：图节点获得焦点时，对应笔记在编辑器内 `scrollIntoView(anchor)` 并 `aria-live` 播报；编辑器切换笔记时，图面板（全局或局部）同步高亮同 `id` 节点。
- **非笔记节点**：`attachment`→`openFile`（只读预览，不误报 unresolved）；`unresolved` 占位→显示「创建笔记？」引导（MVP 仅只读提示，不写库，呼应 A2 §3.10 DEFER）。

## 5. 节点点击 → 笔记/标题/块（消除「图能画但点不开笔记」）

解析规则（消费 A2 §3.3）：
- `node.id` 经 A2 `vault_graph.rs` 的 `sha256_hex("note:"+rel_path)` 反解回 `rel_path` → 调 `useWorkspaceStore.openMd({path:rel_path})`（当前入口 `useWorkspaceStore.ts:556`，设置 `filePath/mdHtml/mdPreview=true/layout.mainView="editor"`）。
- 若边/选择携带 `#Heading`/`#^id`（`GraphEdge.props.anchor`，A2 DTO 预留）→ 扩展 `openMd(entry, anchor?)` 并在 `FileEditor.vue` 渲染后 `scrollToAnchor(anchor)`。**此为 PROPOSED_NOT_AUTHORIZED 实现卡，本波不落地。**
- `attachment` 节点 → `openFile`（非 md 走 `layout.fileEditorOpen=true; mainView="editor"` 只读预览）。

## 6. 状态/事件/组件改动表 + 增量迁移卡

### 6.1 现有真源（保留，不重写）
| 文件:行 | 符号 | 角色 |
|---|---|---|
| `GraphPanel.vue:1-119` | 容器+横幅+起始 ID | 全局图宿主 |
| `GraphViewer.vue:1-104` | `onNode/onEdge`→`store.selectNode/selectEdge`；`clampRender` | 渲染/选择 |
| `GraphFilter.vue:1-48` | query+kinds 芯片 | 过滤（保留） |
| `NodeDetail.vue:1-33`/`EdgeDetail.vue:1-33` | 白名单摘要 | 详情（保留） |
| `useGraphStore.ts:52-310` | 状态机 `loadGraph/selectNode/selectEdge/...` | 状态机 |
| `graphUi.ts:1-518` | `filterNodes/filterEdges/edgeKey/resolveEdgeByKey/layoutPositions/boundedInsert/capacityState/...` | 纯逻辑唯一真源 |
| `useWorkspaceStore.ts:533-565` | `openFile/openMd` | 笔记打开入口 |
| `useWorkspaceStore.ts:468` | `openFileInline` | 右栏 inline 模式（局部图面板复用此模式） |

### 6.2 拟增量（PROPOSED_NOT_AUTHORIZED，仅设计）
1. `GraphPanel.vue`：新增 `mode:'global'|'local'` prop；local 模式由父级传入 `focusId`（当前笔记 id）并约束尺寸为右栏。
2. `useGraphStore`：新增 `focusResource` 计算属性（由 `selectedNodeId` 推导）、`deriveInbound(id)`（纯前端反链，复用 `visibleEdges`）、`orphanIds/unresolvedIds` 派生（复用 A2 §5 逻辑，纯函数置于 `graphUi`）。
3. `useWorkspaceStore`：新增 `noteHistory` + `openTarget(resource, {pushHistory})` + `returnToPrevious()`；`openMd(entry, anchor?)` 支持锚点滚动。
4. `FileEditor.vue`：新增 `scrollToAnchor(anchor)`（消费 `ws.anchor`）。
5. 反链面板：新组件 `BacklinksPanel.vue`（纯展示，`deriveInbound` 驱动），挂右栏。
6. 无新增 tauri 命令、无 ACL 变更、无 `graphQuery` 契约变更（与 A2 §6.4 一致）。

### 6.3 保留现有图谱能力清单
只读壳零 invoke、容量/截断/错误横幅、稳定错误码、边稳定键、确定性布局、渲染有界、懒加载——全部保留，局部图仅复用 `loadGraph({start_id})`。

## 7. 键盘 / IME / 窄窗 / 状态设计

- **键盘**：现有 `GraphViewer` 节点/边已 `tabindex=0 role=button @keydown.enter`（`:62-67,74-81`）。补充：方向键在可见节点间移动、`Esc` 取消选区、`Enter` 在笔记节点上=打开笔记（接 §5）、`Backspace`=返回上一笔记（接 §4 返回栈）。
- **IME**：输入框（`#graph-start-id`、过滤 search）保持受控 `v-model`，IME 合成期间不触发防抖查询（`compositionstart/end` 守卫，避免拼音中途误查）。
- **窄窗**：`GraphFilter` 现有单过滤条在 ≤800px 时换行（已有 `flex-wrap`）；全局图面板在窄窗沿用 `MainArea` 的 `navDensityForWidth` 裁减导航（A6 W17）。局部图面板在右栏宽度 <320px 时折叠为抽屉（复用 `openFileInline` 的右栏折叠范式）。
- **加载/无结果/失败/截断**：加载=现有 `loading` 态；过滤无匹配=新增 `panelStateGraph` 分支 `visibleCount===0 && filter!=''`→「当前过滤条件下无匹配节点」；失败=现有 `error` 红色横幅（稳定码）；截断=现有 `truncated` 横幅（A2 B1 容量债在此显形）。

## 8. 线框 + J3 可操作验收脚本（研究原型，非 native 连接，NOT_RUN）

### 8.1 线框（1440×900 主视区 + 右侧上下文栏）
```
[ActivityBar] [UnifiedTabBar]
[ Browser | Graph(mainView=graph, 全局图, 懒加载) ]   | [ 右侧上下文栏 ]
[ 起始ID输入 | 查询 | 刷新 ]                          |  若当前打开笔记 X:
[ 容量 120/5000 · 边 340/20000 ]                       |   ├ 反向链接(派生): X←A,B,C
[ 过滤: [search] [file][note][tag]… 清除 ]            |   ├ 局部图(start_id=X,depth=2)
[ svg 画布 (clampRender, 确定性布局) ]                 |   └ 点击节点 → openMd(path/#anchor)
[ 节点详情 | 边详情 ]                                  |
```
窄窗(800×600)：右侧上下文栏折叠为抽屉按钮；`GraphFilter` 芯片换行。

### 8.2 J3 验收脚本（合成，交编码 lane；本波 NOT_RUN）
- 复用 `scripts/check-graph-ui-logic.mjs`（当前 113/0）新增断言组 `nav-contract`：
  - `deriveInbound(edges, X)` 返回全部 `to===X` 的边（对照 A2 §5 `backlinks-A` 8 入边）。
  - `computeOrphanIds(nodes,edges)` 对无入出边节点返回 `{id, withinSubgraph:boolean}`。
  - `resolveNodeIdToPath(id)` 对 `sha256("note:"+rel)` 反解为 rel_path（接 A2 §6.2 id scheme）。
  - `openMd` 契约：给定 `resource{note,id,path,anchor}` 应设 `filePath/mdPreview/layout.mainView='editor'`。
- 验收口径：**无「图能画出但点不开笔记」**——每个 `Note` 类型节点必须有可达的 `openMd` 路径；导航/返回/局部上下文共享同一 `GraphFocusResource` 契约。
- 真实 GUI/原生未运行：`NOT_RUN`，旧报告 PASS 不计入本波实测。

## 9. 证据附录

- `CURRENT_PRODUCT`：取自 rebase 后工作树，符号+行号见 §6.1（`types.ts:798`、`useGraphStore.ts:556/468`、`MainArea.vue:237`、`graphUi.ts` 全表）。
- `EXECUTED_SYNTHETIC_TEST`：消费 A2 `bcdfc3b` §5（10 文件合成库，unresolved/backlinks/orphan/F-block 均 PASS）；本 lane 实跑 `check-graph-ui-logic.mjs`=113/0、`check-graph-policy.py`=PASS（与 R2 一致）。
- `REFERENCE_SOURCE`：读 `.obsidian/app.json`/`graph.json`/`core-plugins.json`（本环境只读，aggregate 不在内容）。
- `INFERENCE`/`UNKNOWN`：colorGroups schema、local 图设置落盘、Obsidian 运行时行为、zvec 引擎裁决——明确标注，A3 不裁决引擎。
- 分类（沿用并补）：`COPY=0, ADAPT=4, REIMPLEMENT_FROM_BEHAVIOR=10, DEFER=4, REJECT=2`，新增 W19 待 A2 落地的 `Note/Attachment` 接入（非新分类，属 ADAPT 接入）。

## 10. 状态与债务

- **状态：`PASS_WITH_DEBT`**（研究/设计完整，未落地产品）。
- **债务 D-A3-R2B-1（跨 lane，归 A1）**：容量 B1（8370>5000）需 A1 容量/构建决策；局部图在超量时显形截断横幅。
- **债务 D-A3-R2B-2（归 A2 W19）**：`Note`/`Attachment` 节点类型 + `graph_import_vault` 命令 + id scheme 落地后，本设计零改动接入；在此之前全局图仅含现有 `file/...` 类型，笔记闭环靠现有 `openMd(file)` 路径。
- **债务 D-A3-R2B-3（DEFER）**：重命名/删除级联、colorGroups schema、真实 GUI 验收（NOT_RUN）。
- **不越权**：A3 不持有 zvec 引擎采纳裁决（A0-M5-W18-R2B 第 48 条已更正）；本波零产品代码改动、零其他 lane 改动、零 push。

## 11. R2B 完成包自检（对照 §4 完成包 5 条）
1. 全局图=中央文档、局部图/反链=右栏上下文工具窗口，含选择/返回/焦点 ✓（§3/§4）
2. filter/group/depth/orphan/unresolved 逐行为来源登记 + 与 A2 差异独立复核 ✓（§2）
3. 节点→笔记/标题/块、键盘/IME/窄窗/四态设计 ✓（§5/§7）
4. 状态/事件/组件改动表 + 增量迁移卡 + 保留能力清单 ✓（§6）
5. 线框 + J3 可操作验收脚本（合成、NOT_RUN、标注未接 native）✓（§8）

```text
LANE=A3
STATUS=R2B_PASS_WITH_DEBT
BASE=78d2cfb
REBASE=onto origin/master 434e63f (ahead 2: R1 7dde286 + R2 877b4f5; now atop 434e63f)
HEAD=<this R2B commit sha>
CONSUMED_PEERS=A2 bcdfc3b (A2-obsidian-vault-semantics.md §3.3/§3.7/§3.8/§4/§5/§6);
                A1 a54eed1 (A1-product-baseline-R2-20260908.md G1/右栏 inline);
                WORKBENCH_BLUEPRINT-20260908.md L31/L66/S1/S3;
                A0-M5-W18-R2B-dispatch-20260908.md §3.4/§3.8/§3.10
CURRENT_PRODUCT_MAP=见 §6.1 (types.ts:798; useGraphStore.ts:556/468; MainArea.vue:237;
           graphUi.ts:1-518; GraphPanel/GraphViewer/GraphFilter/NodeDetail/EdgeDetail)
EXECUTED_CHECKS=node scripts/check-graph-ui-logic.mjs → 113 pass/0 fail;
              python3 scripts/check-graph-policy.py → GRAPH_POLICY=PASS
CORRECTION=消费 A2 R2B 将 标题/块解析、unresolved、反链 升级为 EXECUTED_SYNTHETIC_TEST;
            确认 A3 不持 zvec 引擎裁决权(A0 抽查 §3.10)
CLASSIFICATION=COPY=0 ADAPT=4 REIMPLEMENT_FROM_BEHAVIOR=10 DEFER=4 REJECT=2
            (+W19 待 A2 接入 Note/Attachment，属 ADAPT 接入，非新分类)
VERIFY=只读研究：零产品代码改动；未改 vault；未 push
BLOCKER=D-A3-R2B-1(容量 B1,归 A1); D-A3-R2B-2(Note/Attachment 类型+graph_import_vault,归 A2 W19)
MERGE_NOTES=依赖 A1 容量决策 + A2 W19 vault_graph 落地 + 蓝图 S1/S3 验收;
            A3 仅 UX/导航设计，不裁决引擎/不写产品代码
NEXT=待 A1 容量决策与 A2 W19 落地后，将 §6.2 增量卡交编码 lane 实施；J3 验收脚本接 check-graph-ui-logic.mjs
NO_PUSH=confirmed
```
