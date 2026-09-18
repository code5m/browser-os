# 09 — Independent Review (Reviewer)

Reviewer 不接受 Agent 多数意见为证据，重新抽样源码。攻击清单 + 结论。

## 1. 攻击：是不是又新增重复状态？
- 检查：目标模型是否引入 `gridVisible` 存储字段？
- 抽样：`01-STATE-MODEL.md §5` / `07-ALTERNATIVES §1` —— `gridVisible` 仅派生，不存。`check-grid-close-logic.mjs` 契约保留。
- **结论**：未新增重复状态。`desiredGridVisibility` 是 computed（useBrowserStore 内），非新真源。

## 2. 攻击：是不是把 gridVisible 变成第二真源？
- 检查：`useBrowserStore`/`useLayoutStore` 是否新增 `gridVisible` ref？
- 抽样：设计仅要求 computed `gridOpen && mainView === "grid"`（Final Reconciliation 冻结公式，check-view-intent.mjs C5 静态可检）。
- **结论**：否。C5 checker 防回归。

## 3. 攻击：是不是把 exitGrid(mode) 变成多义万能 API？
- 检查：Intent API 是否含 `mode` 分支？
- 抽样：`02-INTENT-MODEL.md §3` 方案 B —— `activate*` + `closeGrid()` 单义；`exitGrid` 被否决（07 §2）。
- **结论**：否。无 `mode` 多义 API。

## 4. 攻击：是不是让 Component 继续编排 lifecycle？
- 检查：组件是否仍直接写 `mainView` / 调 `buildGrid`？
- 抽样：现状 10+ 直写点（05 §1）；目标要求全部改经 `activateView`（C2/C3 静态可检）。
- **结论**：目标态禁止组件编排；C1/C2/C3 checker 保证。当前态确有此问题，正是 1B 要修的。

## 5. 攻击：是不是把 Native execution result 当 domain truth？
- 检查：是否有模块从 `webview.bounds` 回读可见性作为真相？
- 抽样：`04-NATIVE-VISIBILITY.md §2` —— Visibility Controller 单向派生；native 显隐是输出。
- **结论**：否。C12 标注 NOT_RELIABLY_CHECKABLE，靠 owner 单执行链 + Review 保证。

## 6. 攻击：是不是为统一 API 引入更复杂抽象？
- 检查：Visibility Controller / Intent API 是否过度设计？
- 抽样：Intent API 只是把现有 `buildGrid`/`closeGridAll`/`setView` 重命名为单义入口 + 收敛 `mainView` 写入点；Visibility Controller 只是把既有 `syncViewVisibility` 逻辑提炼为纯函数（04 §3）。**无新 Rust 命令、无新 enum、无新状态**。
- **结论**：未引入更复杂抽象；是收敛而非扩张。

## 7. 针对 B9-4 不回归的专项攻击
- 检查：`closeGrid` 后若 `mainView==="grid"` 是否仍会空白？
- 抽样：目标 `closeGrid()` 置 `gridOpen=false` 后，Visibility Controller 见 `desiredGridVisibility=false` 且 `mainView==="grid"` → 派生 `activateBrowser()`（05 §4）。`activateBrowser` → `mainView="browser"` → `schedulePosition` 重定位浏览器 webview 回屏。
- **结论**：从构造上消除 B9-4 类空白（不再依赖 `closeGridAll` 内手写拨位）。T12 验收覆盖。

## 8. Reviewer 裁决

| 问题 | 结论 |
|------|------|
| 重复状态 | 无新增 |
| gridVisible 第二真源 | 否 |
| exitGrid(mode) 多义 | 否 |
| 组件继续编排 | 当前有，目标禁止（1B 修） |
| native 当真源 | 否 |
| 过度抽象 | 否（收敛） |
| B9-4 回归 | 构造上消除 |

**总体：设计可信，无假绿。建议进入 Phase 1B 实施。**
