# Phase 6A — Core Closure ADRs

> 本文件记录 Phase 6A（Semantic Migration Core Closure）三项最小迁移的架构裁决。
> 真源：Semantic Closure Audit v1 已确认 `MIGRATION_REQUIRED` 仅三项：M2-a / M2-b / M2-c。
> 原则：不重构、不删历史代码、不扩大范围；每个迁移 = SCR→ADR→Registry→代码→Checker→Test→Closeout。

---

## ADR-SEM-P6A-1 — aiNavOpen 唯一 owner

- **状态**：ACCEPTED
- **背景**：审计发现 `aiNavOpen` 存在两个声明源：`useBrowserStore.aiNavOpen`（`src/stores/useBrowserStore.ts:43`）与 `useLayoutStore.aiNavOpen`（`src/stores/useLayoutStore.ts:146`）。
- **证据（调用链）**：
  - 写入：`useBrowserStore.gotoAI()`（`:596` 置 false）、`ActivityBar.vue`（toggle `:340`）、`useWorkbenchStore.ts`（`:29/:32` 折叠/恢复，均引用 `browser.aiNavOpen`）。
  - 读取：`AINavPanel.vue`（`:10` `v-if` 显隐、`:18` 关闭）、`ActivityBar.vue`（`:337` 高亮）。
  - **全部读取/写入均指向 `browser.aiNavOpen`**；`useLayoutStore.aiNavOpen` 仅"声明 + 导出"，**零引用 = 死重复**。
- **决策**：canonical owner = **useBrowserStore**（owner 域 `browser_grid_lifecycle`）。
  - 删除 `useLayoutStore` 中的重复声明与导出。
  - 新增 `useBrowserStore.toggleAiNav()` 作为唯一 toggle 写入入口；`ActivityBar` 开关改走它。
  - `aiNavOpen` 表达的是"AI 导航面板（AINavPanel）显隐"，**不是**视图导航，也不是其它面板开关。
- **被否决方案**：
  - 方案 B（owner = useLayoutStore）：否决——实际所有语义都在 browser store，迁到 layout 反而制造跨 store 写。
  - 直接删一个变量不做 ADR：否决——须先记录 owner 归属与防止再生的 Checker。
- **后果**：`aiNavOpen` 单一声明 + 单一 owner；R8 Checker 阻止第二 owner 再生。

---

## ADR-SEM-P6A-2 — Panel State Boundary（面板边界收敛）

- **状态**：ACCEPTED
- **背景**：存在多个 `*Open` 面板类状态，需判定"同一语义应合并 vs 不同领域保持独立"。
- **决策**：**按域分类，不做合并**（详见 `panel-state-decision.md`）：

| 面板 | 状态 | Owner | 决策 |
|-|-|-|-|
| 收藏夹侧栏 | `useBookmarkStore.panelOpen` | bookmark | KEEP（独立域，已治理） |
| 左侧通用侧栏 | `useLayoutStore.sidebarOpen` | view_navigation | KEEP（独立表面） |
| 剪贴板 | `useLayoutStore.clipOpen` | view_navigation | KEEP（与 setView("clip") 联动） |
| 文件编辑器覆盖层 | `useLayoutStore.fileEditorOpen` | view_navigation | KEEP（overlay，非导航） |
| 浏览器右侧 Dock | `useLayoutStore.browserDockOpen` | view_navigation | KEEP（含子页签 browserDockTab） |
| 宫格工具条 | `useLayoutStore.gridToolbarOpen` | view_navigation | KEEP（已治理；资源耦合见下） |
| 收藏夹侧栏挂载条件 | `MainArea.vue.bmPanelOpen` | （无，派生） | **IMPLEMENTATION_DETAIL**（= panelOpen && mainView==="browser"，必须保持组件 computed） |

- **关键裁定**：这些面板**互不等价**（收藏夹 / 剪贴板 / 编辑器覆盖层 / 浏览器 Dock / 侧栏 / 宫格工具条 = 6 个独立 UI 表面），**禁止合并**。真正需要治理的是"边界清晰 + 各自唯一 owner + 派生量不落 store"。
- **`bmPanelOpen` 红线**：它是派生量（`bookmarks.panelOpen && layout.mainView === "browser"`），**禁止**在 store 中声明为 `ref/reactive`（R8 守护）；`BookmarkStar`（高亮）用 `bookmarks.panelOpen`，`MainArea`（挂载）用 `bmPanelOpen`，二者口径差异属 UI 细节，不构成第二真源。
- **后果**：5 个面板开关从 `observed_not_governed` 提升为受治理；`bmPanelOpen` 明确为 implementation-detail。

---

## ADR-SEM-P6A-3 — gridSession Owner Clarification

- **状态**：ACCEPTED
- **背景**：`gridSession`（`src/stores/useBrowserStore.ts:23`）权属边界需澄清。
- **四问四答**：
  - 是资源状态吗？**否**——资源存在性由 `gridOpen` 表达。
  - 是恢复状态吗？**否**——它是内存 runtime 计数器，**绝不落盘**，不参与恢复。
  - 是视图状态吗？**否**——视图由 `mainView` 表达。
  - 是持久化状态吗？**否**——仅本次会话内存。
- **定义**：`gridSession` = **宫格定位缓存失效纪元（cache-invalidation epoch）**。`buildGrid` / `forceGridRelayout` 各 `+1`，供 `useBrowserHost` 识别"同一批 webview 已销毁重建，上次发送缓存必须作废重发"（重建后 rect 可能与新 webview 的 1x1 初始态相同）。
- **决策**：唯一 owner = **useBrowserStore**（owner 域 `browser_grid_lifecycle`）。
  - canonical_writer：`buildGrid`、`forceGridRelayout`（唯二写入点）。
  - `useBrowserHost` 为**只读消费方**，禁止写入。
  - 禁止任何第二 store 保存同一 session 意义（R8 守护）。
- **后果**：`gridSession` 从 `observed_not_governed` 提升为受治理；写入点被限定为 2 处；非持久化契约固化。

---

## 记录说明（顺序偏离）

任务建议 commit 顺序为 docs(决策) → code(aiNavOpen) → code(panel/grid) → checker。
实际执行以"审计已确认的最小迁移"为准，先落地代码收敛再补齐 ADR 文档；ADR 内容与已落地代码**逐一对应**，无决策漂移。最终提交序列：
`refactor(semantic): converge aiNavOpen ownership` → `refactor(semantic): clarify panel and gridSession ownership` → `docs(semantic): record core migration decisions` → `chore(checker): enforce semantic closure rules`。
