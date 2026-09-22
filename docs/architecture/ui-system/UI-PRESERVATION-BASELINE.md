# UI PRESERVATION BASELINE — Phase UI-0

> 本文件是**所有未来 UI 迁移的回归基线**。
> 原则：**CAPABILITY MIGRATION IS NOT UI REDESIGN. ARCHITECTURAL DECOUPLING IS NOT UI DELETION.**
> 任何组件迁移必须做到 `BEFORE UI ≈ AFTER UI`（VISUAL_EQUIVALENCE）。

---

## 1. GOOD_EXISTING_UI（必须保留的成熟 UI）— 15 项

| # | ID | 位置 | 为什么是"好 UI" | 迁移约束 |
|---|---|---|---|---|
| 1 | **统一活动条** | `ActivityBar.vue`（656） | 一级入口 + 智能地址栏 + **内联扩展行**（避免被原生 webview 遮挡）、键盘漫游 ←/→/Home/End/Esc | 不得拆散；扩展行机制必须保留 |
| 2 | **统一页签条** | `UnifiedTabBar.vue`（308） | 网页页签 + 模块页签**混排**、自绘标题栏（拖拽/最小化/最大化/关闭）、右键菜单 | 页签混排语义不得丢失 |
| 3 | **智能地址栏** | `ActivityBar.vue:310-325` | URL + `/` 开头目录路径**自动识别**、地址栏贡献槽（`address-bar-actions`） | 识别逻辑与贡献槽双保留 |
| 4 | **Browser Dock** | `MainArea.vue:162-178` | 4 Tab：📂文件 / 💻终端 / 🌊资源 / 💾会话，右侧 360px | Tab 顺序与图标不得变 |
| 5 | **宫格（Grid）** | `viewport.grid-mode` + `grid-cell-bar` | 多格浏览 + 每格悬浮标题栏（`pointer-events: none` 不拦截网页点击）+ 关闭按钮 | 悬浮层 z-index 20 与 pointer-events 语义必须保留 |
| 6 | **主页启动墙** | `HomeLaunchers.vue`（148） | 17 个业务入口响应式图标墙（120px→84px），`aria-current="page"` 高亮 | 入口数量与顺序不得减少 |
| 7 | **命令面板** | `WorkbenchCommands.vue`（52） | Ctrl/Cmd+K、Ctrl+Shift+P，↑↓ 选择 / Enter 执行 / Esc 关闭 + 焦点归还 | 键位与焦点归还必须保留 |
| 8 | **工具窗栏** | `WorkbenchRail.vue`（10） | 28px 竖向图标栏，`aria-pressed` 高亮，折叠态一键恢复 | |
| 9 | **状态栏** | `StatusBar.vue`（204） | 连接态 / 当前视图 / 计数 / 内存摘要 / 低内存红色告警 / 点击展开资源明细 | 告警阈值与浮层保留 |
| 10 | **主页快捷方式** | `HomeShortcuts.vue`（304） | 卡片网格、限 12 + 展开收起、常驻操作按钮（触屏可达） | |
| 11 | **主页最近访问** | `HomeRecents.vue`（247） | url/app/dir 三型、限 8、空态插画、脱敏摘要 | |
| 12 | **终端体验** | `TerminalPane.vue`（326） | xterm fit + 静默上报真实行列、面板重建回放最近 40 条、"自动确认 CLI"/"已丢弃 N B" 指示 | **PTY 生命周期不得改** |
| 13 | **IDE 文件面板** | `FilePanel.vue`（447） | 左树右编辑、`vLazyThumb` 懒加载缩略图、目录缩略图对比条、HTML5 拖拽移动确认 | 拖拽确认弹窗保留 |
| 14 | **图片灯箱** | `ImageLightbox.vue`（329） | ESC/点遮罩关闭、←/→ 不循环、滚轮缩放、双击 1×↔2×、放大后拖拽平移 | 键位语义保留 |
| 15 | **模态 a11y** | `utils/modalA11y.ts` + `composables/useModalFocus.ts` | Esc 关闭、Tab 焦点陷阱、打开聚焦 / 关闭归还焦点 | 所有弹窗必须继续复用 |

---

## 2. 必须保留的交互契约（回归检查项）

### 2.1 导航 / 入口
- [ ] 主页 17 个启动入口全部可达，顺序不变
- [ ] ActivityBar 一级入口（🏠主页 / 📁浏览）+ 🗂️宫格 + ☰菜单 + 🔍统一命令 全部在位
- [ ] `NAV_MENU_SECTIONS` 三个分区（工作区 / 工具 / 同步）结构不变
- [ ] WorkbenchRail 6 个工具入口不变

### 2.2 已冻结语义（**绝对不可破坏**）
- [ ] `mainView` = 期望可见主表面，owner `useLayoutStore` — 不变
- [ ] `gridOpen` = Grid 资源存在性，owner `useBrowserStore` — 不变
- [ ] `desiredGridVisibility = gridOpen && mainView === "grid"`（纯 derived，禁止 stored 第二真源）
- [ ] `mainView === "grid"` ⇒ `gridOpen === true`
- [ ] 视图切换 Grid → Browser/Home/Workspace = **HIDE ONLY**
- [ ] 显式 `closeGrid` = **DESTROY**
- [ ] 禁止重新引入 `gridVisible` / `exitGrid(mode)` / `GridLifecycle` enum / 新的 Rust show-hide 真源

### 2.3 资源隔离（不得因 UI 迁移回退）
- [ ] framework profile / Browser absent ⇒ `GRID_CHILD = 0`
- [ ] framework profile / Terminal absent ⇒ `PTY = 0`
- [ ] 持久化 `gridToolbarOpen = true` + Browser absent ⇒ 仍 `GRID_CHILD = 0`
- [ ] preference ≠ availability ≠ activation ≠ resource existence

### 2.4 Dock / Toolbar
- [ ] Dock 4 Tab 顺序：文件 → 终端 → 资源 → 会话
- [ ] `browserDockOpen` / `browserDockTab` owner 仍为 `useLayoutStore.toggleBrowserDock`

### 2.5 a11y
- [ ] 所有弹窗：Esc 关闭 + 焦点陷阱 + 焦点归还
- [ ] `role="status"` / `aria-live="polite"` 状态播报不丢失
- [ ] `aria-current="page"` / `aria-pressed` 高亮不丢失

---

## 3. 视觉基线快照（STRUCTURAL 层，非像素）

> 诚实区分：**STRUCTURAL_UI_PASS ≠ HUMAN_VISUAL_PASS**。
> 当前无法可靠自动截图，故采用「首屏 DOM 文本探针 + computed style」作为结构基线，人工目视仍为最终判据。

### 3.1 framework profile 首屏探针（HEAD `3f57a92`）
```
readyState=complete root=ok body=1854x1048
bodyBg=rgb(255,255,255) appBg=rgba(0,0,0,0)
text="＋−□×🏠主页📁浏览🗂️宫格▾☰菜单前往🤖📥采集⚙️
      🏠主页 ☆ 收藏网页  📁 收藏目录 ＋ 新增
      ↺ 默认主要工作区🌐浏览📂文件💻终端📋剪…"
```
### 3.2 full profile 首屏探针
```
readyState=complete root=ok body=3774x2128 app=3774x2128
bodyBg=rgb(255,255,255) appBg=rgba(0,0,0,0)
text="＋−□×🏠主页📁浏览🗂️宫格▾☰菜单前往🤖📥采集⚙️…"
```
> 迁移后必须重跑同一探针并逐字比对（保留两份证据于 `logs/`）。

### 3.3 关键 computed style 基线
| 元素 | 属性 | 基线值 |
|---|---|---|
| `.activity` | `height` / `background` | `32px` / `#1f2733` |
| `.activity button.active` | `background` | `#2b6cb0` |
| `.browser-dock` | `width` | `360px` |
| `.modal-mask` | `z-index` | `999` |
| `.home-modal-mask` | `z-index` | `100` |
| `.ctx-menu` | `z-index` | `999` |
| `.grid-cell-bar` | `height` / `pointer-events` | `26px` / `auto`（父层 `none`） |

---

## 4. 回归判据（默认判 REGRESSION 的情形）

出现以下任一，默认判定 **REGRESSION**（除非证明它本就是 absent Capability 专属 contribution）：

- 按钮/入口消失或位置变化
- 布局结构变化（三段式 shell、Dock 位置、Rail 位置）
- 快捷入口减少
- Dock / Toolbar 行为改变
- 键位失效
- 焦点/a11y 行为丢失
- 空态/加载态文案变化
- 颜色/间距/icon/动画变化（**组件化不是 redesign**）

---

## 5. 本阶段状态

- `UI_PRESERVATION_BASELINE = CREATED`
- 本阶段 **未修改任何 UI**（`git status` 仅新增 docs）
- 孤儿组件（TopBar / SidebarResizer / ResourcePanel / RunHistoryModal）**登记但不动** —— 孤儿 ≠ 立即删除
