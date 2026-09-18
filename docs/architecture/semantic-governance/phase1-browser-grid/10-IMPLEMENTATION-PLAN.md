# 10 — Implementation Plan (Phase 1B)

设计已完成，本轮不实施。以下为可并行的 Phase 1B 子任务草案，供后续派发。

## 0. 原则
- 仅新增 Intent API + 收敛 `mainView` 写入点 + 提炼 Visibility Controller。
- **不**新增 Rust 命令、不新增 enum、不新增存储状态。
- 每子任务自带 checker/测试，独立可验收。

## 1. 子任务拆分

| Batch | 任务 | 文件所有权 | 依赖 | 验收 |
|-------|------|-----------|------|------|
| **A** | Intent API 落地：新增 `activateView/activateBrowser/activateGrid/openGrid/closeGrid/closeGridCell` 于 `useBrowserStore`；`buildGrid`/`closeGridAll`/`closeGridOne` 内部化，不再直写 `mainView` | `useBrowserStore.ts` | 无 | C1/C8 静态 PASS；单测 |
| **B** | Visibility Controller 提炼：把 `syncViewVisibility` 逻辑抽为纯函数（`mainView,gridOpen,activeTabId → 每 webview desired 状态`）；`closeGrid` 后若 `mainView==="grid"` 派生 `activateBrowser` | `useBrowserStore.ts` + 新 `utils/visibilityController.ts` | A | T12 / C9 静态 PASS |
| **C** | Component 迁移：替换所有直写 `mainView` 为 `activateView`；`MainArea` 改用派生 `browserVisible`/`gridVisible` computed（不直读 `gridOpen` 决定 `v-show`） | `App.vue` `MainArea.vue` `HomeLaunchers.vue` `FileEditor.vue` `UnifiedTabBar.vue` `ActivityBar.vue` + 各 store（session/workbench/workspace/system/home） | A,B | C2/C3/C4 静态 PASS；T1–T5/T11 |
| **D** | Native visibility adapter：确保 Visibility Controller 仅编排既有 `bridge.tab_position`/`grid_position`/`hide_all_webviews`，不新增 Rust 命令 | `bridge.ts`（无新增）/ `utils/visibilityController.ts` | B | C6 静态 PASS；T8/T9 resize |
| **E** | Regression tests：扩展 `check-grid-close-logic.mjs`（C7）+ 新增 `scripts/check-view-intent.mjs`（C2/C3/C5）；Vitest 覆盖 T1–T12 | `scripts/check-grid-close-logic.mjs` `scripts/check-view-intent.mjs` `*.spec.ts` | A,B,C | npm run check 绿；T1–T12 |
| **F** | Checker：把 C1–C9 固化进 `pre-merge.sh` 门禁 | `scripts/pre-merge.sh` | E | pre-merge --self-test PASS |
| **G** | Runtime validation：headless 验证 `gridOpen=true,mainView=grid` 时 `BrowserHost` 保留 rect（C10）、切换 20 次无 URL 丢失（C11） | `*.spec.ts` | C,D | T5/T10/T11 |
| **Reviewer** | 独立复审 1B 交付 | — | A–G | REVIEW PASS |

## 2. 依赖关系（不可并行）
- B 依赖 A（Visibility Controller 调新 Intent API）。
- C 依赖 A+B（组件改经 Intent API）。
- D 依赖 B（adapter 编排 Controller 输出）。
- E/F/G 依赖 A–D。

## 3. 不能并行的文件（写入冲突）
- `useBrowserStore.ts`：A + B 都改 → 串行。
- `MainArea.vue`：C 改 → 与 B 的 computed 消费串行。
- `scripts/pre-merge.sh`：F 改 → 单独 batch。

## 4. 回滚边界
- 每 batch 独立 commit，任一 batch 失败可单独 revert 不影响其它。
- 回滚判据：C2/C3 静态 checker 失败 → 回滚该 batch；T1–T12 任一失败 → 回滚对应 batch。
- 不引入数据库迁移/构建体积超 25.2% 硬限（`TOTAL_BYTES_GROWTH_LIMIT_PCT`）；Intent API 为函数重命名，体积极小。

## 5. 禁止项（红线）
- 不得新增 `gridVisible` 存储字段（C5）。
- 不得新增 `exitGrid(mode)`（07 §2）。
- 不得新增 Rust `show_*` 命令（C6）。
- 不得新造 `GridLifecycle` enum（07 §4）。
- `isBrowserVisible` 公式不得回退（C7）。
- 不得为统一 API 扩大重构范围（仅收敛写入点）。

## 6. 验收总门
- `npm run check` 全绿（含新增 `check-view-intent.mjs`）。
- `bash scripts/pre-merge.sh --self-test` PASS。
- T1–T12 全过（无 URL 丢失 / 无空白 / 无资源泄漏）。
- Reviewer（G-Reviewer）独立 PASS。
