# Lane A8 · M5 BUG-HUNT Follow-up · 宫格关闭空白回归（B9-4）

```
LANE: A8
STATUS: PASS
SCOPE: src/stores/useBrowserStore.ts (仅 closeGridAll), scripts/check-grid-close-logic.mjs (新增 focused UI test)
DELIVERED: 关闭宫格后浏览器区不再空白：mainView 复位为 browser、活动页签重激活并重定位回浏览器区；保留网格进程边界。
VERIFY: node scripts/check-grid-close-logic.mjs → 通过 12，失败 0 (GRID_CLOSE_RESULT=PASS)
VERIFY: npm run build → built in 3.33s (无回归)
VERIFY: bash scripts/pre-merge.sh → PRE_MERGE_RESULT=ALL_PASS
METRICS: N/A (纯前端逻辑修复；dist 体积仍 < 25.2% 上限，无体积/性能回归)
PATCH: /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3/logs/checkpoints/Lane-A8-M5-BUG-HUNT-grid-close-20260907-2200.patch
RISKS: headless 环境无法启动 GUI 桌面客户端，未做真实原生 webview 渲染验收；B9-4 修复经源码级断言 + isBrowserVisible 契约仿真闭环。closeGridAll 仅在 mainView==="grid" 时复位，不打扰 files/term 等视图（保留网格进程边界约束）。
NO_PUSH: confirmed
```

## 根因（B9-4 · P1 空白）

`useBrowserStore.closeGridAll` 关闭宫格时设 `gridOpen=false`，但**不复位 `layout.mainView`**（仍为 `"grid"`）。连锁：

1. `isBrowserVisible = !gridOpen && mainView === "browser"` → `false`，`BrowserHost.vue` 把浏览器 webview 容器设为 `visibility:hidden`；
2. `useBrowserHost.schedulePosition` 在 `mainView !== "browser"` 时直接 `return`（L31），活动页签 webview 停在宫格离屏坐标；
3. `mainView` 仍是 `"grid"` 且 `gridOpen=false` → MainArea 的 grid 视图 DOM 不渲染 → 浏览器区整片空白，须手动切视图才恢复。

## 修复

`closeGridAll` 在清理网格后增加：

- `if (activeTabId.value) await bridge.tabActivate(activeTabId.value)` —— 重激活活动页签（后端聚焦）；
- `if (layout.mainView === "grid") layout.mainView = "browser"` —— 仅当确处 grid 视图时复位，避免打扰其它视图；
- `schedulePosition()` —— 兜底重定位活动页签 webview（gridOpen 已 false、mainView=browser 时真正下发 `tabPosition`）；`mainView` 变更 watch 下个 tick 再触发一次 `relocate`。

网格进程边界保留：仍走 `bridge.closeGrid()`，未自行重建/另起网格进程。

## 测试

`scripts/check-grid-close-logic.mjs`（source-level，headless，与 `check-home-ui-logic.mjs` 同范式：pinia store 含 pinia/bridge 依赖无法 node 直载，改静态解析）：

- G1：closeGridAll 7 个修复点齐全（复位 / grid 守卫 / tabActivate / schedulePosition / closeGrid 边界 / gridOpen 翻位 / gridRects 清空）；
- G2：isBrowserVisible 公式正确 + 非空白契约仿真（修复后=true，B9-4 复现=false 回归可检测）；
- G3：schedulePosition 依赖 mainView==="browser"（复位必要且充分）+ mainView watch 存在。

通过 12 / 失败 0。

## 作用域自检

- 仅改 `src/stores/useBrowserStore.ts` 的 `closeGridAll`；新增 `scripts/check-grid-close-logic.mjs`。
- 未触碰其它 lane 文件、未改 bridge/定位层/网格进程逻辑、未 push。
