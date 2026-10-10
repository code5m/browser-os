# SCR-20261010 — Adaptive Chrome shell mode

## New Semantic
- 名称：`shellMode`（stored），`compactMode`（derived compatibility）
- 类型：State、Intent（`setShellMode`）
- 分类：CURRENT_FACT（仅代码已提交的状态层；原生 GUI 行为仍需实机验收）
- Owner：`useLayoutStore`；Window API 操作由 `App.vue` 响应用户意图执行。

## Why Existing Semantic Cannot Represent
已查询 `states.yaml` 的 `mainView`、`sidebarOpen`、`browserDockOpen`、`gridToolbarOpen`，及 `observed_not_governed.useLayoutStore.compactMode`。

- `mainView` 决定中央内容是 browser/files/grid 等，不应承载外壳密度和操作系统窗口的全屏状态。
- `compactMode` 是旧的单一 Boolean，不能同时表达 standard / compact / immersive 三种互斥的视图外壳。
- `shellMode` 是唯一存储的 UI 模式状态；`compactMode = computed(shellMode === "immersive")` 仅兼容旧调用，避免两个可写布尔状态发生偏差。
- OS window fullscreen 的观测值在 `App.vue` 为临时原生反馈，不进入 Pinia 的第二个真源。

## Alternatives & No Duplicate Truth
| 既有语义 | 原因 |
|---|---|
| mainView | 视图内容切换与布局密度无关 |
| compactMode | 已改为只读派生量，兼容旧组件与观察者 |
| navDensity | 依据窗口宽度自动计算工具栏标签密度；不等于用户选择的工作模式 |

## State / Intent / Owner Impact
- `shellMode` stored，允许值 `standard/compact/immersive`，唯一写入点 `useLayoutStore.setShellMode`；
- `toggleCompact` 保留历史兼容入口，只委托到 `setShellMode`；
- `compactMode` derived，禁止单独写入；`mainView`、模块生命周期和 GTK 几何 owner 均不改变；
- 模式变更应触发既有 WebView Host 单一重定位通道，不能创建/销毁 webview。
- 前端 Window API 的真实 fullscreen 成功/失败与 store 状态收敛由 UI 工作流负责，需 Linux GUI 验证。

## Checker Impact
- [x] `states.yaml`：新增 `shellMode` canonical stored 语义、`compactMode` derived 语义；从未治理清单移除旧 `compactMode` 条目。
- [ ] `intents.yaml`：本次使用 `setShellMode` 单一入口（与已有的视图导航意图不重名）；若 checker 后续要求完整 intent 治理再单独登记。
- [ ] `owners.yaml`：owner 未变化。
- [ ] `side-effects.yaml`：未引入新的原生 bridge side effect。
- [ ] `check-semantic-registry.mjs`：不得修改检查器以绕过失败。

## Decision
- [x] APPROVED — 按 2026-10-10 已确认的产品设计文档第 12 章执行；此决定是自主提交的项目审核记录，不冒充人工独立 Reviewer。
- 记录来源：`docs/product/BROWSEROS-UX-MODULAR-LOW-RESOURCE-DECISIONS-20261010.md` §12。
- ADR：该产品决策文档第 12 节作为设计裁决，未修改冻结的 GTK / WebView 生命周期契约。
- Reviewer：Agent self-review（根据 Owner 已授权的本轮改造）；日期：2026-10-10。

## 落地检查（CI 后更新）
- [ ] `node scripts/check-semantic-registry.mjs --self-test` PASS
- [ ] `node scripts/check-semantic-registry.mjs` 无新增 FAIL
- [x] Registry 与代码同一功能分支同步
- [x] 本 SCR 已归档
