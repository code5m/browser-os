# A5 · Vue 专属最小改动复刻蓝图（M5-W18-R2 证据闭环版）

> 配套 `A5-workbench-ux-map.md`（证据层）。本文件给出**只改现有文件、不新增组件/依赖、净字节 ≤ 345 B** 的最小改动计划，目标是让 W19 编码 lane 无需重新发现架构即可落地。
> 分类口径：`COPY`（带署名小单元，体积允许时）｜`ADAPT`（改既有文件/行为）｜`REIMPLEMENT_FROM_BEHAVIOR`（按 dbx 行为自写，因 DTO 未定）｜`DEFER`（超 M5/等依赖）｜`REJECT`（框架/桌面专属/体积不可行）。
> 约束：`measure-build-metrics.py:39` 上限 25.2%（`AC-5` 禁抬）；`pre-merge.sh` 门禁（含 `check-database-ui-logic.mjs` 119 断言）；`cargo_warnings` 只减不增；R2 禁止改 `src/`、禁止引入依赖。

---

## 0. 设计前提（由证据层推导，非推断）

1. 当前产品数据库前端**已成型**：`DatabasePanel.vue` + `useDatabaseStore.ts` + `dbUi.ts` + `bridge.dbConnect/dbQuery/dbDisconnect` 已 live wiring（M4-4）。**蓝图不是从零设计，而是对既有实现的增量打磨**。
2. 体积余量 ≈ 345 B（真基线 `4f0e8ab` 612,943 → 上限 767,404；干净 `052b18a` 767,059）。**任何改动必须零净增或替代既有代码**。
3. 测试范式为既有 `.mjs` 源码逻辑门禁（无 Vitest）；新增断言进 `check-database-ui-logic.mjs`。
4. 无 `useConnectionStore`、无 `src/lib/` → 所有"新建 store/目录"设想作废。

---

## 1. 总览分类表（修正后）

| # | 能力 | 分类 | 目标落点（既有文件） |
|---|---|---|---|
| 1 | 连接树/列表 | ADAPT | `useDatabaseStore.ts`(connections) + `DatabasePanel.vue`(`conn-list`) |
| 2 | Schema 浏览器 | REJECT(M5) / DEFER(W19) | 无当前落点 |
| 3 | 多标签编辑器 | REJECT(M5) | 保持 `db.sql` 单 `<textarea>` |
| 4 | 执行工具栏 | ADAPT | `DatabasePanel.vue` 按钮组 |
| 5 | 取消/进度 | REIMPLEMENT_FROM_BEHAVIOR(W19) | `useDatabaseStore.ts` + A4 取消 DTO |
| 6 | 结果网格 | ADAPT | `DatabasePanel.vue` `<table>` + `dbUi.ts` 导出 |
| 7 | 历史 | DEFER(W19) | 等 A4 历史命令 |
| 8 | 键盘/a11y | ADAPT | `DatabasePanel.vue` + 复用 `useModalFocus` |
| 9 | 加载/空/错误 | ADAPT(内联) / COPY(仅在余量内) | `DatabasePanel.vue` 内联 `role=status` |
| 10 | 响应式 | ADAPT | 侧栏内滚动（无需溢出折叠） |

---

## 2. 各能力最小改动规格（W19 实现卡可直接套用）

### ① 连接树/列表 — ADAPT
- 现状：`useDatabaseStore.connections: DbConnectionSummary[]` + `DatabasePanel.vue:77-84` `conn-list`。`refreshConnections()` 为空实现（A4 未给列表命令）。
- 改动：保持空实现；W19 接 `db_list_connections` 后填充 `selectConnection`，**不新增 store**。
- 状态转移：`idle → (select) → form 填充 → (connect) → connected`。
- 断言新增（`check-database-ui-logic.mjs`）：`connections` 列表项 `label` 永不出现 `@`/`password`（复用 `connectionLabel` 既有测试）。

### ② Schema 浏览器 — REJECT(M5)/DEFER(W19)
- 当前即连即查模型无 schema 树；`ObjectBrowser.vue`(180KB) 体量超限且依赖 30+ 数据库类型。**M5 不做**。W19 由 A4 提供 schema 命令后再议，且须自写虚拟滚动（本仓库无 `RecycleScroller` 依赖）。

### ③ 编辑器内核 — REJECT(M5)
- `QueryEditor.vue`(291KB) 与 CodeMirror 6 在 0 余量下**禁止引入**（C9，见证据层 §5）。
- 保持原生 `<textarea v-model="db.sql">`（`DatabasePanel.vue:137`）。语句级 gutter 运行按钮、vim/dialect 等**全部 DEFER**，待 A0 在 disposable 工作区授权 CodeMirror footprint 测量。

### ④ 执行工具栏 — ADAPT（小）
- 复用 dbx Run/Stop 互斥语义（`[REFERENCE_SOURCE]` EditorToolbar.vue）：当前 `运行` 按钮 `:disabled="db.busy||!db.runGate.ok"`，已是运行/取消二态的等价（无独立 cancel 令牌，故暂为 busy 态禁用）。
- 改动：
  - 写类按钮（复制/清空以外的 destructive）加 `:aria-pressed`；所有动作加 `:aria-label` 与 `:title`（对齐 `check-ui-a11y-logic.mjs` 的 `ariaRoleForVariant` 思路，但网格/工具栏断言须在 `check-database-ui-logic.mjs` 补）。
  - `运行` 按钮在 `db.busy` 时图标切换为 spinner（纯 CSS，0 净增）。

### ⑤ 取消/进度 — REIMPLEMENT_FROM_BEHAVIOR（W19，DTO 依赖）
- 行为契约（自写，语义对齐 dbx `queryStore.ts:105-106`）：`isExecuting`/`isCancelling`/`executionId`/`cancelRequestCount`；取消超时 `10_000ms` + ack 结算 `2_000ms`；进度状态机（pending/running/success/error）+ `affectedRows`/`executionTimeMs`；`isExecuting` 时拒绝关闭结果。
- 依赖 A4：`db_query` 返回须带 `query_id`/`state`，并新增 `db_cancel(conn_id, query_id)`（A4 未冻结）。**R2 仅冻结规格，不落地**。

### ⑥ 结果网格 — ADAPT（小）
- 现状：`DatabasePanel.vue:158-184` `<table>` + `displayRows`（首 200 行，`DISPLAY_ROW_CAP`）+ `hiddenRows` 提示 + `copyCsv`（CSV 经 `dbUi.toCsv`）。
- 改动：把"复制 CSV / 清空结果 /（W19）导出"建模为能力对象 `{visible,disabled,label,tooltip}`（数据驱动，便于单测），渲染仍走既有按钮组。**不引入 `DataGridToolbar.vue` 661KB 内核**。
- 分页：后端 `maxRows=1000` 已截断，前端无需分页器；如超 1000 行再议（当前 `hiddenRows` 提示已覆盖）。

### ⑦ 历史 — DEFER(W19)
- 当前无历史持久化。W19 由 A4 历史命令 + `nextCursor` 游标提供；竞态守卫 `requestSerial` 模式（`[REFERENCE_SOURCE]` historyStore）作为实现要点记录，R2 不落地。

### ⑧ 键盘/可访问性 — ADAPT
- `DatabasePanel.vue` 现状 `aria/role/keydown` 计数 **0**（证据层 §2）。最小改动：
  - `<textarea>` 加 `@keydown.ctrl.enter` / `@keydown.meta.enter` → `db.requestRun()`；`@keydown.esc` 关闭确认框。
  - 写确认弹窗改为复用 `src/components/shared/ConfirmModal.vue` + `useModalFocus`（既有 `GitWriteConfirmDialog.vue` 范式），替换当前内联 `<div class="confirm">`，以满足 `check-ui-a11y-logic.mjs` 的 modal 焦点契约（variant=`danger/confirm`）。
  - 结果表 `<table>` 加 `aria-label`；`th` 加 `scope="col"`。
- 新增断言（`check-database-ui-logic.mjs`，源码级）：确认弹窗使用 `ConfirmModal`/`useModalFocus`；textarea 支持 Ctrl/Cmd+Enter 触发运行（通过 dbUi 纯函数或组件源码契约）。

### ⑨ 加载/空/错误态 — ADAPT(内联优先)
- 现状：`db.busy`/`db.error`/`result` 已驱动显示（`.gate`/`.errors`/`.empty`）。
- 不新建 `QueryLoadingState.vue`/`ErrorBanner.vue`（Copyright 4.2KB 净增超余量）。改为**内联等价 `role=status`/`role=alert`** 于现有 `.errors`/`.gate`，0 净增。
- 仅在 A0 抬限有预算时，再 COPY dbx 两组件并登记 Apache-2.0 署名（归 A10 账本）。

### ⑩ 响应式 — ADAPT
- 面板位于侧栏（固定宽度容器，内部 `overflow:auto`），无需溢出折叠 `settleRightOverflow`。REJECT `set_macos_traffic_light_position`（macOS 专属）。

---

## 3. 测试策略（对齐真实门禁，非 Vitest）

1. **延续 `.mjs` 源码逻辑门禁**：所有新增 UI 行为断言进 `scripts/check-database-ui-logic.mjs`（已 119 断言通过）。新增内容：① 确认弹窗复用 `ConfirmModal`/`useModalFocus` 的源码契约；② 写按钮 `aria-*` 标记；③ textarea Ctrl/Cmd+Enter 运行触发（经纯函数或组件源码契约）。
2. **不引入 Vitest**：产品 `package.json` 无 vitest/test 脚本，引入即违反体积/依赖约束（C4）。dbx 的 Vitest/`?raw` 范式**仅作设计参考**，不在本仓库复刻。
3. **组件级 DOM 渲染测试**：保持 0（与 dbx `@vue/test-utils` 计数 0 一致），用源码契约 + 逻辑单测替代。

---

## 4. 体积与门禁前置条件（硬）

- 任何 UI 改动须满足：`dist.total_bytes` 增长 ≤ 345 B（或**零净增/替代既有行内代码**）。`check-database-ui-logic.mjs` 不进 bundle（脚本不计 dist）。
- 接通新增 `db_*` 命令（列表/取消/历史）须等 A4 DTO 冻结 + `check-database-policy.py` 通过；此前面板保持"无命令调用的既有骨架"。
- `cargo_warnings` 只减不增（后端改动归 A4/A6，本 lane 不涉及）。
- CodeMirror/新依赖：**REJECT(M5)**，DEFER 至 A0 在 disposable 工作区授权 footprint 测量（参照 A8 的 `/tmp/m5-w18-a8-zvec` 做法）。

---

## 5. 直接可 COPY 的最小单元（仅在体积预算内、带 Apache-2.0 署名，归 A10 账本）

- `DataGridToolbarActionCapability` 的**数据形态**（`{visible,disabled,loading,label,tooltip}`，定义于 `DataGridToolbar.vue:18`）——只抄类型与渲染思路，不抄 661KB 组件。
- `historyStore` 的 `requestSerial` 竞态守卫**模式**（行为级，W19 自写参考）。
- 上述均为"行为/类型"级，落地时若产生净字节且超预算，则降级为内联自写（不 COPY 文件）。

---

## 6. 与 R1 的偏差声明

本蓝图相对 R1 蓝图（`A5-replication-blueprint.md` 旧版）的**实质变更**：
- 删除全部 `useConnectionStore` / `src/lib/*` / `src/composables/useDataGrid*` 新建设想（不存在目标）。
- 删除"新增 `check-database-ui-logic.mjs`""采纳 Vitest""懒加载拆编辑器内核"三项过时/错误主张。
- 把"复刻 dbx 组件"全面降级为"打磨既有 `DatabasePanel`/`useDatabaseStore`/`dbUi` 的最小 ADAPT"，并写入 345 B 体积红线。
- 取消/历史的 `REIMPLEMENT_FROM_BEHAVIOR` 仅冻结行为契约，落地归 W19（DTO 依赖）。
