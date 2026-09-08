# A5 · M5-W18-R 检查点（Lane Checkpoint）

- **Lane**：A5 — dbx 桌面工作台 & 数据网格 UX（RESEARCH，第 5 合并序）
- **工作树**：`/home/ainfinit/.codex/worktrees/m5-w18-a5/mvp-browser-os-v3`（分支 `codex/m5-w18-a5`，已 rebase `origin/master`，干净）
- **研究性质**：仅研究 + 产出报告/检查点，**未改任何产品代码，未 push**（符合 W18-R 工作流与 NO_PUSH）
- **参考源**：dbx-src `/home/ainfinit/Documents/极智简单/V3/research/dbx-src`（Apache-2.0，Vue3+Tauri2，421 `.vue`）；既有 `dbx-study/` 分析

## 交付物（本目录）
| 文件 | 内容 |
|---|---|
| `A5-workbench-ux-map.md` | 10 项能力 + 测试/许可证的精确组件/store/测试映射与证据 |
| `A5-replication-blueprint.md` | Vue 专属复刻蓝图（COPY/ADAPT/REIMPLEMENT/DEFER/REJECT 分类 + 目标位置 + 测试策略） |
| `A5-checkpoint.md` | 本文件 |

## 研究范围覆盖（board L1399 全项 √）
连接树 ✓｜Schema 浏览器 ✓｜编辑器标签页 ✓｜执行工具栏 ✓｜取消/进度 ✓｜结果网格 ✓｜分页/筛选/复制/导出 ✓｜历史 ✓｜键盘/可访问性 ✓｜加载/空/错误态 ✓｜响应式布局 ✓

## 关键发现
1. **同栈可迁移**：dbx 即 Vue3+Tauri2，架构（store 驱动 + composable 纯逻辑 + 薄 SFC）与本仓库同构，适配成本低。
2. **三层解耦是核心资产**：`api`(`@/lib/backend/api`) 单点封装 ↔ 本仓库 `src/bridge.ts`+`src/types.ts`；能力对象模式（`DataGridToolbarActionCapability`）使工具栏纯数据驱动；store 的 `loading/error/nextCursor` + 竞态守卫（`requestSerial`）是健壮 UI 的范本。
3. **测试范式可直接采纳**：Vitest composable/store 单测（`vi.mock` 隔离）+ `?raw` 源码契约测试；**不做** DOM 渲染测试（`@vue/test-utils` 计数 0）。
4. **不可整段拷贝**：`QueryEditor.vue`(290KB)、`queryStore.ts`(323KB)、`ConnectionTree.vue`(132KB) 体量超限且含大量 dbx 专属数据库类型；须按行为自写 + 薄封装。
5. **a11y 内建**：focus ring / `aria-*` / `.sr-only` / `title` / 上下文菜单，天然对齐本仓库 `check-ui-a11y-logic.mjs`。

## 分类 tally
- COPY（带署名小单元）：3–4 处（分页纯函数、共享加载/错误组件、竞态守卫模式）
- ADAPT：连接树、Schema 浏览器(非 ER)、编辑器标签、执行工具栏、结果网格能力模式、键盘/a11y、加载/空/错误、响应式
- REIMPLEMENT_FROM_BEHAVIOR：取消/进度、历史（因 A4 DTO 未定 + 大体量 store）
- DEFER（超 M5）：ER 图、Multi-execute/事务组、Mongo/Redis/MQ 专属浏览器、AI 助手
- REJECT：290KB 编辑器内核(改薄封装)、macOS 红绿灯 Tauri 同步、整段 dbx 专属类型分支

## 许可证义务
- dbx = Apache-2.0；复用代码片段须保留版权头并在第三方声明登记；优先行为级复刻以规避归因负担。

## 阻塞 / 依赖（交 A0 / 等待）
- **A4 DTO 未冻结** → 不得接通 `db_*` bridge（board L120 LIMITED START 一致）；W19 实现前本蓝图仅规格冻结。
- 与 **A4**（安全/取消/生命周期）、**A6**（凭据/取消/生命周期）、**A3**（连接池/桥）强耦合，须在其交付后进入实现。
- 体积门禁 25.2%（`AC-5` 禁抬上限）：编辑器内核须动态分包，落地前须评估 pp 贡献。

## 下一步（W19 实现波，待 A0 开门）
1. 按蓝图在 `src/components`/`src/stores`/`src/composables`/`src/lib` 落地骨架（无命令调用）。
2. A4 DTO 冻结后接通 `db_*` bridge + `check-database-policy.py`。
3. 新增 `scripts/check-database-ui-logic.mjs` 承接 dbx 双测试范式。
4. 体积预算评估（尤其 CodeMirror 封装）。

## 状态
✅ RESEARCH 完成，全部交付物就绪，工作树干净（仅新增 `logs/research/M5-W18/A5-*`），**未 push**。待 A0 评审并打开 W19 实现。
