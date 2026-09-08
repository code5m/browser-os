# A5 · dbx 桌面工作台 UX 组件/Store/测试全景映射（M5-W18-R2 证据闭环修订版）

> 修订自 R1 `A5-workbench-ux-map.md`。R1 因引用不存在的 `useConnectionStore`、过时的 DTO 契约、未实测的体积/依赖判断被 A0 审计判定 `REWORK`（`logs/checkpoints/A0-M5-W18-R1-audit-20260908.md` L40）。
> 本版按 `PARALLEL_COMMAND_BOARD.md` L1439《M5-W18-R2 Evidence Closure Dispatch》重写：每一条事实标注证据类型，并显式 retract 每条被纠正的 R1 陈述。
>
> 证据类型图例：`CURRENT_PRODUCT` = 本仓库现存代码/配置；`REFERENCE_SOURCE` = dbx 参考源（`/home/ainfinit/Documents/极智简单/V3/research/dbx-src`，Apache-2.0）；`OBSERVED_BEHAVIOR` = 已执行验证；`EXECUTED_SYNTHETIC_TEST` = 已执行门禁/测试；`INFERENCE` = 推断（须显式标出，不冒充观测）。
>
> 参考源已复核存在且为 Apache-2.0（`[REFERENCE_SOURCE]` LICENSE 头；`git rev-parse` 在该目录无 git 仓库，故不锚定 commit，仅按当前工作树快照引用）。

---

## 0. 关键结论（修订）

1. **dbx 与本仓库同栈（Vue3 + Tauri2）是真**`[REFERENCE_SOURCE]`：前端 421 个 `.vue`、`apps/desktop/src/stores/` 下 13 个 store、`lib/backend/api.ts` 为 invoke 唯一封装面。架构（store 驱动 + composable 纯逻辑 + 薄 SFC）与本仓库同构。
2. **但本仓库数据库前端已是成型实现，不是空白**：`[CURRENT_PRODUCT]` `src/components/workspace/DatabasePanel.vue`（231 行 / 11,278 B）、`src/stores/useDatabaseStore.ts`（218 行 / 6,716 B）、`src/utils/dbUi.ts`（444 行 / 16,771 B）已落地，并由 `src/components/layout/MainArea.vue:50` 以 `defineAsyncComponent` 懒加载，门禁 `scripts/check-database-ui-logic.mjs`（274 行）**已存在且 119 断言全部通过**`[EXECUTED_SYNTHETIC_TEST]`。R1 把"新增 `check-database-ui-logic.mjs`"列为待办是**过时陈述**（文件早已存在并接线 pre-merge.sh:367/563）。
3. **体积硬约束已逼近上限**：`[CURRENT_PRODUCT]` 真基线 `logs/m0-build-metrics/build-metrics-4f0e8ab.json` 的 `dist.total_bytes=612,943`；`measure-build-metrics.py:39` 上限 `TOTAL_BYTES_GROWTH_LIMIT_PCT=25.2` ⇒ 允许上限 `612,943 × 1.252 = 767,404 B`。同目录 `build-metrics-052b18a.json` 记录 `795,517 B`（**脏树产物**，A6 已标 F-1），`pre-merge.sh:221` 用 `ls|sort|head -1` 正好选中它作基线，掩盖真实回归。R1 所谓"懒加载即解决体积"是**错误**：`collect_dist()`（`measure-build-metrics.py:63`）对 `dist/` 下**所有文件**求和，懒加载 chunk 仍计入 `total_bytes`，只改善首屏不改善门禁。
4. **产品无 Vitest、无 test 脚本**：`[CURRENT_PRODUCT]` `package.json` 仅 `dev/build/preview/tauri` 四个脚本、依赖仅 `vue/pinia/@tauri-apps/api/@xterm/*`（5 个运行时 + 2 个 dev）。R1 主张"直接采纳 dbx 的 Vitest 双范式"在**当前产品不可行**（需先引入 vitest 依赖，属体积/依赖扩张，违反 R2 限制与 25.2% 上限）。正确做法是沿用既有 `.mjs` 源码逻辑门禁范式。
5. **本仓库没有 `useConnectionStore`、没有 `src/lib/`**：`[CURRENT_PRODUCT]` `grep -rn useConnectionStore src` 命中 0 行；`src/` 仅 `App.vue bridge.ts components composables main.ts stores styles types.ts utils`。R1 反复引用"M4-2 已建 useConnectionStore / src/lib/editor/shortcutRegistry.ts"均为**不存在的目标**，须 retract。

---

## 1. R1 → R2 纠正表（显式 retract，保留历史）

| # | R1 陈述（误） | 纠正（R2） | 证据 |
|---|---|---|---|
| C1 | "M4-2 已埋 `useConnectionStore`" | 不存在；真实只有 `useDatabaseStore.ts`（含 `connections` ref，但无独立连接 store） | `grep -rn useConnectionStore src` → 0 `[CURRENT_PRODUCT]` |
| C2 | "新增 `scripts/check-database-ui-logic.mjs` 承接 dbx 双测试范式" | 该脚本已存在（274 行 / 119 断言通过），是既有门禁而非待办 | `scripts/check-database-ui-logic.mjs` + `EXECUTED_SYNTHETIC_TEST` 退出码 0 `[CURRENT_PRODUCT]` |
| C3 | "架构可直接借鉴；懒加载可把编辑器内核拆出去规避 25.2%" | 懒加载仍计入 `total_bytes` 门禁；当前净余量 ≈ 345 B（见 §5），**无法容纳任何净新组件** | `measure-build-metrics.py:63-77` `collect_dist` 全量求和 `[CURRENT_PRODUCT]` |
| C4 | "直接采纳 dbx Vitest 单测 + `?raw` 契约" | 产品无 Vitest；须沿用 `.mjs` 门禁范式，新增断言进 `check-database-ui-logic.mjs` | `package.json` 无 vitest/test `[CURRENT_PRODUCT]` |
| C5 | "建议新建 `src/lib/editor/shortcutRegistry.ts`" | `src/lib/` 不存在；键盘归一化应落在既有 `src/composables/` 或并入 `dbUi.ts` | `ls src/` 无 `lib/` `[CURRENT_PRODUCT]` |
| C6 | 取消常量引自 `lib/sql/queryExecutionState.ts` | dbx 实际位于 `apps/desktop/src/stores/queryStore.ts:105-106`（`CANCEL_QUERY_TIMEOUT_MS=10_000`、`CANCEL_ACK_SETTLE_TIMEOUT_MS=2_000`） | `grep` 复核 `[REFERENCE_SOURCE]` |
| C7 | `DataGridToolbarActionCapability` 定义在 `lib/dataGrid/dataGridToolbar` | 实际为 `apps/desktop/src/components/grid/DataGridToolbar.vue:18` 的内联 `type` | `grep` 复核 `[REFERENCE_SOURCE]` |
| C8 | a11y"天然对齐 `check-ui-a11y-logic.mjs`" | 该门禁仅 74 行、9 断言，只覆盖 `modalA11y.ts` 的 modal 焦点决策，**不覆盖** grid/toolbar aria；数据库面板须自建 a11y 断言 | `scripts/check-ui-a11y-logic.mjs` 头部注释 `[CURRENT_PRODUCT]` |

---

## 2. CURRENT_PRODUCT 真实前端盘点（A5 工作唯一真相源）

| 维度 | 真实现状（证据） |
|---|---|
| 连接/查询状态 | `useDatabaseStore.ts`：form / connections / activeId / sql / result / busy / error / risk / verdict / pendingSql；`backendReady`(`bridge.dbConnect/dbQuery/dbDisconnect` 三函数存在性) / `connected` / `runGate` / `requiresConfirm` / `confirmOpen` 等 computed。 |
| 命令层 | `src/bridge.ts:375-382`：`dbConnect(cfg,password)` / `dbQuery(p)` / `dbDisconnect(conn_id)`，调用后端 `db_connect/db_query/db_disconnect`（A4 已落地）。`[CURRENT_PRODUCT]` |
| 纯逻辑层 | `src/utils/dbUi.ts`：连接表单校验、payload 构造（**结构性无 password**，F2）、`DbValue` 解码（`snake_case`：null/bool/int/float/text/blob_len，`[CURRENT_PRODUCT]` types.ts:600-611 与 domain.rs 对齐，已修 B8-1）、结果视图、CSV、截断/状态告警、风险/生产文案、运行门禁。 |
| 视图 | `DatabasePanel.vue`：连接表单 + SQL textarea + 运行/复制CSV/清空 + 写二次确认（**内联 `<div class="confirm">`，非 ConfirmModal**）+ 结果表（首 200 行 `DISPLAY_ROW_CAP`，`maxRows=1000` 后端上限另算）。**零 `aria-*`/`role`/`tabindex`/`@keydown`**（`grep -c` = 0）。 |
| 懒加载 | `MainArea.vue:50` `defineAsyncComponent(() => import("../workspace/DatabasePanel.vue"))`，带 `panelLoading`(`role=status aria-live=polite`) / `panelError`(`role=alert`) 兜底。 |
| 可复用资产 | `src/components/shared/ConfirmModal.vue` + `src/composables/useModalFocus.ts`（已用于 `GitWriteConfirmDialog.vue` 的写确认）；`useLayoutStore.showToast(text)`。 |
| 门禁 | `check-database-ui-logic.mjs`（274 行 / 119 断言 / 退出 0，已接线 pre-merge）；`check-ui-a11y-logic.mjs`（仅 9 断言覆盖 modalA11y）。 |
| 依赖/构建 | `package.json` 5 运行时 + 2 dev；`measure-build-metrics.py` 上限 25.2%；`cargo_warnings` 门禁要求"只减不增"。 |

---

## 3. dbx 参考源逐能力映射（带实测尺寸）

> 所有尺寸为 `wc -c` 实测（`[REFERENCE_SOURCE]`，当前工作树快照，不锚 commit）。

| # | 能力 | dbx 主文件（实测字节） | 关键符号（已复核） |
|---|---|---|---|
| 1 | 连接树 | `ConnectionTree.vue` 132,311；`connectionStore.ts` | `TreeNode`、`canUseLoadedTreeNodeToggle`、`searchAutoExpandedNodeIds` |
| 2 | Schema 浏览器 | `ObjectBrowser.vue` 180,772；`TableStructureEditor.vue` | `RecycleScroller`、`useSchemaOptions`/`useDatabaseOptions`、`executeWithProductionSqlGuard`（遍布 objects/*） |
| 3 | 编辑器标签 | `QueryEditor.vue` **291,246**；`queryStore.ts` **324,752** | `tabs: QueryTab[]`、`reorderTab`、`closeConfirmDirtyTabIds`、`TabExecutionStatus` |
| 4 | 执行工具栏 | `EditorToolbar.vue` 32,173 | Run/Stop 互斥 `activeTab.isExecuting ? cancel : execute`；`disabled=isCancelling||isExplaining||(!isExecuting&&!executableSql.trim())` |
| 5 | 取消/进度 | `queryStore.ts:105-106`（常量）；`queryStore.ts` 整体 | `CANCEL_QUERY_TIMEOUT_MS=10_000`、`CANCEL_ACK_SETTLE_TIMEOUT_MS=2_000`、`cancelTabExecution`、`appendQueryResultSegment`、`resultRuns/resultEvicted` |
| 6 | 结果网格 | `DataGrid.vue` **661,097**；`DataGridToolbar.vue:18`（`DataGridToolbarActionCapability`） | `DataGridToolbarActionCapability{visible,disabled,loading,label,tooltip}`、`useDataGridExport` 等 composable |
| 7 | 历史 | `historyStore.ts` 6,794；`QueryHistory.vue` 36,513 | `entries/loading/loadingMore/total/nextCursor/error`、`search(req,append)`、`requestSerial` 竞态守卫、`setHistoryPanelActive` |
| 8 | 键盘/a11y | `shortcutRegistry.ts`、`keyboardShortcuts.ts` | `normalizeShortcutSettings`、`shortcutToCodeMirrorKey`；全站 focus ring / `aria-*` / `.sr-only` / `CustomContextMenu` |
| 9 | 加载/空/错误 | `QueryLoadingState.vue` 1,843；`ErrorBanner.vue` 4,207 | `QueryLoadingState`、`ErrorBanner`、`translateBackendError` |
| 10 | 响应式 | `AppToolbar.vue` | `toolbarCollapsed` + `rightOverflowCount` + `settleRightOverflow()`（ResizeObserver + scrollWidth 测量）；`set_macos_traffic_light_position`（macOS 专属，REJECT） |
| 11 | 测试 | `*.spec.ts` 多处 | composable/store 单测（`vi.mock`）+ `?raw` 源码契约；`@vue/test-utils` 计数 0 |

---

## 4. 能力 → 当前产品 映射与最小改动意图

| # | 能力 | 当前产品落点 | 最小改动意图（不新增文件/依赖前提下） |
|---|---|---|---|
| 1 连接树 | `useDatabaseStore.connections` + `DatabasePanel.vue` `conn-list` | 当前仅有静态 `connections` 列表（`refreshConnections()` 为空实现，A4 未给列表命令）。**最小改动**：保持空实现，W19 接 `db_list_connections` 后再填充；不新建连接树 store。 |
| 2 Schema 浏览器 | 无当前对应 | **DEFER**：本 M5 即连即查模型无 schema 树。W19 可由 A4 提供 schema 浏览命令后再做，且用 `RecycleScroller` 等价（本仓库无虚拟滚动库，须自写或复用原生滚动）。 |
| 3 编辑器标签 | `db.sql` 单 textarea | **REJECT 多标签内核**：当前单语句查询模型不需要 tab 系统。CodeMirror 290KB 内核在 0 余量下**禁止引入**（见 §5/C9）。保持 `<textarea>`。 |
| 4 执行工具栏 | `DatabasePanel.vue` 的"运行/复制CSV/清空"按钮 | **ADAPT（小）**：补齐 Run/Stop 互斥符号与 `disabled` 公式；为每个动作加 `:aria-label`/`:title`，写类按钮加 `:aria-pressed`。 |
| 5 取消/进度 | `useDatabaseStore.busy` / `pendingSql` | **REIMPLEMENT_FROM_BEHAVIOR**：当前仅 `busy` 布尔，无取消令牌。W19 接 A4 取消 DTO 后，于 `useDatabaseStore` 内增 `isExecuting/isCancelling/executionId`，常量照搬 dbx `10_000/2_000` 语义（`[REFERENCE_SOURCE]` queryStore.ts:105-106），但实现自写。 |
| 6 结果网格 | `DatabasePanel.vue` `<table>`（首 200 行） | **ADAPT（小）**：保持原生 `<table>`；把"复制/导出"动作建模为能力对象 `{visible,disabled,label,tooltip}` 数据结构，便于单测；分页/筛选仅在有大数据量需求时做（当前 `maxRows=1000` 后端截断，前端无需分页器）。 |
| 7 历史 | 无当前对应 | **DEFER/W19**：当前无历史持久化（A4 历史写入契约未定）。竞态守卫 `requestSerial` 模式记为 W19 实现要点。 |
| 8 键盘/a11y | 全组件 | **ADAPT**：给 `DatabasePanel` 加 `aria-label`/`role`/`tabindex`、`@keydown` 在 textarea（Ctrl/Cmd+Enter 运行）；复用 `useModalFocus` 于写确认弹窗。 |
| 9 加载/空/错误 | `db.busy`/`db.error`/`result` | **ADAPT/COPY 小**：`QueryLoadingState`/`ErrorBanner`（1,843/4,207 B）结构可移植，但**须评估字节余量**（见 §5）；更稳妥是内联等价 `<div role=status>`。 |
| 10 响应式 | `DatabasePanel` 内 `<style scoped>` | **ADAPT**：本面板已在侧栏内滚动，无需溢出折叠；REJECT macOS 红绿灯同步。 |

---

## 5. 体积硬约束（基于实测，决策前提）

- 真基线 `4f0e8ab`：`dist.total_bytes = 612,943 B` `[CURRENT_PRODUCT]`。
- 上限 `25.2%` ⇒ 允许 `767,404 B`；当前干净 `052b18a` 实测 `767,059 B`（A6 记忆），**净余量 ≈ 345 B（0.06pp）**。
- 结论（决策级）：**在 25.2% 上限且 `cargo_warnings` 不增的前提下，M5 阶段新增任何净字节的 UI 组件（含 CodeMirror、ErrorBanner、LoadingState 等）都不可行**。任何蓝图级改动必须"零净增"或"替代既有行内代码"，或等 A0 书面抬限（`AC-5` 当前禁止）。
- **C9（体积结论）**：CodeMirror 6 引入属新增依赖（npm 包 + 分包），且 `@codemirror/*` 即便动态分包仍计入 `dist.total_bytes`，在 0 余量下**禁止**。R2 明确"不实测需求与体积不得选 CodeMirror"——本仓库未做实测，故分类 **REJECT（M5）/ DEFER（待 A0 在 disposable 工作区授权 footprint 测量）**。

---

## 6. 许可证义务（已修正 R1）

- dbx = **Apache-2.0**（LICENSE 头已确认）`[REFERENCE_SOURCE]`。
- 本仓库 `LICENSE` = **MulanPSL-2.0**（A0 审计 L31 已指正 A8 的"Apache-2.0"误述；A5 在此确认本仓库为 MulanPSL-2.0）。
- 含义：从 dbx 移植代码须遵守 **MulanPSL-2.0 接受 Apache-2.0  inbound 的兼容义务** + Apache-2.0 的 `NOTICE`/版权头保留；A10 负责最终移植账本与署名。A5 只做行为级复刻（自写实现），规避直接 copy 的归因负担。

---

## 7. 遗留风险 / 移交（Handoff）

- **B-A5-1（High，依赖 A4）**：`db_list_connections`、`db_cancel`、历史命令名与 DTO 未冻结 ⇒ 连接树列表填充、取消令牌、历史三大能力均 **W19 才能落地**。
- **B-A5-2（High，体积）**：25.2% 余量 ≈ 345 B，任何净新组件须先解决体积预算（替代既有代码 / A0 抬限）。
- **B-A5-3（Med，DbValue 漂移）**：本仓库 `types.ts:600` 与 `domain.rs` 现已对齐为 `snake_case`；但 `database.rs` 后端 Rust 侧 `DbValue` 变体为 `I64/F64/Binary{bytes}`，与 `domain.rs` 的 `Int/Float/BlobLen` 仍是双真源（A1 G4 / A4 已记）。A5 仅消费 `types.ts` 镜像，漂移修复归 A1/A4。
- **B-A5-4（Low）**：`check-database-ui-logic.mjs` 当前只覆盖 `dbUi.ts` 纯逻辑；面板组件级 a11y/键盘断言尚缺，须在本 lane 后续或 W19 扩写。
- 与 **A4**（DTO/安全闸门）、**A6**（凭据/取消/生命周期）、**A3**（连接桥）强耦合，顺序：A4 DTO 冻结 → A5 可进入 W19 实现。

---

## 8. 待办（本 lane R2 交付内可完成项）

- ✅ 已 retract R1 全部误述并补齐当前产品真实盘点（本文件 §1-§2）。
- ✅ 已把"懒加载 / Vitest / CodeMirror / useConnectionStore / src/lib"五项误述纠正，给出最小改动意图（§4）与体积硬结论（§5）。
- ⏸ 产品代码实现冻结至 W19（R2 禁止改 `src/`）。本文件为**证据/映射层**，落地代码归 W19 实现卡。
