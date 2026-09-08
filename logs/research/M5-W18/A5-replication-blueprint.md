# A5 · Vue 专属复刻蓝图（Replication Blueprint）

> 配套 `A5-workbench-ux-map.md`。目标：**复刻 dbx 工作台的行为与架构，而非整段拷贝其框架耦合的 UI**。
> 分类口径：`COPY`（小工具/纯函数直接移植带署名）｜`ADAPT`（架构/组件适配本仓库约定）｜`REIMPLEMENT_FROM_BEHAVIOR`（按行为自写，因 DTO 未定/体积受限）｜`DEFER`（超 M5 范围，等依赖就绪）｜`REJECT`（框架/桌面专属或不适用）。
> 约束（来自本仓库门禁）：产物体积 ≤ 25.2%（`AC-5` 禁抬上限）；`pre-merge.sh` 含 `check-database-policy.py` / `check-home-client-policy.py` / `check-client-navigation-logic.mjs` / `check-ui-a11y-logic.mjs`；A4 DTO 未冻结前不得接通 `db_*` bridge。

## 总览分类表

| # | 能力 | 分类 | 本仓库目标位置（建议） |
|---|---|---|---|
| 1 | 连接树 | ADAPT | `src/components/connection/ConnectionTree.vue` + `src/stores/useConnectionStore.ts` |
| 2 | Schema 浏览器 | ADAPT(+DEFER ER图) | `src/components/objects/ObjectBrowser.vue` + `useSchemaOptions`/`useDatabaseOptions` |
| 3 | 编辑器标签页 | ADAPT(+REJECT 290KB 内核) | `src/components/layout/AppTabBar.vue` + `src/stores/useQueryStore.ts`(tabs) |
| 4 | 执行工具栏 | ADAPT | `src/components/layout/EditorToolbar.vue` |
| 5 | 取消/进度 | REIMPLEMENT_FROM_BEHAVIOR | `src/stores/useQueryStore.ts` + `src/lib/sql/queryExecutionState.ts` |
| 6 | 结果网格 | ADAPT(能力模式) | `src/components/grid/DataGrid*.vue` + `src/composables/useDataGrid*.ts` |
| 7 | 历史 | REIMPLEMENT_FROM_BEHAVIOR | `src/stores/useHistoryStore.ts` + `src/components/editor/QueryHistory.vue` |
| 8 | 键盘/可访问性 | ADAPT | `src/lib/editor/shortcutRegistry.ts` + 既有 a11y 门禁 |
| 9 | 加载/空/错误态 | COPY/ADAPT | `src/components/common/QueryLoadingState.vue` + `ErrorBanner.vue` |
| 10 | 响应式布局 | ADAPT(+REJECT 红绿灯同步) | `AppToolbar.vue` 溢出折叠逻辑 |

---

## 1. 连接树 — ADAPT
- **复用架构**：递归 `TreeItem` + `useConnectionStore` 的 `TreeNode`（懒加载 `children`、展开态诊断）。
- **适配点**：本仓库 `useConnectionStore`（M4-2 已建）承接 `connections`/`sidebarLayout`；`canUseLoadedTreeNodeToggle` 的“已加载直接展开 / 未加载异步取数”判定直接迁移。
- **拒绝整段拷贝**：132KB `ConnectionTree.vue` 含大量 dbx 专属树类型（Mongo/Redis/MQ/Nacos/Consul…），只取通用 SQL 连接树分支。
- **测试**：`scripts/check-database-ui-logic.mjs` 加“树节点展开态不越权”断言；store 单测仿 `connectionStore.*.spec.ts`。

## 2. Schema 浏览器 — ADAPT（ER 图 DEFER）
- **复用**：`ObjectBrowser` 的 `RecycleScroller` 虚拟滚动 + `useSchemaOptions`/`useDatabaseOptions` 三级联动 + `executeWithProductionSqlGuard` 护栏。
- **适配**：本仓库 `useConnectionStore` 提供连接上下文；生产护栏对接 A4 `security_policy.rs` 判定（fail-closed）。
- **DEFER**：`SchemaDiagramDialog`/`LayerPanel`（ER 图）超出 M5，待 W19。
- **测试**：采用 dbx `?raw` 源码契约测试守护“粘贴前 schema 归一化”等关键不变量（`ObjectBrowserClipboard.spec.ts` 范式）。

## 3. 编辑器标签页 — ADAPT（内核 REJECT）
- **复用**：`useQueryStore.tabs: QueryTab[]` + `AppTabBar`（pinned/普通分栏、`useTabDrag`→`reorderTab`、脏关闭确认、`TabExecutionStatus`）。
- **REJECT 290KB `QueryEditor.vue`**：本仓库建“薄 CodeMirror 6 封装”——动态 `import()` 分包按需加载、语句 gutter 运行按钮（`runStatementGutterExtension` 行为）、vim/dialect 作为可选 compartment。体积须受 25.2% 上限约束（单一编辑器组件预计占 0.5–1pp，需提前预算评估）。
- **测试**：标签页逻辑（reorder/close-confirm）走 composable/store 单测。

## 4. 执行工具栏 — ADAPT
- **必复刻模式**：Run/Stop 互斥 = `activeTab.isExecuting ? emit('cancel') : emit('execute')`，图标 `Play`↔`Square`(`fill-current`)，`isCancelling` 显示 `Loader2 spin`；按钮禁用公式 `isCancelling || isExplaining || (!isExecuting && !executableSql.trim())`。
- **适配 a11y**：每个动作按钮带 `:aria-label`/`:title`；toggle 类带 `:aria-pressed`；对齐 `check-ui-a11y-logic.mjs`。
- **事务组 / Multi-execute**：M5 先做单连接执行；`MultiDbExecuteDialog` 与事务组 DEFER 至 A4 DTO 就绪。

## 5. 取消/进度 — REIMPLEMENT_FROM_BEHAVIOR
- **行为契约（自写实现）**：
  - tab 级真相源 `isExecuting`/`isCancelling`/`executionId`/`cancelRequestCount`；
  - `cancelTabExecution` + `cancelMultiDbExecutionScope`（并发取消 worker）；
  - 超时：`CANCEL_QUERY_TIMEOUT_MS=10000`、`CANCEL_ACK_SETTLE_TIMEOUT_MS=2000`；
  - 进度：`applyBatchSqlProgress` 的语句级 `status` 状态机 + `affectedRows`/`executionTimeMs`；
  - 流式：`appendQueryResultSegment` 按 `maxRows` 截断追加 + 列变更/重复检测；
  - 生命周期：`resultRuns`/`resultEvicted`/`touchResult`，`isExecuting` 时拒绝关闭。
- **原因**：A4 的取消/执行 DTO 仍在形成，且 dbx 该 store 为 323KB，必须按行为自写而非拷贝。
- **测试**：`useQueryStore` 取消/进度走单测（仿 `useSqlExecution.spec.ts` 55KB 的行为覆盖）。

## 6. 结果网格 — ADAPT（能力模式）
- **最值得借鉴**：`DataGridToolbar` 的 **能力对象模式**——`DataGridToolbarActionCapability{visible,disabled,loading,label,tooltip}` 由 `lib/dataGrid/dataGridToolbar` 的 `select/trigger/toggle` 驱动，工具栏只渲染纯数据。本仓库 `DataGridToolbar.vue` 直接采用此解耦。
- **适配分页/筛选/复制/导出**：`useDataGridPagination`/`useDataGridFilterBuilder`/`useDataGridExport` 等 composable 隔离纯逻辑，便于单测；导出进度走 `useExportTracker` 任务生命周期（仿 `useDataGridExport.sqlProgress.spec.ts`）。
- **拒绝**：dbx 网格 200+ 单元格编辑器（Enum/Temporal/Json/Binary…）按数据库类型渐进实现，不一次性移植。
- **测试**：`scripts/check-database-ui-logic.mjs` 覆盖“复制/导出格式约定”；`useDataGrid*` 走 Vitest 单测。

## 7. 历史 — REIMPLEMENT_FROM_BEHAVIOR
- **行为契约**：`entries/loading/loadingMore/total/nextCursor/error` + `search(request,append)` 游标分页 + `loadMore`；**竞态守卫** `requestSerial`/`mutationGeneration`/`destructiveMutations` 防止过期响应覆盖；`setHistoryPanelActive` 切出即作废在途请求。
- **适配**：写 SQL 经 A4 `productionExecutionGuard` 等价闸门；`nextCursor` 对接后端游标（A4 DTO）。
- **测试**：`useHistoryStore` 单测覆盖“快速切换过滤时旧响应不落地”。

## 8. 键盘/可访问性 — ADAPT
- **复用**：`shortcutRegistry` 归一化（`normalizeShortcutSettings`/`shortcutToCodeMirrorKey`）+ 全局/编辑器内快捷键双层；focus ring、`aria-*`、`.sr-only`、`CustomContextMenu` 基元。
- **适配**：对接本仓库既有 `check-ui-a11y-logic.mjs` 与 `useModalFocus` 模式；快捷键注册须不冲突于现有 `check-client-navigation-logic.mjs`。

## 9. 加载/空/错误态 — COPY/ADAPT
- **COPY 小型共享组件**：`QueryLoadingState.vue`、`ErrorBanner.vue` 结构可直接移植（带署名），store 暴露 `loading`/`error` 一等字段。
- **适配**：`translateBackendError` 对接本仓库 `i18n/backend-errors` 既有体系。

## 10. 响应式布局 — ADAPT（+REJECT 红绿灯同步）
- **复用**：`AppToolbar` 的 `toolbarCollapsed` + `rightOverflowCount` + `settleRightOverflow()`（ResizeObserver + scrollWidth 测量，溢出项收进 “More”）。这是比纯 CSS 更稳健的窄窗策略，本仓库工具栏直接采用。
- **REJECT**：`invoke("set_macos_traffic_light_position")` 为 macOS 桌面专属红绿灯对齐，本仓库若无此需求则不做。

---

## 测试策略（统一采纳 dbx 双范式）
1. **逻辑单测（Vitest）**：所有 `use*` composable 与 `use*Store` 走 `vi.mock(api/store/i18n/tauriRuntime)` 隔离测试，以 `createOptions()` 工厂注入 `computed`/`ref`。新增 `scripts/check-database-ui-logic.mjs` 作为本仓库的“源码逻辑门禁”对应物。
2. **源码契约测试（`?raw`）**：关键不变量（粘贴归一化、生产护栏调用点、取消超时常量）用 `import X from "./X.vue?raw"` + 正则 `.toMatch()` 守护，防结构漂移。
3. **不做**：DOM 渲染式组件测试（`@vue/test-utils` 在 dbx 计数为 0，本仓库亦不引入，避免脆弱与体积负担）。

## 体积与门禁前置条件
- 任何组件落地前须评估对 25.2% 上限的贡献；编辑器内核（CodeMirror 6）必须动态分包，禁止 290KB 单体。
- 接通 `db_*` bridge 调用须等 A4 DTO 冻结 + `check-database-policy.py` 通过；在此之前组件以“无命令调用的骨架”存在（与 board L120 LIMITED START 一致）。
- 安全/取消/生命周期与 A4、A6 强耦合，W19 实现前本蓝图仅作规格冻结。

## 直接可 COPY 的最小单元（带 Apache-2.0 署名）
- `lib/dataGrid/dataGridPagination.ts` 的 `resolveDataGridPaginationTotal` / `canGoNextDataGridPage` 纯函数（分页论断）。
- `historyStore` 的竞态守卫 `requestSerial`/`mutationGeneration` 模式（行为级，建议自写但参考）。
- `ErrorBanner.vue` / `QueryLoadingState.vue` 结构。
- 上述均须保留版权头并在第三方声明登记。
