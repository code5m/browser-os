# A8 · M5 BUG-HUNT Follow-up · 宫格关闭空白回归（B9-4）— 验收细节

> Lane A8 在 BUG-HUNT Follow-up Dispatch 中由 MANUAL QA 升级为 CODE：
> 修复 `useBrowserStore.closeGridAll`，使关闭宫格后活动页签/视图与原生 webview
> 布局不会让浏览器区空白，并保留网格进程边界。附 focused UI test。

## 1. 现象与定位（证据来自 BUG-HUNT-SUMMARY.md B9-4）

- 文件：`src/stores/useBrowserStore.ts:458-468`（修复前）
- 摘要原文：`closeGridAll 后未复位活动页签坐标、mainView 不变、watch 不触发 → 浏览器区空白，须手动切视图恢复。`
- 链路确认（读源码）：
  - `isBrowserVisible = () => !gridOpen.value && layout.mainView === "browser"`（store L146）
  - `BrowserHost.vue:15` `:style="{ visibility: browser.isBrowserVisible ? 'visible' : 'hidden' }"`
  - `useBrowserHost.schedulePosition` L31：`if (layout.mainView !== "browser" || ...) return;`（mainView 非 browser 直接跳过定位）
  - `MainArea.vue:133` browser/grid 共用同一模板块，靠 `gridOpen` + `isBrowserVisible` 区分显隐

## 2. 修复 diff（要点）

```ts
async function closeGridAll() {
  gridOpen.value = false;
  await bridge.closeGrid().catch(() => {});
  layout.gridToolbarOpen = false;
  gridRects.splice(0, gridRects.length);
  // B9-4: mainView 复位为 browser（仅当当前确为 grid 视图），重激活活动页签，兜底重定位
  if (activeTabId.value) await bridge.tabActivate(activeTabId.value).catch(() => {});
  if (layout.mainView === "grid") layout.mainView = "browser";
  syncFreeze();
  schedulePosition();
  layout.showToast("已关闭宫格");
}
```

可观测行为：关闭宫格后视图自动落回浏览器、活动页签 webview 重定位进浏览器区（不再空白），网格进程仍由 `bridge.closeGrid()` 统一回收。

## 3. 验证

| 项 | 命令 | 结果 |
|---|---|---|
| focused UI test | `node scripts/check-grid-close-logic.mjs` | 通过 12 / 失败 0，`GRID_CLOSE_RESULT=PASS` |
| 构建 | `npm run build` | built in 3.33s，无错误 |
| 集成闸门 | `bash scripts/pre-merge.sh` | `PRE_MERGE_RESULT=ALL_PASS` |

## 4. 诚实边界（RISK）

- headless 运行环境无法启动 Tauri 原生桌面客户端，**未做真实 GUI 渲染验收**；B9-4 修复以源码级断言 + `isBrowserVisible` 公式契约仿真闭环，逻辑正确但需用户在原生客户端目视确认一次。
- 复位仅在 `mainView === "grid"` 时执行，处于 files/term 等视图期间关宫格不会强行跳回 browser（符合"保留网格进程边界、不打扰其它视图"约束）。

## 5. 交付物

- 补丁：`logs/checkpoints/Lane-A8-M5-BUG-HUNT-grid-close-20260907-2200.patch`
- checkpoint：`logs/checkpoints/Lane-A8-M5-BUG-HUNT-grid-close-20260907-2200.md`
- 测试：`scripts/check-grid-close-logic.mjs`（新增）
- NO_PUSH：confirmed（仅 A0 集成/提交/推送）
