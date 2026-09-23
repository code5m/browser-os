# WORKBENCH UI BOUNDARY — Phase UI-0E

> 定义 Workbench Shell **可以拥有**什么、**不应该拥有**什么。
> 原则：Shell 可以丰富、可以漂亮、可以拥有通用 UX；**但不得持有业务语义与业务资源生命周期**。

---

## 1. Workbench Shell 可以拥有 ✅

| 类别 | 当前实现 | 状态 |
|---|---|---|
| Window chrome | `UnifiedTabBar.vue`（自绘标题栏：拖拽/最小化/最大化/关闭） | ✅ 已有 |
| 通用布局 | `App.vue` 三段式（tabbar / activity / body + status） | ✅ 已有 |
| Navigation 容器 | `ActivityBar.vue` 一级入口 + ☰菜单 + 🔍命令 | ✅ 已有 |
| Toolbar 容器 | `ActivityBar` 扩展行机制 | ✅ 已有 |
| Main Surface 容器 | `MainArea.vue` + `workbench-main` 槽 | ✅ 已有 |
| Dock 容器 | `MainArea.vue` + `browser-dock` 槽 | ✅ **UI-4 已改为贡献驱动**（页签由 Contribution 声明 label/icon/order；absent capability → 页签消失，修复死页签） |
| ActivityBar / Rail 容器 | `WorkbenchRail.vue` | ✅ 已有 |
| Status Bar | `StatusBar.vue` | ✅ 已有 |
| Home framework | `HomePanel` + Launchers/Recents/Shortcuts | ✅ 已有 |
| Settings framework | `SettingsPanel.vue` | ✅ 已有 |
| Command / Search infrastructure | `WorkbenchCommands.vue`（Ctrl+K / Ctrl+Shift+P） | ✅ 已有 |
| Contribution hosts | `MainArea`（5 槽）+ `ActivityBar`（2 槽） | ✅ 已有 |
| 通用 dialog 基础设施 | `utils/modalA11y.ts` + `composables/useModalFocus.ts` | ✅ 已有（a11y 层） |
| generic loading / error / empty | ❌ **缺失**（20+/15 处各写各的） | ➡️ Pilot 目标 |
| Theme | `useSettingsStore` 外观设置 | ✅ 已有 |
| 通用 UI 服务（toast / layout） | `useLayoutStore.showToast` / `sidebarOpen` | ✅ 已有（但被业务面板反向直写） |

---

## 2. Workbench Shell 不应该拥有 ❌

| 不应拥有 | 当前状况 | 判定 |
|---|---|---|
| Browser lifecycle | `App.vue:5` 持 `useBrowserStore`；`ActivityBar` 调 `gridMode/gridCount/rebuildGrid/layoutGrid` | ⚠️ **违反**（用 public store 操作宫格仪式） |
| Terminal PTY lifecycle | `App.vue:14` 持 `useTerminalStore`；`App.vue:88-92,144-153` 分支 `activateTerm`/`toggleBrowserDock("term")` | ⚠️ **违反** |
| Workspace file state | `App.vue:9,10,11` 持 `useArtifactStore`/`useRepoStore`/`useFileStore`；`App.vue:99` **硬编码中文目录名** `fs.startDirs.find(d => d.name.includes("成果工作区"))` | ⚠️ **违反**（业务硬编码进 Shell） |
| Bookmark domain state | Shell 不持 `useBookmarkStore` | ✅ 合规（走贡献槽） |
| Git repository state | `App.vue:12` 持 `useGitStore`；`App.vue:269` 挂 `GitWriteConfirmDialog` | ⚠️ **违反** |
| Database connection state | `MainArea.vue:204` 直渲 `DatabasePanel` | ⚠️ **违反** |
| Agent session state | `MainArea.vue:221` 直渲 `AgentManagerPanel` | ⚠️ **违反** |

---

## 3. 边界规则（写入门禁）

### W-01 Shell 不得 import 能力内部（`state/` `ui/` 私有路径）
**当前：0 违反** ✅

### W-02 Shell 不得直接渲染业务面板
**当前：11 违反**（`UI-INVENTORY.md §5.1`）→ 需显式 baseline，不得静默洗绿

### W-03 Shell 不得持有业务 store
**当前：7 违反**（`App.vue:5,8,9,10,11,12,14`）

### W-04 Shell 不得触发 capability-owned 重资源创建
**当前：0 违反** ✅（`buildGrid` / `spawnTerm` 双闸；framework 实测 grid-child=0，ptmx=0）

### W-05 Shell 不得硬编码业务常量
**当前：1 违反**（`App.vue:99` 硬编码 `"成果工作区"`）

### W-06 Shell 不得新增 `position: fixed` browser overlay
**当前：3 处既有基线**（`.ctx-menu` / `.home-modal-mask` / `.modal-mask`）→ 允许保留，禁止新增

---

## 4. 通用 UI 服务接口（解耦业务面板反向直写）

业务面板当前**反向直写** Shell store，是隐性耦合：

| 业务面板 | 反向写的 Shell state |
|---|---|
| `DatabasePanel.vue:81` | `layout.sidebarOpen = false` |
| `DatabasePanel.vue:60,71` | `layout.showToast` |
| `VaultPanel.vue:42` | `workbench.collapsed` |
| `CredentialList.vue:70/74/83/85` | `layout.showToast` |
| `AuditPanel.vue:12` | `layout.sidebarOpen = false` |
| `ResourcePanel.vue:26` | `layout.*` |

> **结论**：即使把面板下沉到能力，这些仍需 Shell 提供**显式 UI 服务接口**（`toast` / `layout`）而非直写 store。
> **本阶段不实现**，仅登记为后续批次需求（避免与 Pilot 混在一起）。

---

## 5. 目标新增 Capability 流程（§22 理想态 vs 现状）

**理想流程**
```
create capability → declare manifest → implement public contract
→ implement capability UI → register contributions
→ assembly includes capability → Workbench renders contribution
```

**现状差距**
| 步骤 | 是否需要改 Shell | 说明 |
|---|---|---|
| 注册 `workbench-main` surface | ❌ 不需要 | ✅ 已达理想态（workspace 7 个 main 面板证明） |
| 注册 `browser-dock` surface | ⚠️ **需要** | 必须改 `MainArea.vue:164-177` Tab 白名单 + `useLayoutStore.ts:151` 联合类型 |
| 注册 `browser-sidebar` / `browser-host` | ❌ 不需要 | ✅ 已达理想态 |
| 注册 `address-bar-actions` / `activity-bar-trailing` | ❌ 不需要 | ✅ 已达理想态 |
| 新增菜单项 | ⚠️ **需要** | `NAV_MENU_SECTIONS` 硬编码于 `useLayoutStore.ts:43-75` |
| 新增状态栏项 | ⚠️ **需要** | `StatusBar` 直接读 6 个 store |

> **已达理想态的扩展点：5 / 7**。剩余差距集中在 **Dock Tab 白名单**与**菜单/状态栏无贡献类型**。

---

## 6. §23 物理包评估：`shared/ui` → `packages/ui-system`？

| 判据 | 现状 | 结论 |
|---|---|---|
| imports 稳定 | ❌ `shared/` 仅 3 文件，其中 1 个（ConfirmModal）实为业务组件 | 否 |
| public contract | ❌ 未定义 `shared/ui` 契约 | 否 |
| build boundary | ❌ 无独立构建 | 否 |
| consumer boundary | ⚠️ `ImageGallery` 1 个消费者、`ImageLightbox` 全局单例 | 否 |

### **READY_FOR_UI_PACKAGE = NO**（诚实判定，不虚报）
理由：当前 `shared/` 目录**不构成一个真实的共享 UI 层**（仅 1 个真正的通用组件）。先完成 Pilot 提炼 + 边界门禁，再评估物理包。

---

## 7. 状态

- `WORKBENCH_BOUNDARY = CREATED`
- Shell 合规项：**5**（资源边界 0 违反、无能力内部 import）
- Shell 待迁移项：**12**（11 直渲 + 1 业务硬编码）
- 已达理想扩展点：**5 / 7**
- `READY_FOR_UI_PACKAGE = NO`
