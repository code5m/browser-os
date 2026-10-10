# SCR-20261010 — Adaptive Chrome shellMode

## New Semantic
- 名称：`shellMode`，State，`CURRENT_FACT`（PR #18 已提交）
- Owner：`useLayoutStore`；`setShellMode` 是唯一写入口。
- 含义：标准 `standard`、紧凑 `compact`、沉浸 `immersive` 三种 UI 外壳显示意图。
- 不含义：原生窗口实际 fullscreen、WebKitGTK 创建/销毁状态、浏览器导航或宫格生命期。

## Why Existing Semantic Cannot Represent
- 已查询 `mainView`、`compactMode`、`navDensity`、`browserDockOpen`。
- `mainView` 是内容视图；`navDensity` 是随宽度变化的密度；`browserDockOpen` 是侧栏；都不能表示独立的 UI 显示意图。
- `compactMode` 由旧 ref 改为 `computed(() => shellMode === "immersive")`，作为派生兼容字段，不再产生第二个可写状态。

## Alternatives and Ownership
- 独立的多个布尔 ref 会造成组合不合法，故采用单一三态。
- Owner 不变：仍为 `useLayoutStore`；App 只协调主窗口 fullscreen API，Native 实际状态不写入 Store 的 shellMode 作为重复真源。

## Checker Impact
- `docs/architecture/semantic-registry/states.yaml` 的 `observed_not_governed.useLayoutStore` 新增 `shellMode`，与旧 `compactMode` 同层。待后续正式治理升级时登记完整状态契约。
- 不改 Checker 代码、不放宽任一现有检查。
- 预期：`SEMANTIC_UNREGISTERED_STATE` 归零；必须以实际 CI 为准。

## ADR
- 产品设计决策：`docs/product/BROWSEROS-UX-MODULAR-LOW-RESOURCE-DECISIONS-20261010.md` §12，用户已经明确授权实施。
- 本次属于已有批准产品方案的语义登记，暂不修改原生 GTK 生命周期。

## 验证
- [ ] `node scripts/check-semantic-registry.mjs --self-test`
- [ ] `node scripts/check-semantic-registry.mjs`
- [ ] `bash scripts/pre-merge.sh`
- [ ] 真实 Linux GUI 验收

本次 SCR 为实现补登记，不能凭此文档宣称已有 GUI_PASS 或所有门禁通过。
