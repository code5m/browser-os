# Phase 6A-2 — Panel State Decision

> 目标：判定多个 panel 类状态是"同一语义应合并"还是"不同领域保持独立"，并记录每项的归属与 Registry 状态。
> 结论：**全部保持独立（不合并）**；唯一需澄清的是 `bmPanelOpen` 属派生量（implementation-detail），禁止落 store。

## 决策表

| Panel | Meaning（表达什么） | State 表达式 | Owner | Registry | Decision |
|-|-|-|-|-|-|
| 收藏夹侧栏 | 收藏夹栏是否展开 | `useBookmarkStore.panelOpen` | bookmark | GOVERNED（原 Phase 3） | KEEP / INDEPENDENT |
| 收藏夹挂载条件 | 收藏夹栏是否**挂载**（仅浏览器视图） | `MainArea.vue` `bmPanelOpen = computed(panelOpen && mainView==="browser")` | 无（派生） | 不入 Registry | **IMPLEMENTATION_DETAIL**（必须保持 computed） |
| 左侧通用侧栏 | 通用左栏（文件树等）开合 | `useLayoutStore.sidebarOpen` | view_navigation | GOVERNED（Phase 6A 提升） | KEEP / INDEPENDENT |
| 剪贴板面板 | 剪贴板面板开合（联动 clip 视图） | `useLayoutStore.clipOpen` | view_navigation | GOVERNED（Phase 6A 提升） | KEEP / INDEPENDENT |
| 文件编辑器覆盖层 | Markdown/文件覆盖编辑器开合 | `useLayoutStore.fileEditorOpen` | view_navigation | GOVERNED（Phase 6A 提升） | KEEP / INDEPENDENT |
| 浏览器右侧 Dock | 浏览时右侧 Dock 开合 | `useLayoutStore.browserDockOpen` | view_navigation | GOVERNED（Phase 6A 提升） | KEEP / INDEPENDENT |
| Dock 子页签 | Dock 内 files/term/net/session | `useLayoutStore.browserDockTab` | view_navigation | GOVERNED（Phase 6A 提升） | KEEP / INDEPENDENT（Dock 内维度，非新面板） |
| 宫格工具条 | 宫格工具条开合 | `useLayoutStore.gridToolbarOpen` | view_navigation | GOVERNED（原 Phase 1） | KEEP / INDEPENDENT（与 browser.gridOpen 有资源耦合，但资源归 useBrowserStore） |

## 为什么不合并

各 panel 表达的是**不同 UI 表面**，不存在"同一个 Panel 两个状态"的重叠：

- 收藏夹（bookmark 域）≠ 剪贴板（系统域）≠ 文件编辑器覆盖层 ≠ 浏览器 Dock ≠ 通用侧栏 ≠ 宫格工具条。
- 任意两项合并都会制造"打开 A 面板却关了 B 面板"的耦合，属**过度治理**。

## 边界红线（由 Checker 守护）

1. **单一声明**：每个受治理面板开关只允许在 owner store 中声明一次（R8：`SEMANTIC_STATE_MULTI_OWNER`）。
2. **派生不落 store**：`bmPanelOpen` 禁止在 store 中被声明为 `ref/reactive`（R8：`SEMANTIC_DERIVED_PANEL_STORED`）。
3. **禁止为已有面板新建第二开关**：若将来某面板需要新状态，应走 SCR + 本 Registry 更新，而非就地新增 `xxxOpen`。

## 与视图导航的关系

- 面板开关（overlay / dock / toolbar）= "**内容表面**是否可见"。
- 视图导航 `mainView` = "**主表面**显示哪个视图"。
- 两者正交：`clipOpen` 打开时会 `setView("clip")`，但 `mainView` 仍是唯一导航真源（owner: useLayoutStore.view_navigation）。
