# A3 · M5-W18-R · Obsidian 图谱/搜索 UX 逆向研究与 Vue 复刻蓝图

> LANE=A3　WAVE=M5-W18-R（研究波，**零产品代码**）　BASE=`78d2cfb`
> 分派原文：`Reverse-engineer Obsidian graph/search UX: global/local graph, filters, groups, orphans, unresolved nodes, depth, selection, navigation, keyboard/accessibility, settings persistence, empty/error/loading states, and narrow-window behavior. Produce screen/state flows and a Vue component/state blueprint; no Obsidian assets or code copying.`
> 证据分级：**[OBSERVED]** = 本环境直接读取本地文件得到；**[BEHAVIOR-INFERRED]** = 依据 Obsidian 公开行为/通用知识推断（本环境无法运行 Obsidian，未直接观测）；**[UNKNOWN]** = 证据不足。
> 本报告不含任何产品代码改动；Obsidian 非开源代码捐献方，全部结论均为**行为复刻**口径。

---

## 0. 摘要（给 A0/A11 的 5 行结论）

1. 本地 vault 配置提供了**完整且可直接对照**的图谱设置真源（`.obsidian/graph.json` 22 键 + `app.json` 忽略过滤 + `core-plugins.json` 表面清单），**过滤/显示/力导向/孤儿/未解析/颜色分组**的语义均可精确落地。
2. 当前产品已有**相当完整的图谱底座**（3 条只读命令 + 518 行纯逻辑层 + 5 个组件 + 取消/防抖/截断/容量），差距集中在 **global 图缺失、`search` 查询 DSL 过弱、孤儿/未解析/标签/附件/颜色分组等开关缺失、设置不持久化**。
3. **用户自有笔记给出关键定调**：「图谱本身对 Agent 几乎没用（Agent 不读图）」「Graph View 只是附属红利」「Obsidian 本体不开源」→ 图谱 UX 应定为**纯人读增值面**，不得为核心功能让渡运行时权限或引入新依赖。
4. 全部 Obsidian 项只能 `REIMPLEMENT_FROM_BEHAVIOR`（无代码捐献方）；社区插件 Graph Analysis 等**不得复制**。
5. **最大阻塞**：global graph 与「随 vault 变更实时刷新」分别受限于「无全量枚举命令」与「后台 watcher 属 LOCKED 权限」，均需 A0 决策，本波只能 `DEFER` 并给方案。

---

## 1. 参考证据（精确路径与内容）

| 证据 | 路径 | 观测结果 | 分级 |
|---|---|---|---|
| 图谱视图设置 | `.../secondBrain/.obsidian/graph.json` | 22 键，见 §2 全表 | [OBSERVED] |
| 库级忽略过滤 | `.../secondBrain/.obsidian/app.json` | `userIgnoreFilters` 11 条 glob | [OBSERVED] |
| 核心插件开关 | `.../secondBrain/.obsidian/core-plugins.json` | 33 项，`graph/backlink/outgoing-link/tag-pane/global-search/...` 为 true | [OBSERVED] |
| 主题 | `.../secondBrain/.obsidian/appearance.json` | `{"theme":"obsidian"}` | [OBSERVED] |
| 工作区布局 | `.../secondBrain/.obsidian/workspace.json` | 仅含菜单标签 `graph:查看关系图谱`，**无 local graph 状态** | [OBSERVED] |
| 库规模 | vault 全库 | **8253** 个 `.md`（排除 `.obsidian`；含 `phantom-wiki` 亦为 8253，说明该目录无 md） | [OBSERVED] |
| 用户定调 | `Obsidian问答总结-开源与选型理由.md`、`Obsidian与竞品软件详细对比.md` | 「图谱只是附属红利」「Agent 不读图」「本体不开源」 | [OBSERVED] |
| 社区插件 | `Obsidian插件生态超详细梳理.md` | Graph Analysis（共引/社区检测）为**社区插件**，非核心 | [OBSERVED] |

**未观测**：Obsidian 运行时交互（本环境无法启动 Obsidian），故本地图谱设置落盘位置、快捷键细节、屏幕阅读器行为均为 [BEHAVIOR-INFERRED]/[UNKNOWN]。

---

## 2. `.obsidian/graph.json` 全键语义对照（核心证据）

| 键 | 观测值 | 语义 | 产品现有对应 | 差距 |
|---|---|---|---|---|
| `search` | `-(path:phantom-wiki AND ext:py) -(...) -(path:... AND (name:__pycache__ OR name:.venv))` | 图谱过滤查询 DSL：`path:`/`ext:`/`name:` 字段，`AND`/`OR`/`-` 取反，括号分组 | `GraphFilterState{query,kinds}`，仅**子串包含**匹配 | **DSL 缺失**（无字段运算符/取反/分组） |
| `showTags` | `false` | 是否把标签作为节点纳入图 | `GraphNodeKind` 含 `tag`（节点类型），但**无独立开关** | 缺开关 |
| `showAttachments` | `false` | 附件是否入图 | 无 | 缺 |
| `hideUnresolved` | `false` | 隐藏「链接指向但不存在」的节点 | 无（概念缺失） | 缺 |
| `showOrphans` | `true` | 显示孤儿节点（无连接） | 无（概念缺失） | 缺 |
| `colorGroups` | `[]` | 颜色分组（按路径/标签着色） | `NODE_KIND_COLOR` 仅按 kind 着色 | 缺（且本库为空 → **schema 未知**） |
| `showArrow` | `false` | 有向箭头 | 边有 `kind`，渲染未区分方向展示 | 可加 |
| `textFadeMultiplier` | `0` | 标签淡出程度 | 无 | 显示项 |
| `nodeSizeMultiplier` | `1` | 节点尺寸倍率 | 无 | 显示项 |
| `lineSizeMultiplier` | `1` | 连线粗细倍率 | 无 | 显示项 |
| `scale` | `0.6667` | 缩放级别 | 无持久化缩放 | 显示项 |
| `centerStrength` | `0.5187` | 力导向向心力 | `layoutPositions` 为**确定性布局**（非力导向） | 范式差异 |
| `repelStrength` | `10` | 斥力 | 同上 | 范式差异 |
| `linkStrength` | `1` | 连线引力 | 同上 | 范式差异 |
| `linkDistance` | `250` | 连线距离 | 同上 | 范式差异 |
| `collapse-filter` / `collapse-display` / `collapse-forces` / `collapse-color-groups` | `true/false` | UI 分区折叠态 | 无 | 纯 UI 态 |
| `close` | `true` | 视图/分区关闭态 | 无 | 纯 UI 态 |

> **关键洞察**：`centerStrength/repelStrength/linkStrength/linkDistance` 说明 Obsidian 用**力导向**布局；而本产品 `graphUi.layoutPositions` 是**确定性布局**（有 `layoutKindOf` 返回 `tree|force|cluster` 但实现为确定性坐标，且**明令不引入 d3**）。因此力导向参数只能 `REIMPLEMENT_FROM_BEHAVIOR` 且需重新设计（见 §6.4），不能直接照搬数值。

---

## 3. 屏幕/状态流（Screen & State Flows）

### 3.1 Global Graph（全库图）[BEHAVIOR-INFERRED + 配置证据]
```
[打开图谱视图] ──读取 graph.json 设置──> [渲染全库节点/边]
      │                                      │
      ├─ 过滤器分区：search DSL + showTags/showAttachments/hideUnresolved/showOrphans
      ├─ 显示分区：箭头/文字淡出/节点尺寸/连线尺寸
      ├─ 力导向分区：四项力参数（本产品为确定性布局）
      └─ 颜色分组分区：colorGroups
[点击节点] → 聚焦/展开邻居 → [再次点击/背景点击] → 取消选择
[悬停] → 高亮一跳邻居，其余淡出（textFadeMultiplier 相关）
```
状态：`idle → loading(全量枚举) → ready | empty(库空) | truncated(超容量) | error`

### 3.2 Local Graph（局部图：当前笔记 ± N 跳）[BEHAVIOR-INFERRED]
```
[当前笔记] ──depth=1(默认,可调)──> [邻居子图]
```
状态同 global，但数据源为「起点 + 深度」而非全量。
**本产品已有天然对应**：`graph_query(start_id, depth, limit)` ← `bounded_neighbors(start_id, depth, limit)`。
`GRAPH_DEFAULT_QUERY_DEPTH=2`、`GRAPH_MAX_DEPTH=4`（domain.rs:2194/2202）。

### 3.3 选择 / 导航
- 选择：本产品 `selectNode(id)` / `selectEdge(key)`，**边用稳定标识 `from|to|kind` 选择**（`edgeKey`/`resolveEdgeByKey`），过滤后不漂移 —— **优于索引选择，保留**。
- 导航：Obsidian 点击节点打开笔记 [BEHAVIOR-INFERRED]。本产品点击节点 → `NodeDetail.vue` 详情面板（不跳转）。

### 3.4 键盘 / 无障碍 [BEHAVIOR-INFERRED，Obsidian 本身较弱]
- Obsidian 图谱为 canvas 渲染，**屏幕阅读器支持弱**；本产品为 Vue DOM/SVG，**有条件做得更好**。
- 本产品已有 `selectionText`（`已选择X节点 Y`）供 `aria-live` 播报 —— **保留并扩展**。

### 3.5 空 / 载入 / 错误态
- 本产品已有 `panelStateGraph` 四态 + `readOnly`（「只读壳」预期态，与 error 分流）+ `capState`（ok/near/over）+ `truncated` 横幅 + `GRAPH_TRUNCATION_NOTICE`。**这块已达标**，只需补「过滤后无匹配」这一子空态。

### 3.6 窄窗口行为
- Obsidian 面板可折叠/停靠 [BEHAVIOR-INFERRED]。本产品 `MainArea.vue`/`StatusBar.vue` 由 A6/A7 负责；图谱侧需保证：控制分区（过滤器/显示/力导向）在窄窗口下折叠为抽屉，`collapse-*` 键正好对应此语义。

---

## 4. 当前产品现状（用于差距判定）

**后端** `src-tauri/src/graph.rs`（733 行）：
- `GraphStore::{new, insert_node, insert_edge, bounded_neighbors(start_id,depth,limit), to_json, from_json}`
- `graph_query_impl` / `graph_node_get_impl` / `graph_stats_impl` / `load_snapshot(path)`
- `validate_graph_node/edge`、`graph_props_contain_secret`、`validate_id_public`
- 常量 `domain.rs:2188-2202`：`GRAPH_PROPS_MAX_BYTES=MAX_TEXT_FIELD_BYTES`、`GRAPH_LABEL_MAX_BYTES=256`、`GRAPH_NODE_ID_HEX_LEN=64`、`GRAPH_MAX_DEPTH=4`、`GRAPH_QUERY_LIMIT=1000`、`GRAPH_MAX_NODES=5000`、`GRAPH_MAX_EDGES=20000`、`GRAPH_DEFAULT_QUERY_DEPTH=2`

**命令面**：`graph_query` / `graph_node_get` / `graph_stats`（`main.rs:1482-1484` 注册；`default-commands.toml:128-130` ACL；`bridge.ts:441-450` 类型化，**带 `GRAPH_COMMANDS_AVAILABLE` 总开关与 `makeGraphCommandDisabledError` 回滚路径**）。

**前端**：`src/stores/useGraphStore.ts`（310 行，含 300ms 防抖 + AbortController + `request_id` 乱序丢弃 + 有界 `boundedInsert`）、`src/utils/graphUi.ts`（518 行纯逻辑）、`src/components/graph/{GraphPanel,GraphViewer,GraphFilter,NodeDetail,EdgeDetail}.vue`。

**门禁**：`scripts/check-graph-ui-logic.mjs`（大量断言）、`scripts/check-graph-policy.py`。

### 差距表（Obsidian 能力 → 本产品）
| Obsidian | 本产品 | 差距级别 |
|---|---|---|
| Global graph | **无全量枚举命令**（`graph_query` 必须带 `start_id`） | 大（需新只读命令，W19） |
| Local graph | `graph_query(start_id, depth, limit)` | 小（已有，仅差 depth UI 与默认值对齐） |
| `search` DSL（path/ext/name/AND/OR/-） | 子串 `query` | 中 |
| showTags / showAttachments | 无开关 | 中 |
| hideUnresolved / showOrphans | 无概念 | 中（**派生态**，非节点类型） |
| colorGroups | 仅按 kind 着色 | 中（schema 未知） |
| 箭头/文字淡出/尺寸倍率/缩放 | 无 | 小（纯显示） |
| 力导向四参数 | 确定性布局 | 中（范式差异，需重设计） |
| 设置持久化 | **不持久化**（刷新即丢） | 中 |
| 随 vault 变更实时刷新 | 无 watcher（后台 worker 属 LOCKED） | 大（需 A0 决策） |
| 全局搜索（global-search） | **无独立搜索面** | 大（属 W19 切片） |

---

## 5. 数据流 / 控制流

```
组件(GraphPanel)
  └─ useGraphStore.loadGraph({start_id, depth, limit})
       ├─ guard(): backendReady? 否 → 零 invoke（只读壳）
       ├─ normalizeGraphQueryRequest (depth≤4, limit≤1000, request_id)
       ├─ debouncer(300ms) + AbortController.signal
       └─ bridge.graphQuery → viewToQueryResult (删 props 第三闸)
            → boundedInsert(nodes, 5000) / boundedInsert(edges, 20000)
            → truncated / applied 信号 → UI 横幅
选择：selectNode/selectEdge（稳定 key） → visibleNodes/visibleEdges（filterNodes/filterEdges） → 渲染
容量：loadStats → capState(ok/near/over) + approaching_capacity 黄牌
```
**Global graph 缺口**：无「枚举全量」路径；`graph_stats` 只给总量，不给全量节点。故 global 图在 W19 前不可实现（见 §7 分类）。

---

## 6. Vue 组件 / 状态蓝图（A3 核心交付）

### 6.1 组件树（在现有组件上增量，不重写）
```
GraphPanel.vue                    （容器：三态/横幅/分区排版）
├── GraphControls.vue    【新增】 过滤器/显示/分组/力导向四个可折叠分区（collapse-*）
│   ├── GraphFilter.vue  【改造】 query DSL 输入 + kinds + 四个开关(showTags/showAttachments/hideUnresolved/showOrphans)
│   ├── GraphDisplay.vue 【新增】 showArrow/textFade/nodeSize/lineSize/scale
│   ├── GraphGroups.vue  【新增】 colorGroups（schema 待定 → 可后置）
│   └── GraphForces.vue  【新增】 本产品为确定性布局 → 先给「布局模式」而非力参数
├── GraphViewer.vue      【改造】 SVG/Canvas 渲染 + 键盘导航 + 窄窗口抽屉
├── NodeDetail.vue / EdgeDetail.vue 【保留】
└── GraphEmpty.vue       【新增】 区分「库空 / 过滤无匹配 / 只读壳」三种空态
```

### 6.2 状态形状（Pinia `useGraphStore` 增量，**不破坏现有字段**）
```ts
// 新增：视图模式与派生开关（默认对齐观测到的 graph.json 语义）
viewMode: "local" | "global"        // global 需后端枚举命令 → 先 local-only，global DEFER
depth: number                        // 默认 GRAPH_DEFAULT_QUERY_DEPTH=2，上限 GRAPH_MAX_DEPTH=4
display: { showArrow, textFadeMultiplier, nodeSizeMultiplier, lineSizeMultiplier, scale }
visibility: { showTags, showAttachments, hideUnresolved, showOrphans }
colorGroups: ColorGroup[]            // schema 未知 → 先空数组
collapse: { filter, display, forces, colorGroups }   // 分区折叠态（窄窗口即抽屉）
derived: { orphanIds: Set<string>, unresolvedIds: Set<string> }  // 派生，不入库
layoutMode: "tree" | "cluster" | "force"             // 复用 layoutKindOf，不引 d3
```
**保留**：`nodes/edges/selectedNodeId/selectedEdgeKey/loading/error/backendReady/readOnly/startId/truncated/filter/capState/selectionText/state` 及全部 action。

### 6.3 孤儿 / 未解析必须是**派生态**
本产品 `GraphNodeKind = file|dir|tab|script|skill|agent|tag|topic` 是**类型**，而孤儿/未解析是**连接度派生属性**：
- `orphanIds` = 入度+出度均为 0 的节点（在已载入子图内计算；注意：局部图内「看起来是孤儿」不等于全库孤儿，**UI 必须标注为「当前子图内无连接」**，避免误导 —— 这是与 Obsidian 全库语义的重要差异）。
- `unresolvedIds` = 作为 `to` 出现但 `nodes` 中不存在的 id（需后端或边集合推导）。

### 6.4 布局：不引入 d3，力导向参数需重新设计
现有 `layoutPositions` 为确定性（同输入同输出，无随机）。Obsidian 四力参数属力导向范式。
**建议**：短期保留确定性布局，`GraphForces.vue` 改为「布局模式选择 + 确定性参数（如连线理想长度/聚类强度）」；若 W19 确认需力导向，再以**纯函数 + 有界迭代**实现（仍不引 d3），并补「确定性/可测」断言。Obsidian 的具体力参数数值**不可照搬**（不同引擎语义不同）。

### 6.5 持久化（隐私关键）
- 建议落 `localStorage` 的键：`viewMode/depth/display/visibility/collapse/layoutMode`。
- **禁止落库**：`search` 查询原文（本库 `search` 含 `path:phantom-wiki` 等**本地路径片段**）、`colorGroups` 中的路径规则、任何节点 label/路径。
  → 与 W17 A3 已落地的「浏览器存储只留非敏感元数据」口径一致：`toPersisted()` 式过滤同样适用于图谱设置。
- 库级忽略过滤**不复制** `app.json`（那是 Obsidian vault 配置）；本产品应复用既有路径策略（`check_path_within_roots` 等）。

### 6.6 无障碍 / 窄窗口
- 所有开关用原生 `<input type=checkbox>`/`<select>` + `<label>`；图谱容器 `role="application"` + `aria-live` 播报 `selectionText`。
- 键盘：节点可聚焦（tabindex），方向键在可见节点间移动，Enter 选中，Esc 取消；**该能力 Obsidian 弱，本产品可超越**。
- 窄窗口：四个分区自动折叠（`collapse` 全 true）为抽屉；`GraphViewer` 保持最小高度。

---

## 7. 分类（COPY / ADAPT / REIMPLEMENT_FROM_BEHAVIOR / DEFER / REJECT）

| # | 特性 | 分类 | 理由 |
|---|---|---|---|
| 1 | 全局/局部图概念 | REIMPLEMENT_FROM_BEHAVIOR | 无代码捐献方，按行为复刻 |
| 2 | Local graph (start_id+depth) | **ADAPT** | 产品已有 `graph_query`+`bounded_neighbors`，仅需 UI 对齐 |
| 3 | Global graph（全量枚举） | **DEFER** | 需新只读命令 + 超 5000 节点容量方案，待 W19/A0 |
| 4 | `search` DSL（path/ext/name/AND/OR/-） | REIMPLEMENT_FROM_BEHAVIOR | 语法可从 graph.json 精确复刻，需自研解析器 |
| 5 | showTags / showAttachments | REIMPLEMENT_FROM_BEHAVIOR | 语义明确，映射为 kind 过滤开关 |
| 6 | hideUnresolved | REIMPLEMENT_FROM_BEHAVIOR | 需先定义「未解析」派生规则 |
| 7 | showOrphans | REIMPLEMENT_FROM_BEHAVIOR | 派生态；需标注「当前子图内」以免误导 |
| 8 | colorGroups | **DEFER** | 本库 `colorGroups: []` → schema 未知，需更多证据 |
| 9 | showArrow / textFade / nodeSize / lineSize / scale | REIMPLEMENT_FROM_BEHAVIOR | 纯显示项，语义明确 |
| 10 | 力导向四参数 | **REJECT**（照搬）/ REIMPLEMENT_FROM_BEHAVIOR（重设计） | 引擎语义不同，数值不可照搬；且禁止引入 d3 |
| 11 | `collapse-*` 分区折叠 | REIMPLEMENT_FROM_BEHAVIOR | 直接对应窄窗口抽屉 |
| 12 | 选择（节点/边） | **ADAPT** | 保留现有稳定 key 选择（优于索引） |
| 13 | 键盘导航 / a11y | REIMPLEMENT_FROM_BEHAVIOR（**可超越**） | Obsidian canvas 弱，Vue DOM 可做得更好 |
| 14 | 设置持久化 | REIMPLEMENT_FROM_BEHAVIOR | 需排除路径类敏感查询（隐私） |
| 15 | 空/载入/错误/截断态 | **ADAPT** | 已有 `panelStateGraph`/`capState`/`truncated`，补「过滤无匹配」子态 |
| 16 | 随 vault 变更实时刷新 | **DEFER** | watcher 属 LOCKED 后台 worker，需 A0 决策 |
| 17 | 全局搜索（global-search 面） | **DEFER** | 无独立搜索面，属 W19 切片（与 A8 zvec-grep 路线相关） |
| 18 | 社区插件 Graph Analysis（共引/社区检测） | **REJECT** | 非核心能力；社区插件不在复制范围，且无开源代码可依 |
| 19 | Obsidian 图标/品牌/资源 | **REJECT** | 明令禁止复制资产 |
| 20 | dbx / zvec-grep 代码 | 不在本 lane | 属 A4-A9/A10 |

**计数**：COPY=0，ADAPT=4，REIMPLEMENT_FROM_BEHAVIOR=10，DEFER=4，REJECT=2（其中 1 项为「照搬力参数」被 REJECT）。

---

## 8. 安全 / 隐私 / 性能 / 许可

- **隐私**：K7 三闸已保证 props 不进 UI；新增风险是**持久化 `search` 查询含本地路径片段** → 必须过滤（§6.5）。节点 label 可能含路径 → 展示沿用 `summarizeNode` 白名单（仅 id/kind/label/neighborCount）。
- **安全**：无新增命令/权限/网络；`graph_*` 三命令已带总开关与 ACL。global 图若新增命令须走 `check_invocation_source` + ACL + bridge/types + 策略同包交付。
- **性能/容量**：库 8253 篇 > `GRAPH_MAX_NODES=5000` → global 图必然触发截断，需 `truncated` 横幅 + `clampRender`（已有）+ `capacityState` 黄牌（已有）。确定性布局在 5000 节点规模需补性能测试（[UNKNOWN]，本波未实测）。
- **许可**：Obsidian 非开源 → 只能行为复刻，禁止复制代码/图标/品牌；建议 NOTICE 中不引用 Obsidian 代码来源（A10 负责最终 BOM）。**不新增任何 npm 依赖**（明确拒绝 d3）。

## 9. 测试复用建议（W19）
- `scripts/check-graph-ui-logic.mjs`：新增断言 —— DSL 解析（path/ext/name/AND/OR/-/括号）、四开关过滤、孤儿/未解析派生、子图内孤儿标注文案、设置持久化过滤（不含 `path:`）、空态三分、确定性布局不变性。
- `scripts/check-graph-policy.py`：若新增命令/能力，补奇偶与只读守门。
- 夹具：**合成节点集**（不放真实 vault 内容），避免把用户笔记带进仓库。

## 10. 未解决问题（需 A0/A11 裁决）
1. **Local graph 设置在 Obsidian 落盘何处**：`workspace.json` 无相关状态 → [UNKNOWN]，需运行时观测或官方文档。
2. **`colorGroups` schema**：本库为空数组 → [UNKNOWN]。
3. **`search` DSL 完整语法**：仅观测到取反与 AND/OR/括号，正向条件、`tag:` 等是否支持 → [UNKNOWN]。
4. **力导向是否必要**：取决于 W19 是否接受纯确定性布局（本产品禁止 d3）。
5. **实时刷新**：是否解除 watcher 锁定（属 LOCKED 权限）。
6. **分派文本矛盾（提请 A0/A11）**：dispatch 第 1358 行写「add the dependency ... without A0 approval **after A3's adoption verdict**」，但 A3 的本波任务是 Obsidian 图谱/搜索 UX，而 zvec-grep 采用结论由 **A8** 负责 → 疑似笔误（应为 A8）。本 lane 不对 zvec-grep 依赖作出采用裁决。

---

```
LANE=A3
STATUS=PASS
BASE=78d2cfb
HEAD=<lane research commit sha>
REFERENCE_EVIDENCE=/home/ainfinit/Documents/Knowledge-Base/secondBrain/.obsidian/{graph.json,app.json,core-plugins.json,appearance.json,workspace.json};
                   vault Markdown 全库 8253 篇（只读统计）;
                   用户笔记 Obsidian问答总结-开源与选型理由.md / Obsidian与竞品软件详细对比.md / Obsidian插件生态超详细梳理.md;
                   本产品 src-tauri/src/graph.rs(733) domain.rs:2188-2202 src/utils/graphUi.ts(518) src/stores/useGraphStore.ts(310)
                   src/components/graph/*.vue main.rs:1482-1484 default-commands.toml:128-130 bridge.ts:441-450
FILES=logs/research/M5-W18/A3-obsidian-graph-search-ux-20260908-0759.md, logs/checkpoints/A3-M5-W18-R-20260908-0759.md
SOURCE_MAP=graph.json 22 键全表; GraphStore::bounded_neighbors/insert_node/insert_edge/to_json/from_json;
           graph_query_impl/graph_node_get_impl/graph_stats_impl/load_snapshot;
           graphUi: filterNodes/filterEdges/layoutPositions/panelStateGraph/capacityState/clampRender/edgeKey/viewTo*;
           useGraphStore: loadGraph(debounce+abort+request_id)/loadNode/loadStats/selectNode/selectEdge
CLASSIFICATION=COPY=0 ADAPT=4 REIMPLEMENT_FROM_BEHAVIOR=10 DEFER=4 REJECT=2
VERIFY=只读研究：无产品代码改动；未执行写操作/未改动 vault；统计与读取命令见 §1（8253 md、22 键、4 配置文件）
CHECKPOINT=logs/checkpoints/A3-M5-W18-R-20260908-0759.md
MERGE_NOTES=依赖 A1 基线（全局差距清单）与 A2（vault/链接语义，孤儿与未解析的定义应与其一致）；
            全局图需 A0 裁决是否新增只读枚举命令；实时刷新涉及 LOCKED 后台 watcher；
            搜索面与 A8 的 zvec-grep 路线选择相邻，需避免重复设计；
            分派 1358 行「A3's adoption verdict」疑似应为 A8（见 §10-6）
NEXT=待 A2 提供链接/孤儿/未解析的权威语义后收敛派生规则；待 A1 基线与 A8 路线；
     W19 建议切片：① 图谱过滤 DSL + 四开关 + 派生孤儿/未解析（纯前端，可先行）
     ② 设置持久化（含敏感过滤）③ global 图（需新命令，待 A0）④ 搜索面（与 A8 协同）
```
