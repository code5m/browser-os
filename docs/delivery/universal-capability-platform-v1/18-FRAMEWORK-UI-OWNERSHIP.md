# 18 · FRAMEWORK UI OWNERSHIP（framework profile 启动后可见 UI 归属清单）

> 本清单回答：**framework profile（零能力注册）启动后，界面上每一个可见元素归谁所有？**
>
> 方法：逐个读组件源码与渲染条件，**不按文件名猜**。
> 分类标记：`FRAMEWORK_CORE_UI` / `CAPABILITY_CONTRIBUTION` / `LEGACY_SHELL_COUPLING` / `UNKNOWN`
>
> 关键声明：
> **`LEGACY_SHELL_COUPLING` 不等于 DELETE。**
> 它等于 `PRESERVE UX + FUTURE ARCHITECTURAL MIGRATION`。
> 本阶段最高原则是 `CAPABILITY MIGRATION IS NOT UI REDESIGN`。

---

## 0. 汇总

| 分类 | 数量 | 处置 |
|---|---|---|
| FRAMEWORK_CORE_UI | 34 | 保留（Shell 可以丰富、可以漂亮） |
| CAPABILITY_CONTRIBUTION | 15 | 由能力注册，absent 时槽为空 |
| LEGACY_SHELL_COUPLING | 13 | **保留 UX**，后续逐项迁移 ownership |
| UNKNOWN | 0 | — |

---

## 1. App.vue 外壳（FRAMEWORK_CORE_UI）

| # | 元素 | 位置 | 渲染条件 | 分类 |
|---|---|---|---|---|
| 1 | 启动遮罩「正在启动…」 | App.vue L247-250 | `!ready && !shellError` | FRAMEWORK_CORE_UI |
| 2 | 外壳错误兜底 | App.vue L252-255 | `shellError` | FRAMEWORK_CORE_UI |
| 3 | UnifiedTabBar 统一页签条 | App.vue L258 | `!compactMode` | FRAMEWORK_CORE_UI |
| 4 | ActivityBar 活动栏 | App.vue L259 | `!compactMode` | FRAMEWORK_CORE_UI |
| 5 | WorkbenchCommands 命令面板 | App.vue L260 | 常驻（内部 `commandOpen`） | FRAMEWORK_CORE_UI |
| 6 | WorkbenchRail 左侧工具窗竖条 | App.vue L262 | `!compactMode` | FRAMEWORK_CORE_UI |
| 7 | MainArea 主表面容器 | App.vue L264 | 常驻 | FRAMEWORK_CORE_UI |
| 8 | StatusBar 状态栏 | App.vue L266 | 常驻 | FRAMEWORK_CORE_UI |
| 9 | ConfirmModal 通用确认弹窗 | App.vue L267 | 常驻（内部 `rp.preview`） | FRAMEWORK_CORE_UI |
| 10 | ImageLightbox 图片预览 | App.vue L273 | 常驻（内部 `preview.open`） | FRAMEWORK_CORE_UI |
| 11 | GitWriteConfirmDialog | App.vue L269 | 常驻（内部 `pv`） | FRAMEWORK_CORE_UI（遗留组件，非能力） |

> 死代码（不渲染，不计入）：`components/layout/TopBar.vue`、`SidebarResizer.vue` —— 全仓无 import。
> `layout.sidebarOpen` 存在但无任何组件按其渲染侧栏。

---

## 2. Navigation / ActivityBar

### 2.1 FRAMEWORK_CORE_UI

| # | 元素 | 位置 | 分类 |
|---|---|---|---|
| 12 | 🏠 主页 一级入口 | ActivityBar L247-259（store TOP_NAV_ITEMS） | FRAMEWORK_CORE_UI |
| 13 | ☰ 菜单按钮 + 功能菜单扩展行 | ActivityBar L285-297 / L429-445 | FRAMEWORK_CORE_UI |
| 14 | 🔍 统一命令（Search） | ActivityBar L299 | FRAMEWORK_CORE_UI |
| 15 | 折叠/恢复工具窗按钮 | ActivityBar L300 | FRAMEWORK_CORE_UI |
| 16 | 智能地址栏（omnibox）主体 | ActivityBar L310-319 | FRAMEWORK_CORE_UI |
| 17 | 「前往」按钮 | ActivityBar L325 | FRAMEWORK_CORE_UI |
| 18 | ⚙️ 系统设置入口 | ActivityBar L353-355 | FRAMEWORK_CORE_UI |
| 19 | 最近/常用行（omni 扩展行） | ActivityBar L448-474 | FRAMEWORK_CORE_UI |
| 20 | WorkbenchRail 5 按钮（文件/数据库/Vault/Git/终端） | WorkbenchRail L7-9 | FRAMEWORK_CORE_UI |
| 21 | ☰ 菜单「工作区」4 项（文件/笔记/剪贴板/知识库） | ActivityBar L430-433 | FRAMEWORK_CORE_UI |
| 22 | ☰ 菜单「工具」11 项 | ActivityBar L434-442 | FRAMEWORK_CORE_UI |
| 23 | ☰ 菜单「同步」2 项（仓库/审计） | ActivityBar L443 | FRAMEWORK_CORE_UI |

> 说明：☰ 菜单 17 个视图入口是 **Shell 的通用导航容器**能力。即便某个视图的具体内容由
> Workspace 能力贡献（见 §4），**导航入口本身归 Shell** —— 不可删除（删了等于毁导航体验）。

### 2.2 LEGACY_SHELL_COUPLING（保留 UX，后续迁 ownership）

| # | 元素 | 位置 | 为什么算 coupling |
|---|---|---|---|
| 24 | 🗂️ 宫格一级入口 | ActivityBar L262-271 | Shell 直接调 `browser.activateGrid()` |
| 25 | 宫格设置行（模式/布局/格数/网址/资源/关闭） | ActivityBar L375-404 | 读写 `browser.gridCount/gridLayout/gridUrls` |
| 26 | AI 群发输入行 | ActivityBar L362-371 | 依赖 `browser.gridOpen/gridMode/gridAiInput` |
| 27 | 各格网址行 | ActivityBar L420-426 | 依赖宫格资源状态 |
| 28 | 资源监控行（每格 RSS） | ActivityBar L406-419 | Browser 专属遥测 |
| 29 | ← → ⟳ 浏览器前进/后退/刷新 | ActivityBar L304-306 | `browser.goBack/goForward/reloadActive` |
| 30 | 🗂 / 💻 浏览器 Dock 开关 | ActivityBar L334-335 | Shell 持有 `browserDockOpen` 这一 Browser 语义状态 |
| 31 | ⛶ 精简模式 | ActivityBar L336 | Browser 视图专属 |
| 32 | 🤖 AI 导航按钮 | ActivityBar L339-348 | `browser.toggleAiNav()` |
| 33 | AINavPanel（AI 站点栏） | App.vue L263 | `browser.aiNavOpen` |
| 34 | 📥 采集 | ActivityBar L349-352 | Artifact 采集（Workspace 语义，Shell 直连） |

### 2.3 CAPABILITY_CONTRIBUTION

| # | 元素 | 槽 | 注册者 |
|---|---|---|---|
| 35 | 📑 收藏夹入口按钮 | `activity-bar-trailing` | Bookmark（`bookmark/index.ts` L51-57） |
| 36 | ☆ 收藏星标 | `address-bar-actions` | Bookmark（`bookmark/index.ts` L44-50） |

---

## 3. Main surfaces（MainArea）

| # | 元素 | 位置 | 分类 |
|---|---|---|---|
| 37 | HomePanel（主页） | MainArea L136-138 | FRAMEWORK_CORE_UI |
| 38 | HomeLaunchers 17 张启动卡 | HomeLaunchers L22-40 | FRAMEWORK_CORE_UI |
| 39 | HomeShortcuts / HomeRecents / HomeShortcutEditor | HomePanel L47-49 | FRAMEWORK_CORE_UI |
| 40 | ClipboardPanel（剪贴板） | MainArea L188-190 | FRAMEWORK_CORE_UI |
| 41 | AppPanel（应用） | MainArea L193-195 | FRAMEWORK_CORE_UI |
| 42 | ToolBox（工具箱） | MainArea L198-200 | FRAMEWORK_CORE_UI |
| 43 | DatabasePanel | MainArea L203-205 | FRAMEWORK_CORE_UI（未能力化，Debt） |
| 44 | VaultPanel | MainArea L207 | FRAMEWORK_CORE_UI（未能力化） |
| 45 | TaskPanel | MainArea L209-211 | FRAMEWORK_CORE_UI（未能力化） |
| 46 | PluginManager | MainArea L214-216 | FRAMEWORK_CORE_UI（未能力化） |
| 47 | SkillManagerPanel / AgentManagerPanel | MainArea L217-222 | FRAMEWORK_CORE_UI（未能力化） |
| 48 | GraphPanel | MainArea L223-225 | FRAMEWORK_CORE_UI（未能力化） |
| 49 | SettingsPanel | MainArea L228-230 | FRAMEWORK_CORE_UI |
| 50 | 「当前视图不可用」兜底面板 | MainArea L243-257 | FRAMEWORK_CORE_UI |
| 51 | BrowserHost | `browser-host` 槽 | **CAPABILITY_CONTRIBUTION**（Browser） |
| 52 | FilePanel | `browser-dock@files` | **CAPABILITY_CONTRIBUTION**（Workspace） |
| 53 | TerminalDockPanel | `browser-dock@term` | **CAPABILITY_CONTRIBUTION**（Terminal） |
| 54 | ResourceWaterfall | `browser-dock@net` | **CAPABILITY_CONTRIBUTION**（Browser） |
| 55 | SessionPanel | `browser-dock@session` | **CAPABILITY_CONTRIBUTION**（Browser） |
| 56 | TerminalView | `workbench-main-resident@term` | **CAPABILITY_CONTRIBUTION**（Terminal） |
| 57 | BookmarkPanel | `browser-sidebar` | **CAPABILITY_CONTRIBUTION**（Bookmark） |
| 58 | FileIdeView | `workbench-main@files` | **CAPABILITY_CONTRIBUTION**（Workspace） |
| 59 | ArtifactPanel | `workbench-main@arts` | **CAPABILITY_CONTRIBUTION**（Workspace） |
| 60 | RepoPanel | `workbench-main@repo` | **CAPABILITY_CONTRIBUTION**（Workspace） |
| 61 | ScriptPanel | `workbench-main@scripts` | **CAPABILITY_CONTRIBUTION**（Workspace） |
| 62 | CommandSnippetPanel | `workbench-main@commands` | **CAPABILITY_CONTRIBUTION**（Workspace） |
| 63 | AuditPanel | `workbench-main@audit` | **CAPABILITY_CONTRIBUTION**（Workspace） |
| 64 | FileEditor | `workbench-main@editor` | **CAPABILITY_CONTRIBUTION**（Workspace） |

---

## 4. Dock / Toolbar / Status

| # | 元素 | 位置 | 分类 |
|---|---|---|---|
| 65 | 浏览器右侧 Dock 容器 + 4 个 tab | MainArea L162-178 | LEGACY_SHELL_COUPLING（容器在 Shell，内容多由能力贡献） |
| 66 | GridArchiveBar 归档工具条 | ActivityBar L373 | LEGACY_SHELL_COUPLING（Browser/Grid 专属） |
| 67 | 终端工具条（单/2/4/9 布局、＋终端） | TerminalView L37-54 | CAPABILITY_CONTRIBUTION（Terminal） |
| 68 | StatusBar 当前视图名 / 页签数 | StatusBar L85-87 | FRAMEWORK_CORE_UI |
| 69 | StatusBar「终端 就绪/未启」「仓库 已配置」「审计 N 条」 | StatusBar L88-90 | FRAMEWORK_CORE_UI（读取能力状态展示） |
| 70 | StatusBar 资源摘要（内存/每宫格 RSS/预算） | StatusBar L92-111 | LEGACY_SHELL_COUPLING（宫格专属遥测） |
| 71 | StatusBar Toast | StatusBar L112 | FRAMEWORK_CORE_UI |

---

## 5. Settings / Home 明细

| # | 元素 | 位置 | 分类 |
|---|---|---|---|
| 72 | 外观（主题） | SettingsPanel L49-58 | FRAMEWORK_CORE_UI |
| 73 | 快捷键方案（VSCode/IDEA/Eclipse）+ 预览表 | SettingsPanel L60-76 | FRAMEWORK_CORE_UI |
| 74 | 性能（页签休眠 checkbox） | SettingsPanel L78-92 | FRAMEWORK_CORE_UI |
| 75 | 默认浏览器（设为默认，二次确认） | SettingsPanel L94-119 | FRAMEWORK_CORE_UI |
| 76 | 关于（版本 v0.9.10） | SettingsPanel L121-127 | FRAMEWORK_CORE_UI |
| 77 | Home 头部 ☆收藏网页 / 📁收藏目录 / ＋新增 / ↺默认 | HomePanel L27-34 | FRAMEWORK_CORE_UI |

---

## 6. 结论

- **UNKNOWN = 0。**
- 本阶段 **UI 删除数 = 0**。第 2.2 / 4 节的 13 项 `LEGACY_SHELL_COUPLING` 全部**原样保留**，
  仅其中 1 项（宫格资源创建）的**资源副作用**被能力闸截断，UX 不变。
- framework profile 下 CAPABILITY_CONTRIBUTION 的 15 项因槽为空而不渲染 —— 这是
  **正确的 absent 语义**，不是 UI 被删。
