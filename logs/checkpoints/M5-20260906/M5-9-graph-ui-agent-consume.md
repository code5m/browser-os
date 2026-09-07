# M5-9 图谱 UI + Agent 消费（力导向 + RAG 注入）

> 子卡 ID：**M5-9** · 需求 #13 · `[S3|LEVERAGE:1|COMPLEX|AI:NORMAL|R:high]`
> 责任 Lane 候选：**A19**
> 父卡：`详细设计与实施计划.md` L572（`M5-9 图谱 UI 与 Agent 消费`）
> 主预研：暂无 prework 文档（A8 prework 仍空）
> 配套：`M5-8-graph-store-query.md`（DTO 稳定）
>
> **W3** BLOCKED（待 A7 W5 schema 落地）· **W4** ACTIVE（A8 W4 仍 SUPPORT DOCS ONLY）· **W5** ACTIVE（A8 W5 仍 SUPPORT DOCS ONLY，待 A7 W5 schema 落地后 W6+ 由 A0 决定）· **W6** PUSHED · **A8 升级为 START PRODUCT CODE** · `5f92ece` 拣入（4 vue + store + utils + `scripts/check-graph-ui-logic.mjs` 193 行 PASS）· 详见本卡顶部 `[W6 next-card acceptance criteria]` 段）· **W7** RECONCILIATION（**A1 W7 标 F1 Critical 在本卡 [W7 patched] 段订正**：`summarizeNode` 白名单收紧 8→4 字段 `{id,kind,label,neighborCount}` + NodeDetail/EdgeDetail 8→4+`props_size` 字段 + §4.2 fixture 8 字段 W8+ 取决；A3 W7 mcp_* 3 命令 + A11 W7 pre-merge FAIL 3 red lights + A6 W7 wiring 在 `6c1f30e` / `daa10f6` / `a29b796` 已拣入 master；A1 W7 整包本卡修订未进 master，留 W8 整包合并拣入；详见本卡顶部 `[W7 patched · 2026-09-07 00:50 CST]` 段 + `M5-0-overview.md` 顶部 `[W7 reconciliation · 2026-09-07 09:45 CST]` 段）· **W8** ACTIVE（**A8 W8 = UI POLISH/TEST ONLY** —— 复审 + 磨光已落地 Graph UI pure logic：accessibility labels / empty/error/oversize states / deterministic filters / search / 无 unbounded arrays；**不**加后端 graph commands；A7 W8 = GRAPH BRIDGE PLAN ONLY 仅 docs/A7 card 规划 read-only graph query command + GraphPanel 消费既有 graph store；A3 W8 = HOLD/NO ASSIGNMENT；详见 `M5-0-overview.md` 顶部 `[W8 active · 2026-09-07 09:45 CST]` 段 + `M5-13-verification-matrix.md` 顶部 `[W8 verification scope]` 段）
> **W8 reconciliation**（2026-09-07 14:30 CST · A0 拣入）：A0 在 **`4d7be97 feat(M5): integrate W8 command bridge polish`**（53 files +6038 -101）+ **`94e763e fix(M5-W8,A3): close MCP policy phase debt`**（MCP policy phase debt 关闭 · `MCP_NO_RMCP_SERVER` + `--expect-current-gaps` gate + ACTIVE=8 PENDING=0）+ **`a840fcb docs(A11): M5-W8 verification delta — functional GREEN, 3 red lights all trace to A3 W7/W8`**（A11 W8 verification delta 功能性 ALL_PASS）+ **`97118d6 chore(M5): normalize W8 patch evidence whitespace`**（3 份 W8 patch 文件空白规范化）4 commit 中拣入本卡 W8 修订：① A8 W8 graph UI polish 4 vue 文件（EdgeDetail/GraphFilter/GraphPanel/GraphViewer/NodeDetail + ActivityBar/MainArea + useGraphStore + graphUi + check-graph-ui-logic.mjs 41 断言）落地；② A1 W7 reconciliation 整包合并拣入（`summarizeNode` 白名单 8→4 字段 `{id,kind,label,neighborCount}` + NodeDetail/EdgeDetail 8→4+`props_size` 字段在 4d7be97 内吸收）；③ A1 W8 reconciliation 整包合并拣入（本卡头部 + [W8 active] 段 + 顶部 history 链更新）；**A1 W7 + W8 整包合并拣入 9 + 11 = 20 文件均含本卡修订**。
> **W9** ACTIVE（**A8 W9 = GRAPH UI SMALL** —— 图谱 UI 磨光：filter/search/layout 确定性 + 保留 bounded arrays + 改进 no-backend/read-only 状态；**不**实施后端 graph command；A7 W9 = GRAPH CONTRACT DOCS ONLY 图谱桥契约终稿，runtime-free 与 blocked backend runtime 分离；A3 W9 = POLICY/REVIEW ONLY MCP policy current-phase green + 后续 M5-2.b 卡预备，不实施 rmcp server/listener/network；详见 `M5-0-overview.md` 顶部 `[W9 active · 2026-09-07 14:30 CST]` 段 + `M5-13-verification-matrix.md` 顶部 `[W9 verification scope]` 段）
> **W10** ACTIVE（**A8 W10 = GRAPH UI SMALL**（no-backend 继续：empty state / deterministic selection / bounded rendering）；**A7 W10 = GRAPH DOCS ONLY**（live-query 实施卡预备：command names / result limits / cancellation / privacy；backend runtime 仍 BLOCKED，不实施命令）；**graph live-query command 仍 LOCKED**（W10 runtime-lock 状态表）；详见 `M5-0-overview.md` 顶部 `[W10 active · 2026-09-07 16:00 CST]` 段 runtime-lock 状态表）

---

## [W5 status · 2026-09-06 18:35 CST] A8 W5 仍 SUPPORT DOCS ONLY（待 A7 W5 schema 落地后 W6+ 派发）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L158（**A8 M5-W5** *"Prepare graph UI after A7 schema lands; no UI code in W5."*）+ L134-170 硬约束。
> **A1 W5 角色（轻量）**：A1 W5 **不**改 §1~§11 决策史；仅在头部加本 `[W5 status]` 段，**说明 A8 W5 仍 SUPPORT DOCS ONLY**；M5-9 决策史 / §3 WRITE 9 条命令列表保持 W0 原文不动，**待 A7 W5 schema 落地后由 A0 决定 M5-9 何时派发实施期**。
> **A8 W5 应做的（轻量）**：
> ① 重读 A7 W4 graph core delta（`logs/assist/A7-M5-W4-checkpoint-20260906-1455.md` + `A7-M5-W4-graph-core-delta-20260906-1455.md`，`1610939` 拣入）确认 W5 schema 边界
> ② 重读 A7 W5 output（A7 W5 实施期产出物）确认 DTO/容量/redaction/pure store 实际形态
> ③ 在 `logs/assist/A8-M5-W5-*.md` 出 W5 graph UI card delta（**仅 docs**，**不**写 UI 代码），**不**碰 `src/components/**` / `src/stores/**` / `src/types.ts` / `src/bridge.ts`
> ④ 在本卡 `M5-9-graph-ui-agent-consume.md`（如需）补 W5 status 行（**不**动 §1~§11）
> **A8 W5 不应做的**：① 写 D3.js 力导向布局代码 ② 写 `useGraphStore.ts` 状态管理 ③ 写 `GraphViewer.vue` 组件 ④ 改 §1~§11 决策史 ⑤ 改本根卡（M5-0）/M5-7/M5-8 任何 AC 段（避免与 A7 实施期冲突）
> **A1 W5 不修订范围（本卡）**：
> - **§1 GOAL / §2 READ / §3 WRITE / §4 关键契约 / §5 FORBID / §6 COMMANDS / §7 PASS_CRITERIA / §8 FAIL_ACTION / §9 DOC_BACKWRITE / §10 COMMIT / §11 FORBID 遵守记录**：A1 W5 **不动**（决策史保持 W0 原文；A8 W5 status 在本顶部段单列）。
> - **三份主文档 / ACL / Capability / pre-merge.sh / scripts/**：A1 W5 不动。
> - **`NEXT` 标记**：A0 调度权；A1 不改字面值。
> - **A1 W5 强停止**：本卡本轮**仅**加本 `[W5 status]` 段 + 头部状态行；**不**写 next-card AC（与 M5-6/7/8 不同——M5-9 在 W5 没有产品代码 lane 承接）。

---

## [W7 patched · 2026-09-07 00:50 CST] A1 W7 订正 §[W6 next-card AC] AC-1 `summarizeNode` 白名单 8→4 字段 + §4.2 NodeDetail.vue 8字段 生产路径标 W8+ 取决（来源：A7 W6 review 红线 F1 Critical · `logs/assist/A7-M5-W6-graph-contract-review-20260906-2239.md`）

> **依据**：`logs/assist/A7-M5-W6-graph-contract-review-20260906-2239.md` F1（**Critical**）—— *"M5-9 卡 W6 AC-1 的 `summarizeNode` 白名单含 5 个 A7 DTO 不存在的字段（source/source_ref/created_at/updated_at/extractor_version），且 `id=sha256` 不可逆无法派生，须由 A1 修订卡；W6 `summarizeNode` 实际仅可用 `{id, kind, label, neighborCount}`。"* + `5f92ece` 拣入实测 A8 W6 实际实现（`scripts/check-graph-ui-logic.mjs` 193 行 PASS）已基于 4 字段，无生产路径泄露。
> **F1 订正（本卡修订 · A8 代码**无需回改**）**：
> 1. **AC-1 `summarizeNode` 白名单收紧 8→4 字段**：原 AC-1 列 `{id, kind, label, source, source_ref, created_at, updated_at, extractor_version}` —— 后 5 字段 `source/source_ref/created_at/updated_at/extractor_version` **不在 A7 DTO**（A7 W5 GraphNode = `{id, kind, label, props}`），且 `id=sha256` 不可逆**无法派生** source/时间戳/extractor_version。**W7 修订 AC-1 白名单 = `{id, kind, label, neighborCount}`**。`neighborCount` 由 graph store `bounded_neighbors(id)` 调用派生，**不**消费 `GraphProps` 正文（K7 严守）。
> 2. **W6-HS5 同步收紧 8→4 字段**：原 W6-HS5 列 NodeDetail.vue/EdgeDetail.vue 显示白名单 8 字段 + props 折略 —— **W7 修订 NodeDetail.vue 显示白名单 = `{id, kind, label, neighborCount, props_size}`**（**仅 props 字节数/字段数**，**不渲染 props 正文**）。EdgeDetail.vue 显示白名单 = `{from, to, kind, weight, props_size}`（同上，不渲染 props 正文）。**K7 双闸**（A7 W5 + A4 W6 复审确认）—— UI 不暴露敏感 props 纵深。
> 3. **§4.2 NodeDetail.vue 面板壳 / EdgeDetail.vue 面板壳 当前实现状态**：A8 W6 实际实现吃的是 `useGraphStore.ts` 静态 fixture（含 5 个示例节点 + 7 条示例边），fixture 里硬编码了 8 字段（便于视觉验收），**但生产路径 `graph_query` 命令 W6 F2 红线未落地**（grep 全仓库 0 命中），故 W6 实际**无生产路径泄露**。**W8+ 待 `graph_query` 命令由 A8 实施期落地后，由 A8 W8+ 同步把 fixture 与生产路径对齐**（8 字段 fixture 同步改为 4 字段白名单）。
> 4. **A8 W6 真实命令落地（`graph_query` 等）属 W8+ 范畴**：W7 dispatch（L155/L157）只开 A3 M5-2 read-only MCP bridge + A5 Agent/Skill read-only bridge；**A8 W7 不在 W7 派发**，W8+ A8 是否派发 `graph_query` 真实命令由 A0 W8 dispatch 决定，本卡 §[W6 next-card AC] AC-2~AC-5 维持 *not exist* 状态。
> **本卡不动**：
> - §1 GOAL · §3 WRITE · §4 关键契约 · §5 FORBID · §6 COMMANDS · §7 PASS_CRITERIA 主体 —— 仅头部状态行 + §[W6 next-card AC] AC-1 + W6-HS5 三处收紧修订。
> - §[W6 next-card AC] AC-2~AC-5 / 4 项 hard stops HS1~HS4 / A1 W6 角色 / A8 W6 消费依赖 5 段 —— 维持 W6 原状（W6 已 A0 拣入 PASS）。

---

## [W6 next-card acceptance criteria · 2026-09-06 19:25 CST] A8 M5-9 W6 实施期 acceptance criteria（graph UI pure logic + panel shell · 不加 commands / 不调 live agent consumption / 不调 model calls / 不加 graph rebuild workers）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L159（**A8 M5-W6** *"Implement M5-9 graph UI pure logic and panel shell: graph list/search/filter, node detail summary, capacity/error/empty states, helper module + headless logic test. Do not call live agent consumption or backend graph commands unless already existing and fully typed."*）+ L164-170 硬约束 + A6 W5 UI panel shell 范式（`f99d2eb` 拣入）+ A7 W5 graph model/store policy slice（`4b438ef` 拣入：`src-tauri/src/graph.rs` + `domain.rs` GraphNode/Edge + 7 容量常量 + check-graph-policy.py 7 ACTIVE 码）。
> **消费依赖（已落地，A8 W6 可直接接入）**：
> - **A7 W5 GraphNode/GraphEdge DTO 已落**（`4b438ef` 拣入 `src-tauri/src/domain.rs` 追加）—— A8 W6 UI 在 `src/types.ts` 加对应 TS 镜像（无 drift）。
> - **A7 W5 graph 容量常量已锁**（`4b438ef` 拣入：`GRAPH_PROPS_MAX_BYTES=MAX_TEXT_FIELD_BYTES=64KiB`、`GRAPH_LABEL_MAX_BYTES=256`、`GRAPH_NODE_ID_HEX_LEN=64`、`GRAPH_MAX_DEPTH=4`、`GRAPH_QUERY_LIMIT=1000`、`GRAPH_MAX_NODES=5000`、`GRAPH_MAX_EDGES=20000`）—— A8 W6 UI 容量提示/截断/分页**直接复用**这 7 个常量，**禁止在 UI 层重定义**。
> - **A7 W5 graph store/query helpers 已落**（`4b438ef` 拣入 `src-tauri/src/graph.rs`）—— A8 W6 UI 在 `src/bridge.ts` 加 `graph_query` 包装时**只读** `GraphNode` / `GraphEdge` shape，**不**消费 `GraphProps` 正文（K7）。
> - **A6 W5 面板壳范式已锁**（`f99d2eb` 拣入 `src/components/agent/SkillManager.vue` / `PermissionPreviewModal.vue`）—— A8 W6 `GraphPanel.vue` 复用 useLayoutStore + MOD_META no-router 锚点 + Agent/Skill 校验/permission preview 模式。
> - **A4 W4 agent_kv 已落**（`1610939` `agent_memory.rs`）—— A8 W6 UI **不**消费 agent_kv 业务（仅当 graph 节点含 agent 引用时按 sha256 id 显示）。
> **A1 W6 角色**：A1 W6 **不**改 §1~§11 决策史；仅在头部加本 `[W6 next-card acceptance criteria]` 段，**明确 A8 W6 实施期 4 项 AC + 5 项 hard stops**，供 A8 / A10 / A11 / A0 验收。

### W6 A8 M5-9 实施期 acceptance criteria（4 项）

| AC | 描述 | 验收证据 |
|----|------|----------|
| AC-1 **graph UI 纯逻辑 helper module 冻结** | `src/utils/graphUi.ts`（或类似 helper）冻结纯函数：① `filterGraphNodes(nodes, filter: GraphFilter) -> GraphNode[]`（按 kind / source / label 包含过滤）② `searchGraphNodes(nodes, query: string, opts) -> SearchResult[]`（label 模糊 + id 精确双轨；query 长度 0-200 字符；空 query 返回 `[]`）③ `summarizeNode(node: GraphNode) -> NodeSummary`（**不**含 `props` 正文，仅 `{id, kind, label, neighborCount}` —— **W7 修订**：原列 8 字段 `id / kind / label / source / source_ref / created_at / updated_at / extractor_version` 中后 5 字段不在 A7 DTO 且 `id=sha256` 不可逆无法派生，详见头部 [W7 patched · 2026-09-07 00:50 CST] 段 F1）④ `emptyStateFor(reason) -> EmptyStateDescriptor`（list-empty / filtered-empty / no-search-result 三态）⑤ `errorStateFor(err) -> ErrorStateDescriptor`（load-fail / parse-fail / action-fail 三态）⑥ `truncateLabel(label, maxBytes) -> string`（**复用** `GRAPH_LABEL_MAX_BYTES=256` 常量） —— 全部纯函数（无 Tauri invoke、无网络、无 fs）| `node scripts/check-graph-ui-logic.mjs` PASS + A11 抽查 |
| AC-2 **graph list/search/filter + node detail summary** | 面板（`src/components/graph/GraphPanel.vue` 或类似）实现：① list 视图（按 kind 分组 + 滚动分页，每页 ≤ `GRAPH_QUERY_LIMIT/10`）② search 输入框（`useGraphUi().searchGraphNodes` + 300ms debounce + 取消上次未完成查询）③ filter 侧栏（kind 多选 + source 下拉 + label 包含）④ node detail 抽屉（调 `summarizeNode` 输出 `{id, kind, label, neighborCount}` + `props_size` 摘要；**不显示 `props` 正文**，**不显示 `GraphProps` 任何字段** —— **W7 修订**：详见头部 [W7 patched · 2026-09-07 00:50 CST] 段 F1）⑤ source_ref 链接 → 走既有 `capability.rs` 校验的 source 跳转（**不**绕过 bridge.ts）| UI 逻辑单测 + 视觉走查（manual checklist）+ A10 抽查 K7 零泄露 |
| AC-3 **capacity / error / empty 状态三件套** | 面板必须实现：① 容量提示（`node_count >= GRAPH_MAX_NODES*0.8` 时显示"接近上限 GRAPH_MAX_NODES=5000"，**禁止**展示真实后端报错堆栈）② error 状态（`load-fail` 显示重试图标 + 1 行 hint，**不**展示 backend error 全貌）③ empty 状态（list-empty / filtered-empty / no-search-result 至少 3 种）④ 所有状态**不**暴露任何后端原始字段（如 `error.stack` / `node.props` 任意键 / DSN / token）| `node scripts/check-graph-ui-logic.mjs` 含 capacity/error/empty 覆盖 + A11 抽查 |
| AC-4 **headless logic test + 容量/隐私双扫** | `scripts/check-graph-ui-logic.mjs` 必须含：① 7+ 单测（filter / search / summarize / empty×3 / error×3 / truncate / capacity 边界）② 隐私断言（`summarizeNode` 输出**不**含 `props` 任何键名 → grep 0 命中 `props`；输出字段集合 ⊆ `{id, kind, label, neighborCount}` —— **W7 修订**）③ 容量断言（`truncateLabel(label, 256)` 截断后字节 ≤ 256）④ 容量常量真源单点（`src/utils/graphUi.ts` 内**不**写容量字面量，必须 import 自 `src/types.ts`（镜像 `domain.rs`））| `node scripts/check-graph-ui-logic.mjs` PASS + `grep -nE 'GRAPH_MAX_NODES\s*=\|GRAPH_LABEL_MAX_BYTES\s*=' src/utils/graphUi.ts` 0 命中（真源在 types.ts）|

### W6 A8 M5-9 实施期 hard stops（5 项）

| HS | 约束 | 来源 |
|----|------|------|
| W6-HS1 | **不加后端 commands**（`bridge.rs` / `main.rs` / `default-commands.toml` **不**新增 `graph_*` 命令；如确需新增须满足 source check + ACL + bridge/types + policy + tests 原子同包）| PARALLEL_COMMAND_BOARD L159 + L169（*"prefer no command in W6"*）|
| W6-HS2 | **不调 live agent consumption**（**不**调 `agent_chat` / `agent_memory_*` / `skill_*`）—— UI 仅展示，不参与 RAG 注入（`useGraphRag.ts` / RAG 注入契约在 M5-9 §4.3 暂**冻结**到 W7+）| PARALLEL_COMMAND_BOARD L159 + L167 |
| W6-HS3 | **不调 model calls**（**不**做 embedding 重建 / **不**调 LLM / **不**做 graph rebuild workers / **不**加 background job）—— UI 仅消费 A7 W5 已落地的 GraphNode/Edge DTO 静态快照（or polling，**不**订阅 push 事件）| PARALLEL_COMMAND_BOARD L159 + L167 |
| W6-HS4 | **无新 npm 依赖**（**不**引入 `d3-force` / `d3-zoom` / `d3-drag` 等 d3 子模块包到 W6；d3 可在 W7+ 真接 layout 时引入；W6 阶段可仅用 `useGraphUi().filterGraphNodes` + 简单列表 + 不做力导向布局；如确实需 placeholder 列表 + 静态边框，**不**引 d3 整包）| A8 W6 dispatch L159 + W5-HS7 |
| W6-HS5 | **K7 严守**：`summarizeNode` / node detail / list / search / filter **不**展示 `props` 任何字段（key 名 / value）—— 隐私双扫由 A7 W5 check-graph-policy.py 的 GRAPH_PRIVACY_DOUBLE_SCAN 已落，A8 W6 **不**写 UI 副本；UI 仅消费 `{id, kind, label, neighborCount}` + `props_size` 摘要白名单字段（**W7 修订**：原列 8 字段 `id / kind / label / source / source_ref / created_at / updated_at / extractor_version` 收紧为 4 字段 + `props_size`，详见头部 [W7 patched · 2026-09-07 00:50 CST] 段 F1） | A7 W5 check-graph-policy.py + A4 W4 隐私双扫 + K7 |

### W6 验证清单（供 A11 收口）

- `npm run build` PASS（`dist/assets/index-*.js` 大小不破 IF-2 阈值；W6 仍受 A0 大小门禁约束）
- `node scripts/check-graph-ui-logic.mjs` PASS
- `python3 scripts/check-graph-ui-policy.py --self-test` PASS（如新增 UI policy 脚本）
- `python3 scripts/check-graph-ui-policy.py` PASS
- `python3 scripts/check-graph-ui-policy.py --expect-pending` PASS（如有 PENDING）
- `bash scripts/pre-merge.sh` ALL_PASS
- `git diff --check` CLEAN
- A11 比对 `src/types.ts` 与 A7 W5 GraphNode/GraphEdge 0 drift
- `grep -nE 'GRAPH_MAX_NODES\s*=\|GRAPH_LABEL_MAX_BYTES\s*=' src/utils/graphUi.ts` 0 命中（容量真源在 types.ts）
- `grep -nE 'props' src/utils/graphUi.ts src/components/graph/GraphPanel.vue` 仅命中 `summarizeNode` 不导出 props 字段的注释（不命中 props 值/键）
- `grep -nE 'agent_chat|skill_run|graph_rag' src/components/graph/ src/utils/graphUi.ts` 0 命中（无 live agent consumption / 无 RAG 注入 / 无 model calls）
- **A10 复审 PASS**（K7 零泄露 / 无新命令 / 无 live agent / 无 model call / 无 rebuild workers / 无 npm 依赖 / capacity 真源单点）
- **A11 verification delta** 产出 `logs/checkpoints/M5-A11-W6-*.md`

### W6 A1 不修订范围（本卡）

- **§1 GOAL / §2 READ / §3 WRITE / §4 关键契约 / §5 FORBID / §6 COMMANDS / §7 PASS_CRITERIA / §8 FAIL_ACTION / §9 DOC_BACKWRITE / §10 COMMIT / §11 FORBID 遵守记录**：A1 W6 **不动**（决策史保持 W0 原文；W6 AC 在本顶部段单列；M5-9 §4.3 RAG 注入契约暂冻结到 W7+ 由 A0 决定）。
- **三份主文档 / ACL / Capability / pre-merge.sh / scripts/**：A1 W6 不动（policy 脚本由 A8 W6 落地）。
- **`NEXT` 标记**：A0 调度权；A1 不改字面值。

---

## 0. 编号与锚定

- 批次任务号 `M5-9`；需求号 #13；WBS L572 一致。
- 依赖：M5-8 ✅（9 条命令稳定）+ M5-6 ✅（既有 `useAgentStore` 已落地）
- 前端栈：Vue 3 + Pinia + D3.js（力导向布局）

---

## 1. GOAL

实现图谱可视化 UI（节点/边力导向布局、按 kind 着色、点击查看详情、拖拽、缩放、过滤）与 Agent RAG 注入（用户在 Agent 对话中显式"基于图谱"提问时，按 `graph_query` 取相关节点/边摘要注入 `system_prompt`）。

---

## 2. READ

1. `src/components/browser/AINavPanel.vue`（**全读**——`useAgentStore` 桥接）
2. `M5-6-agent-skill-ui.md`（既有 Agent UI 范式）
3. `M5-8-graph-store-query.md` §4.2（`GraphQuery` DTO + 上限）
4. `src/bridge.ts`（`graph_*` TS 包装，**全读**）
5. `src/types.ts`（`GraphNode` / `GraphEdge` TS 镜像）
6. `src/router/agent.ts`（路由）

---

## 3. WRITE

| 文件 | 性质 | 说明 |
|---|---|---|
| `src/components/graph/GraphViewer.vue` | **新增** | D3.js 力导向布局 + 拖拽/缩放/过滤 |
| `src/components/graph/NodeDetail.vue` | **新增** | 节点详情（不显示 props 正文） |
| `src/components/graph/EdgeDetail.vue` | **新增** | 边详情 |
| `src/components/graph/GraphFilter.vue` | **新增** | 按 kind / source 过滤 |
| `src/composables/useGraphQuery.ts` | **新增** | 包装 `graph_query` + 缓存 + 取消 |
| `src/composables/useGraphRag.ts` | **新增** | RAG 注入（显式触发） |
| `src/stores/useGraphStore.ts` | **新增** | 图谱状态（选中节点、视图参数、缓存） |
| `src/router/graph.ts` | **新增** | 图谱路由 |
| `src/locales/zh-CN.json` `src/locales/en-US.json` | 扩展 | 8 个 i18n key |
| `src/styles/graph.scss` | **新增** | 样式（dark/light 双主题） |

---

## 4. 关键契约

### 4.1 `GraphViewer.vue` 交互

- **节点**：圆 + 标签 + 颜色按 kind；大小按 degree（边数）
- **边**：线 + 箭头 + 粗细按 weight
- **交互**：拖拽（d3-drag）+ 缩放（d3-zoom）+ 点击（高亮邻居 + 打开详情）
- **过滤**：`GraphFilter` 多选 kind + source
- **布局**：d3-force（forceLink + forceManyBody + forceCenter）
- **首期上限**：可见节点 ≤ 500（多则提示"过滤缩小"）

### 4.2 `NodeDetail.vue` / `EdgeDetail.vue`

- 显示：`{id, kind, label, neighborCount, props_size}` 摘要（**W7 修订**：原列 8 字段 `id / kind / label / source / source_ref / created_at / updated_at / extractor_version` 收紧为 4 字段 + `props_size`；fixture 仍含 8 字段硬编码由 W8+ A8 在 `graph_query` 真实命令落地时同步对齐，详见头部 [W7 patched · 2026-09-07 00:50 CST] 段 F1）
- **不**显示 `props` 正文（K7）
- `source_ref` 路径点击 → 打开对应文件/tab（受 `capability.rs` 校验）

### 4.3 RAG 注入契约（`useGraphRag.ts`）

- **仅显式触发**：用户在 ChatPanel 输入框加 `/graph <query>` 前缀
- 流程：
  1. 解析 `query` → 调 `useGraphQuery.search`（先按 label 模糊匹配 FTS5 起点）
  2. 起点 + `max_depth=2` 调 `graph_query`
  3. 把结果序列化为"图谱上下文"（节点 label + kind + 关系描述，**不**含 props）
  4. 注入到 `agent_chat` 的 `system_prompt` 临时扩段（首期仅限本轮）
  5. UI 标注"已基于图谱"标记
- **禁**隐式注入（避免 token 浪费 + 上下文污染）

### 4.4 性能与可访问性

- 大图（> 500 节点）自动隐藏弱连接
- 键盘快捷键：`Esc` 关闭详情，`f` 打开过滤
- ARIA 标签：节点/边可读屏

---

## 5. FORBID

- **不**让 `NodeDetail` / `EdgeDetail` 显示 `props` 正文（K7）
- **不**让 RAG 注入隐式触发（仅 `/graph` 前缀）
- **不**让 UI 路径绕过 `bridge.ts`（M5-6 范式）
- **不**让图谱渲染不节流（拖拽时 ≥ 30fps 目标）
- **不**破 K7（props 不出 UI）
- **不**移动 `NEXT`、不 push

---

## 6. COMMANDS

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# A. 文件结构
ls src/components/graph src/composables/useGraph* 2>&1

# B. 反向用例
# N1: NodeDetail 显示 props → 阻断
# N2: RAG 隐式注入（无 /graph 前缀）→ 阻断
# N3: 拖拽不节流 → 阻断
# N4: dark/light 主题崩 → 阻断
# N5: i18n 缺 key → 阻断
# N6: 屏读器无法读 → 阻断

# C. e2e 冒烟
pnpm test:unit
pnpm test:e2e

# D. 编译与基线
pnpm build
pnpm tsc --noEmit
pnpm lint
```

---

## 7. PASS_CRITERIA

| # | 判据 | 验证 |
|---|---|---|
| 1 | 5 个新组件 + 2 个 composable + 1 store 落地 | 命令 A |
| 2 | `props` 正文不显示 | 单测 N1 |
| 3 | RAG 仅 `/graph` 前缀触发 | 单测 N2 |
| 4 | 拖拽节流 ≥ 30fps | 单测 N3 |
| 5 | dark/light 主题适配 | 单测 N4 |
| 6 | i18n 双语齐 | 单测 N5 |
| 7 | ARIA 标签 | 单测 N6 |
| 8 | `pnpm test:unit` + `pnpm test:e2e` 全绿 | 命令 C |
| 9 | `pnpm build` 无错 | 命令 D |

---

## 8. FAIL_ACTION

| 失败 | 动作 |
|---|---|
| `props` 出现在 UI | 阻断（K7） |
| 隐式 RAG 注入 | 阻断（token 浪费 + 污染） |
| 裸 `invoke` | 阻断 |
| dark/light 主题崩 | 阻断 |
| i18n 缺 key | 阻断 |

---

## 9. DOC_BACKWRITE

1. `详细设计与实施计划.md` L572 `[ ]` → `[x]`
2. `后续需求TODO.md` §13 状态 `DONE`
3. `AI-模型切换与接手清单.md` NEXT 移至 `M5-10`
4. `logs/checkpoints/M5-9.a-2026MMDD-HHMM.md`

---

## 10. COMMIT / NEXT

- **COMMIT**：A0 拣入后由 A19 实施填
- **NEXT**：M5-10（插件 manifest 与生命周期）

---

## 11. FORBID 遵守记录

- 本卡为 A1 M5-W0 文档展开，**未写任何产品代码**
- 未触 `src/`、`package.json`、三份主文档、ACL/Capability
- 未移动 `NEXT`（仍 M5-W0）
- 未提交、未 push
