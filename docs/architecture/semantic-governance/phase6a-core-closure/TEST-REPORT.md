# Phase 6A — Test Report

> 环境：Node v26.7.0（原生支持 `.ts` import）；项目无 vitest（`package.json` 无测试框架），故沿用仓库既有 `.mjs` checker 约定，不新增框架（避免未请求的工程面扩张）。

## 1. `node scripts/check-semantic-closure-logic.mjs`

```
SEMANTIC_CLOSURE_LOGIC_RESULT=PASS (26/26)
```

### 6A-1 aiNavOpen（打开 / 关闭 / 读取路径 / writer 唯一）

| 断言 | 结果 |
|-|-|
| aiNavOpen 在 useBrowserStore 中恰好声明 1 次 | ok |
| useLayoutStore 不再声明 aiNavOpen（死重复已删） | ok |
| 源码无 `layout.aiNavOpen` 第二真源 | ok |
| 初始关闭 | ok |
| `toggleAiNav()` 打开 | ok |
| `toggleAiNav()` 再次关闭 | ok |
| `gotoAI` 关闭 aiNavOpen（writer 唯一） | ok |

### 6A-2 Panel（每个面板行为不回归；不误合并）

| 断言 | 结果 |
|-|-|
| sidebarOpen / clipOpen / fileEditorOpen / browserDockOpen / browserDockTab 各恰好声明 1 次 | ok ×5 |
| bmPanelOpen 在 MainArea 是 computed 派生量 | ok |
| bmPanelOpen 派生自 `panelOpen && mainView==='browser'` | ok |
| `toggleSidebar` 仅切 sidebarOpen，不动 bookmark | ok |
| `toggleClipboard` 仅切 clipOpen 并导航 clip 视图 | ok |
| `toggleBrowserDock` 切 browserDockOpen 且子页签=files | ok |
| `togglePanel` 仅切 bookmark.panelOpen | ok |
| bmPanelOpen 派生 = panelOpen && mainView==='browser' | ok |

### 6A-3 GridSession（打开 / 恢复 / 状态保持）

| 断言 | 结果 |
|-|-|
| gridSession 在 useBrowserStore 中恰好声明 1 次 | ok |
| gridSession 仅由 buildGrid/forceGridRelayout 写入（2 处） | ok |
| gridSession 初始为 0 | ok |
| `forceGridRelayout` 使 gridSession 自增 +1（缓存失效纪元） | ok |
| gridSession 不落 localStorage（内存 runtime，非持久化） | ok |

> 运行时"gridSession 变化后缓存失效 → 重发定位"另由既有 `scripts/check-browser-sync-logic.mjs` G8 覆盖（native 定位层）。

## 2. `node scripts/check-semantic-registry.mjs --self-test`

```
SELF_TEST_RESULT=ALL_PASS
  ✓ positive fixture: 0 fail/warn
  ✓ negative fixture R1/R2/R3/R4/R5/R6/R7/R8 全检出
  ✓ false-positive fixture: 0 fail/warn
```

## 3. `node scripts/check-semantic-registry.mjs`（真实仓库扫描）

```
fail=0 warn=6 info=72
SEMANTIC_REGISTRY_RESULT=PASS
```

- `fail=0`：R8 在真实仓库无违规（三项收敛生效）。
- `warn=6`：既有 R5 副作用认知提示（非本次引入）。
- `info=72`：`observed_not_governed` 提示（治理外域，按设计保留）。

## 4. 回归

| 项 | 结果 |
|-|-|
| `read_lints`（useBrowserStore / useLayoutStore / ActivityBar） | 0 error |
| `git diff --check` | 干净 |
| `check-browser-sync-logic.mjs`（gridSession 相关既有测试） | 未改动，保持通过 |
