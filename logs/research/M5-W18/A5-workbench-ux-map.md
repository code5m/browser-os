# A5 · dbx 桌面工作台（Workbench）UX 组件/Store/测试全景映射

> 研究目标（M5-W18-R Dispatch，board L90 / L1399）：映射 **dbx 桌面工作台 UX** 的精确组件 / store / 测试，覆盖连接树、Schema 浏览器、编辑器标签页、执行工具栏、取消/进度、结果网格、分页/筛选/复制/导出、历史、键盘/可访问性、加载/空/错误态、响应式布局，并据此产出 Vue 专属复刻蓝图。
>
> 参考源（Apache-2.0，Cargo.lock 锁定）：`/home/ainfinit/Documents/极智简单/V3/research/dbx-src`（dbx 本身是 **Vue 3 + Tauri 2** 桌面数据库工作台，`apps/desktop` 含 421 个 `.vue`、74 个 Pinia store、110 个 composable）。既有分析：`/home/ainfinit/Documents/极智简单/V3/dbx-study/`。
> 本报告只做映射与证据登记，**不含任何产品代码**。

## 0. 方法论与关键结论

dbx 前端采用 **“store 驱动 + composable 纯逻辑 + 薄 SFC 视图”** 三层架构，对当前产品（同构 Vue3+Tauri2）高度可迁移：

- **Store（Pinia）** 持有全部会话态与领域态，组件只通过 `useXxxStore()` 消费，极少直接 `invoke`。
- **`api` 调用层**（`@/lib/backend/api`）是 Tauri `invoke` 的唯一封装面；组件/store 经由它访问 Rust 后端（`api.executeQuery`、`api.searchHistory`、`api.saveHistory` …）。→ 对应本仓库的 `src/bridge.ts` + `src/types.ts`（snake_case DTO）。
- **composable 承载纯逻辑**（分页、筛选、导出、选择、列布局…），可被 Vitest 单测隔离验证。
- **可访问性** 是默认内建：focus ring、`aria-label`/`aria-pressed`/`aria-expanded`、`.sr-only`、`title`、自定义右键菜单。
- **加载/空/错误** 是 store 一等公民（`loading`/`error`/`total`/`nextCursor`），组件用 `QueryLoadingState` / `ErrorBanner` 呈现。
- **响应式** 由组件内 ResizeObserver + scrollWidth 测量驱动（非纯 CSS），toolbar 可折叠进 “More”。

> 结论：**架构可直接借鉴（ADAPT）**；但 dbx 体量远超当前产品（30+ 数据库类型、ER 图、Mongo/Redis/MQ 专属浏览器、AI 助手等），且本仓库有 25.2% 产物体积硬上限与 `pre-merge.sh` 多门禁约束 —— 因此必须 **“复刻行为/架构、适配组件到本仓库约定、拒绝整段拷贝”**（详见 `A5-replication-blueprint.md`）。

---

## 1. 连接树（Connection Tree）

| 维度 | dbx 实现 |
|---|---|
| 主组件 | `apps/desktop/src/components/sidebar/ConnectionTree.vue`（132KB，极大）、`TreeItem.vue`、`SidebarTreeRuntimeHost.vue`、`SidebarTreeItemDialogs.vue` |
| Store | `src/stores/connectionStore.ts`（`useConnectionStore`：connections、sidebarLayout、lazy `TreeNode`、completion metadata） |
| 支撑 lib | `src/lib/sidebar/sidebarLayoutMonitor.ts`（展开/折叠/滚动壳诊断）、`src/lib/table/objectBrowserRowsCache` |
| 关键行为 | 懒加载子节点（展开才取 `children`）；`store.canUseLoadedTreeNodeToggle(node)` 区分“已加载节点直接展开”与“未加载需异步取数”；搜索时自动展开命中分支（`searchAutoExpandedNodeIds`）；连接/ schema 展开态上报给 layout monitor 以诊断“半高视口卡死”类 bug |
| 右键/对话框 | `SidebarTreeItemDialogs.vue`（新建/编辑/删除连接、DDL、导入导出） |
| a11y | `TreeItem` 复用展开/折叠；`title` 提示；上下文菜单 `CustomContextMenu.vue` |
| 测试 | `stores/__tests__/connectionStore.*.spec.ts`（completion 70KB、databaseInfo、defaultSchema、disconnectDataTabMetadata、dorisCatalog …）|

> 复刻要点：连接树是“受控懒加载树 + 搜索自动展开 + 展开态诊断”的组合。`TreeItem` 递归组件 + `connectionStore` 的 `TreeNode` 模型可直接适配本仓库的 `useConnectionStore`（M4-2 已埋点）。

---

## 2. Schema 浏览器（Schema / Object Browser）

| 维度 | dbx 实现 |
|---|---|
| 主组件 | `src/components/objects/ObjectBrowser.vue`（对象/表浏览器，虚拟滚动）、`src/components/structure/TableStructureEditor.vue` |
| ER 图 | `src/components/diagram/SchemaDiagramDialog.vue`、`LayerPanel.vue`、`DiagramToolbar.vue`、`DiagramInspector.vue` |
| 选择 composable | `useSchemaOptions`、`useDatabaseOptions`（catalog/database/schema 三级联动） |
| 支撑 lib | `src/lib/table/objectBrowserRows`（行构建/排序/过滤/统计/缓存）、`tableClipboard`（表数据复制粘贴）、`objectSourceEditor`、`dbAdminSql`（drop/truncate/vacuum/duplicate SQL 生成）、`objectBrowserRowAction`（单击/双击/延迟判定） |
| 关键行为 | `RecycleScroller` 虚拟滚动；表/视图/函数等按 FK 依赖排序（`sortTablesByFkDependency`）；DDL 查看/导出；表数据复制→粘贴到另一 schema（带 schema 归一化与部分失败回滚）；DDL 写操作经 `executeWithProductionSqlGuard` 生产库保护 |
| a11y | 行菜单懒绑定 `getObjectBrowserMenuItems(item)`；`title`/`role`；复制粘贴上下文菜单 |
| 测试 | `components/objects/ObjectBrowserClipboard.spec.ts`（**`?raw` 源码契约测试**：正则断言“粘贴前归一化 schema”“仅完全成功才消费剪贴板”等代码形态）、`components/structure/TableStructureEditor.{primaryKey,charsetCollation}.spec.ts` |

> 复刻要点：Schema 浏览器 = 虚拟滚动对象列表 + 三级选择联动 + 生产护栏 + 复制粘贴。ER 图（SchemaDiagramDialog）属 DEFER（超 M5 范围）。`?raw` 源码契约测试是 dbx 特有、值得借鉴的“防结构漂移”手段。

---

## 3. 编辑器标签页（Editor Tabs）

| 维度 | dbx 实现 |
|---|---|
| 标签条 | `src/components/layout/AppTabBar.vue`、`TabExecutionStatus.vue` |
| 拖拽/滚动 | `useTabDrag`（→ `queryStore.reorderTab`）、`useTabScroll` |
| 模型 | `useQueryStore.tabs: QueryTab[]`，每项含 `mode`(query/data)、`title`、`pinned`、`isExecuting`/`isCancelling`、`dirty`（`closeConfirmDirtyTabIds`）、`resultRuns` |
| 呈现 lib | `src/lib/tabs/tabPresentation.ts`（`tabDisplayTitle`、`tabTooltipLines`、`connectionColor`） |
| 关键行为 | 固定(pinned)/普通标签分栏；拖拽重排；双击重命名（`renameTab`，选区到扩展名前）；关闭脏标签确认弹窗（单个/批量）；classic vs wrap 布局切换；`TabExecutionStatus` 显示每个标签的执行态 |
| 编辑器内核 | `src/components/editor/QueryEditor.vue`（290KB，**CodeMirror 6**：动态 import 分包、vim 模式、SQL dialect、语义高亮、自动补全、语句 gutter 运行按钮、搜索 keymap） |

> 复刻要点：`useQueryStore.tabs` + `AppTabBar` + `useTabDrag` 的组合是直接可适配的。但 290KB 的 `QueryEditor.vue` **不可整段拷贝**——本仓库必须做一个“薄 CodeMirror 6 封装”（动态分包按需加载、语句级运行 gutter），以符合 25.2% 体积上限。

---

## 4. 执行工具栏（Execution Toolbar）

| 维度 | dbx 实现 |
|---|---|
| 主组件 | `src/components/layout/EditorToolbar.vue`（每标签一行，h-9 固定高） |
| 运行/取消 | Run 按钮：`variant` 随 `activeTab.isExecuting` 在 `ghost`↔`destructive` 切换；图标 `Play`→`Square`（停止）；`isCancelling` 时显示 `Loader2 animate-spin`。点击逻辑：`activeTab.isExecuting ? emit('cancel') : emit('execute')` |
| 其它动作 | Explain（`GitBranch`/停止）、Autotrace（DM/PG/SQLServer，字母 A 切换 `aria-pressed`）、Format、Compress、关键字大小写切换（a/A）、SQL 语义诊断（`SpellCheck2`）、Redis 危险命令盾（`Shield`）、Save/Open/Import 结果归档、ExPaste、Multi-execute（`CirclePlay`→`MultiDbExecuteDialog.vue`）、事务组（auto/commit/rollback，`Tx:` 徽标 + `aria-pressed`） |
| 选择器 | 连接/目录(catalog)/库(database)/schema 四级 `SearchableSelect`，带生产上下文徽标（`ProductionContextBadge`）、加载态、`hexToRgba` 连接色 |
| 禁用逻辑 | `disabled="isCancelling || isExplaining || (!isExecuting && !executableSql.trim())"`；执行中禁用格式/压缩/事务切换 |
| 运行 gutter | `QueryEditor.vue` 的 `runStatementGutterExtension`：每行 `Play` 图标，作用域限定到该行语句/命令（`shouldShowStatementGutter`） |

> 复刻要点：`EditorToolbar` 是“能力按钮 + 运行/取消互斥切换 + 生产徽标 + 事务组”的典范。**运行按钮直接用 `isExecuting` 做 Run/Stop 互斥** 这一模式必须复刻；按钮级 `aria-pressed`/`aria-label`/`title` 对齐本仓库 `check-ui-a11y-logic.mjs` 门禁。

---

## 5. 取消 / 进度（Cancel / Progress）

| 维度 | dbx 实现 |
|---|---|
| Store（核心） | `useQueryStore`：`tab.isExecuting` / `isCancelling` / `executionId` / `queryExecutionStartedAt` / `cancelRequestCount` / `batchSqlExecution` |
| 取消 API | `cancelTabExecution(tabId)`、`cancelMultiDbExecutionScope(scopeId)`（并发取消所有 worker） |
| 超时常量 | `CANCEL_QUERY_TIMEOUT_MS = 10_000`、`CANCEL_ACK_SETTLE_TIMEOUT_MS = 2_000` |
| 可取消判定 | `lib/sql/queryExecutionState.ts`（`canCancelQueryExecution`） |
| 进度 | `applyBatchSqlProgress(tab, progress)`：逐语句 `status`(pending/running/success/error) + `executionTimeMs` + `affectedRows`；流式结果经 `appendQueryResultSegment`（按 `maxRows` 截断追加，检测列变更/重复） |
| 结果生命周期 | `resultRuns`（多次运行历史）、`resultEvicted`（LRU 驱逐）、`touchResult`；`closeQueryResult`/`clearQueryResults` 在 `isExecuting` 时拒绝 |
| UI 呈现 | `EditorToolbar` 取消按钮 + spinner；`DataGrid` 加载态（`QueryLoadingState`）；导出进度 `ExportProgressPopover` / `ExportProgressDialog`（`useExportTracker` 任务生命周期） |

> 复刻要点：**取消必须带超时与 ack 结算**，且 tab 级 `isExecuting/isCancelling` 是 UI 唯一真相源；批量进度用“语句级状态机 + 受影响行数”模型。这是行为级复刻（`REIMPLEMENT_FROM_BEHAVIOR`），因为本仓库 A4 的 DTO 仍在形成中。

---

## 6. 结果网格（Result Grids）

| 维度 | dbx 实现 |
|---|---|
| 主组件 | `src/components/grid/DataGrid.vue`（巨大）、`DataGridPagination.vue` |
| 工具栏 | `DataGridToolbar.vue`（**能力（capability）模式**：`DataGridToolbarActionCapability{visible,disabled,loading,label,tooltip}`，由 `lib/dataGrid/dataGridToolbar` 的 `select/trigger/toggle` 驱动） |
| 筛选/条件 | `DataGridQueryControls.vue`（WHERE/ORDER BY 内联编辑器 + 可拖拽分隔条）、`DataGridFilterBuilder.vue`、`DataGridConditionEditor.vue`、`DataGridTextFilterWorkbench.vue` |
| 复制/导出 | `DataGridCopyColumnNamesDialog.vue`、`DataGridExtractorDialog.vue`、`DataGridBulkEditDialog.vue`、`DataGridInsertRowsDialog.vue`、`DataGridCellDetailDialog.vue`/`Panel`、`DataGridColumnLayoutPopover.vue`、`DataGridTypeColorSchemeDialog.vue`、`ImagePreviewDialog.vue`、`LayerPreviewDialog.vue`、`EnumCellEditor.vue`、`TemporalCellEditor.vue` |
| 逻辑 composable | `useDataGridActions`(23KB)、`useDataGridExport`(58KB)、`useDataGridExport.sqlProgress`、`useDataGridSelection`(28KB)、`useDataGridColumnLayout`(33KB)、`useDataGridColumnResize`、`useDataGridConditionEditor`、`useDataGridFilterBuilder`、`useDataGridSearch`、`useDataGridSort`、`useDataGridAutoRefresh`、`useDataGridCanvasRuntime`(8KB) |
| 支撑 lib | `lib/dataGrid/{paginationPageSize,queryResultRowLimit,dataGridPagination,dataGridSql,dataGridNavigation,gridRowStatus,dataGridConditionHistory}`、`lib/table/{tableEditing,tableDependencySort}` |
| 分页 | `DataGridPagination` + `resolveDataGridPaginationTotal`/`canGoNextDataGridPage`/`canFetchNextDataGridSegment`；store 端 cursor/segment（`appendQueryResultSegment`） |
| 测试 | `composables/__tests__/useDataGrid*.spec.ts`（actions/export/selection/columnLayout/columnResize/conditionEditor/filterBuilder/search/sort/autoRefresh/canvasRuntime）、`useDataGridExport.sqlProgress.spec.ts` |

> 复刻要点：**“能力（capability）对象 + 工具栏渲染器”** 解耦是 dbx 网格最值得借鉴的架构——按钮可见性/禁用/加载都由纯数据驱动，便于单测与权限收敛。`useDataGridExport` 用 `vi.mock(api)` 隔离测试进度生命周期，是可直接套用的测试范式。

---

## 7. 历史（History）

| 维度 | dbx 实现 |
|---|---|
| 主组件 | `src/components/editor/QueryHistory.vue` |
| Store | `src/stores/historyStore.ts`（`useHistoryStore`）、`savedSqlStore.ts` |
| 关键行为 | `entries/loading/loadingMore/total/nextCursor/error/connectionOptions`；`search(request, append)` 游标分页（`loadMore`）；`requestSerial`/`mutationGeneration`/`destructiveMutations` **防止过期响应覆盖**（竞态守卫）；`add/remove/clear`；`setHistoryPanelActive` 切出面板即作废在途请求 |
| 写保护 | 写 SQL 经 `lib/database/productionExecutionGuard.ts` 的 `executeWithProductionSqlGuard`（生产库确认） |
| UI | `RecycleScroller` 虚拟滚动；防抖搜索；日期范围/连接/库过滤；上下文菜单（重跑/复制/删除）；`sr-only`/`focus:ring`/`title`/`aria` |
| 测试 | 由 historyStore 单测 + 复用 `useDataGrid*` 选择逻辑 |

> 复刻要点：**游标分页 + 请求串行号竞态守卫** 是 store 层的典范模式，应行为级复刻到本仓库的查询历史 store。`productionExecutionGuard` 对应本仓库的 A4 安全闸门。

---

## 8. 键盘 / 可访问性（Keyboard / Accessibility）

| 维度 | dbx 实现 |
|---|---|
| 快捷键注册 | `lib/editor/shortcutRegistry.ts`（`normalizeShortcutSettings`、`shortcutToCodeMirrorKey`）、`lib/editor/keyboardShortcuts.ts`（`isCancelSearchShortcut`）、`codemirror*` keymap（`searchKeymapWithoutModD`、`defaultKeymapForGlobalShortcuts`） |
| 编辑器快捷键 | CodeMirror 6 `Prec` compartment 动态重配（font/theme/wordWrap/lineNumbers/vim/sqlLanguage/completion/diagnostic…），全局快捷键与编辑器内快捷键分离 |
| 通用 a11y | 全站 `focus:ring`、`:focus-visible:outline-none`、`:aria-label`、`.sr-only`、`title` 提示、`role`/`aria-pressed`/`aria-expanded`/`aria-controls`、`CustomContextMenu.vue` |
| 对话框 | `Dialog`/`Popover`/`DropdownMenu` 组件内建 focus 管理 |
| 测试 | `ObjectBrowserClipboard.spec.ts`（`?raw` 契约）；组件级 a11y 主要靠源码契约 + `scripts/` 逻辑门禁 |

> 复刻要点：dbx 的“全局快捷键 / 编辑器内快捷键”双层 + `shortcutRegistry` 归一化，对应本仓库的 `check-client-navigation-logic.mjs` / 快捷键约定，应适配复用。a11y 基元（focus ring、aria-*）与本仓库 `check-ui-a11y-logic.mjs` 门禁天然对齐。

---

## 9. 加载 / 空 / 错误态（Loading / Empty / Error）

| 维度 | dbx 实现 |
|---|---|
| 加载 | `src/components/common/QueryLoadingState.vue`；store `loading/loadingMore`；`Loader2 animate-spin` 在工具栏/导出按钮 |
| 错误 | `src/components/ui/ErrorBanner.vue`；store `error` 字段（如 `historyStore.error`）；`translateBackendError` 后端错误本地化 |
| 空态 | 历史/网格在无数据时显示空态文案；列表 `empty-text` |
| 缺库提示 | `EditorToolbar` 的 `database-required-prompt`（抖动手势 + 红色） |

> 复刻要点：加载/空/错误是 store 一等字段 + 两个共享组件模式，应直接适配本仓库（`useXxxStore` 的 `loading/error` + 共享 `QueryLoadingState`/`ErrorBanner` 等价物）。

---

## 10. 响应式布局（Responsive Layout）

| 维度 | dbx 实现 |
|---|---|
| 全局 toolbar | `AppToolbar.vue`：`toolbarCollapsed`（宽度 < 屏宽一半即折叠）；`rightOverflowCount` + `settleRightOverflow()`（ResizeObserver + scrollWidth/clientWidth 测量，把溢出项收进 “More” `LightDropdown`）；macOS 红绿灯位置经 `invoke("set_macos_traffic_light_position")` 同步 |
| 编辑器 toolbar | `EditorToolbar.vue`：固定 `h-9`，flex 分组，连接色背景，`toolbarStyle` 由 `hexToRgba` 计算 |
| 主区 | `App.vue` / `ContentArea.vue` / `WelcomeScreen.vue` 区域切换；`app-toolbar`/`app-editor-toolbar` 固定高 + `shrink-0` |
| 测试 | `styles/__tests__/legacyWebviewFallback.spec.ts`（25KB，旧 webview 回退） |

> 复刻要点：dbx 的响应式是**“JS 测量 + 溢出折叠”**而非纯 CSS，toolbar 溢出项动态收进 More。该模式对窄窗/移动态稳健，应适配；但 `set_macos_traffic_light_position` 这类 Tauri 桌面专属同步可按需 REJECT（本仓库若不做 macOS 红绿灯对齐）。

---

## 11. 测试体系（Tests）— 跨切面

dbx 有 500+ spec，两类范式：

1. **composable / store 纯逻辑单测（主流）**：Vitest + `vi.mock` 隔离 `@/lib/backend/api`、stores、`vue-i18n`、`tauriRuntime`；以 `createOptions()` 工厂构造 `computed`/`ref` 入参，调用 composable 后断言行为。例：`composables/__tests__/useDataGridExport.sqlProgress.spec.ts`（mock `startQueryResultExport` 的 `onProgress` 回调，断言进度对话框生命周期）、`useSqlExecution.spec.ts`（55KB）、`stores/__tests__/connectionStore.completion.spec.ts`（70KB）。
2. **源码契约测试（`?raw`）**：`import src from "./X.vue?raw"` 后用正则 `.toMatch()` 断言“必须存在某段代码形态”（如 ObjectBrowser 粘贴归一化逻辑）。用于防结构漂移，不渲染 DOM。

> 注意：**`@vue/test-utils` 计数为 0** —— dbx 不做 DOM 渲染式组件测试，全部走“逻辑单测 + 源码契约”。这正契合本仓库 `scripts/*.mjs` 源码逻辑门禁思路，应直接采纳为复刻测试策略。

---

## 12. 许可证义务（License Obligations）

- dbx 源码为 **Apache-2.0**（含 `Cargo.lock` SHA-256 锁定，分发需保留 NOTICE/归因）。
- 复刻须遵守：① 复用任何 dbx 代码片段须保留 Apache-2.0 头与版权声明；② 若整文件移植，须在 `LICENSE`/第三方声明中登记；③ 建议优先 **行为级复刻 + 自写实现**（规避直接 copy 的归因负担与框架耦合），仅对明确可移植的小工具函数做带署名的 COPY。
- 本仓库现有 Apache-2.0 兼容策略（见 `dbx-study/dbx-冲突风险.md`）应延续。

---

## 13. 待办 / 移交（Handoff）

- 本仓库 M4-2 已埋 `useConnectionStore` / `database.rs` 骨架；A4 安全闸门（DTO）未合前，**不得**接通 `db_*` bridge 调用（见 blueprint DEFER 项）。
- 与 A4（安全/取消/生命周期）、A6（凭据/取消/生命周期）、A3（连接池/桥）强耦合：A5 的复刻蓝图须在 A4 DTO 冻结后方可进入 W19 实现。
- 完整复刻决策见 `A5-replication-blueprint.md`；本文件为证据/映射层。
