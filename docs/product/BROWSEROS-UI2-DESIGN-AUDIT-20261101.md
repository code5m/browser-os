# BrowserOS UI 2.0 · 产品设计对比与审核报告

> 审核日期：2026-11-01
>
> 审核分支：`browseros-ui2-aplus-20261101`（仅设计/文档）
>
> 真实产品基线：用户提供的 26 张 1920×1080 实机截图；当前设计归档 `docs/product/browseros-current-version-20261101/`
>
> 代码核对：`origin/master@c90eb564`（相对截图设计基点 `8a5e8380`，增加 Agent Runtime Lab 和 CI 可选原生构建）
>
> 旧 UI2：`origin/design/browseros-ui2-aplus-20261101` 的 `docs/prototypes/browseros-ui2/`。
>
> 新 A+：本分支 `docs/prototypes/browseros-ui2-aplus-20261101/index.html`。
>
> 证据等级：**[实机截图]**、**[代码]**、**[Chrome 实测]**、**[评审建议]**。所有“产品实现”判断都不以 HTML Mock 冒充。

## 0. 审核结论（Verdict）

**当前 A+ 单文件原型：不建议作为开发基线直接移植，产品交互验收 NOT READY。**

- 视觉排版优于当前实机许多内部浅色页，但更像综合管理后台，Chrome 浏览器壳层退化。
- 上一版 UI2 的路由覆盖、真实标签交互、地址建议、键盘、Grid 演示和状态机 Mock 更完整；A+ 不应替换这些契约。
- 新 A+ 的独立 `Agent` 页面不可经导航到达，`Grid` 独立页面缺失，浏览页不存在；许多功能被折叠到工具中心，未给出完整操作链。
- 已通过 Chrome 证实标签关闭、`Ctrl+L`、定时任务抽屉初次开合等问题。
- **安全阻断：`code5m/browser-os` 当前为 public；已提交的截图包含可识别业务/人员字段及内部连接信息，必须开展脱敏/历史清理审批。**

推荐设计决议：**真实 Chrome Shell + 旧 UI2 的可用交互模型 + 新 A+ 的轻量视觉 Token + 当前生产 Capability 的专业结构**。保持设计分支隔离，不 cherry-pick 旧 UI2 实现代码；仅在新设计文档中吸收原则。生产开发另建 feature 分支。

## 1. 已执行的审核（可复核）

| 方法 | 对象 | 结果 |
|---|---|---|
| 文件完整性 | AS-IS HTML → `screenshots/` | 26 个引用、26 个唯一原始 PNG、0 个缺失 |
| 静态页面解析 | A+ `section.page` | **12 个独立页面**，并非 14 个独立页面 |
| 静态交互扫描 | A+ HTML | 初始静态 57 个 button 元素，27 个 inline onclick（含非按钮）；46 个 button 无自身 inline 点击处理（注意部分可能由外层/其他事件接管） |
| Chrome 1440×900 | 旧 UI2 / 新 A+ | 两版均可无头渲染 |
| Chrome 1024×900 | 新 A+ | 首页主要 Hero 变窄但未完全溢出 |
| Chrome 760×900 | 新 A+ | 主内容被固定侧栏 + Hero 双列挤压；首页 Hero 卡片宽 **117px**，标题宽 **59px** |
| Chrome DOM/交互 | A+ 标签关闭 | 关闭标记 `×` 的按钮被当作导航，`openTabs` 不减少 |
| Chrome DOM/交互 | A+ `Ctrl+L` | 地址栏未获取焦点 |
| Chrome DOM/交互 | A+ Agent 导航 | `go('agent')` 被强制转到 `studio`，独立 `#agent` 页面不可达 |
| Chrome DOM/交互 | A+ 定时任务 | 表单初始可见、开关默认 on；点击“新建”后首次反而隐藏 |
| Chrome DOM/交互 | A+ 数据表达 | 首页静态 CPU/内存/磁盘以及任务 24、运行 5、成功 1982、失败 16；**仅 Mock 数据** |
| Chrome DOM/交互 | 旧 UI2 `Ctrl+L` | 地址栏成功聚焦 |
| 代码结构 | A+ vs 旧 UI2 | A+ 单文件 41,978 bytes；旧方案独立 `index.html`/`styles.css`/`app.js`（明确 Demo 状态） |
| 生产边界 | `AGENTS.md` / `docs/architecture/ui-system/ui-components.yaml` | GTK 子 WebView 不能用 DOM z-index 覆盖；Capability Public API / 独立资源 Owner 必须保持 |

以上测试只验证设计原型和当前代码审查；没有运行 BrowserOS 真实安装版 GUI，也没有测试实际 SQL/Git/Agent/PTY 副作用、性能基线或跨平台表现。

## 2. 综合评分：纠正此前未验证的 9.2 分

评分仅针对**原型设计可交付成熟度**，不等于真实产品性能或用户满意度。各维度 0～10 分，为评审判断，下面的通过/失败事实独立于评分。

| 维度 | 权重 | 旧 UI2 | 新 A+ | 评语 |
|---|---:|---:|---:|---|
| Chrome 壳层完整度 | 20% | 8.5 | 5.0 | 新版缺收藏栏、真实标签关闭与键盘 |
| 模块与场景覆盖 | 20% | 8.5 | 6.0 | 旧版 17 条可导航路由；新版 12 页，关键入口合并/遗漏 |
| 交互完成度 | 25% | 7.5 | 3.0 | 新版大量静态控件、假反馈、状态流缺失 |
| 视觉与层次 | 15% | 8.0 | 8.5 | 新版排版和大屏美感占优，仍存在 Emoji 与过度卡片化 |
| 缩放与窗口适应 | 10% | 6.0 | 3.0 | 新版 760px 明显挤压；旧版也需完整实机验收 |
| 可访问性设计 | 5% | 6.0 | 3.0 | 旧版有 role/aria/快捷键；新有大量 div onclick |
| 真实状态与安全表达 | 5% | 7.0 | 2.0 | 新版伪状态未显式标示 Demo，任务默认启用风险 |
| **加权评审分** | **100%** | **77.3 / 100** | **47.8 / 100** | **A+ 是目标名，不代表已达到 A+ 质量门槛** |

**当前实机产品不纳入同一原型评分**：它具备真实功能，而设计原型不连接后端；实机截图可说明视觉/结构，但不能证明完整可访问性、响应时延、操作成功率等。**AS-IS 设计板亦不属于可操作 UI**，只评价其文档/可追溯性（26/26 图片有效）。

## 3. 全模块覆盖：当前产品 vs 旧 UI2 vs 新 A+

说明：完整=提供对应独立可导航视图，部分=仅静态、合并或弱化，缺失=入口或完整流程未提供。

| 模块 / 场景 | 当前实机（截图） | 旧 UI2 原型 | 新 A+ 原型 | 审核裁定 |
|---|---|---|---|---|
| Browser Shell + Tab | 完整 | 独立壳层与关闭/历史 Mock | 外形在，关键行为不完整 | P0 |
| 收藏栏 / 书签 | 完整 | 完整 Mock | 缺失 | P0 |
| 浏览器独立页 | 完整 | `browser` 路由 | “浏览”卡片返回 `home` | P0 |
| Home / 最近访问 | 完整 | 独立页 | 独立页，改为 Hero + 项目/活动 | 保留新版层级但删除假指标 |
| Files / 右键菜单 | 完整 | 可导航 Mock | 独立页，右键能力不完整 | P1 |
| Coding / Project | 完整 | 独立 Studio | 独立 Studio，仅展示代码 | P1 |
| Agent Center | 有项目会话与控制 | 独立 `agents` 路由 | `agent` 页面存在但不可达 | P0 |
| Grid / 2/4 格 | 当前空态截图 | 1/2/4 格 Mock | **独立视图缺失** | P0 |
| Database Studio | SQL / 表数据 / 结果三态 | SQL、模拟结果、事务信息 | 高密度静态页面 | P1 |
| Git / Diff | Log / Diff 两态 | History/Changes/Diff Mock | 静态 Git 页 | P1 |
| Terminal | 独立页、1/2/4/9 | 独立终端演示 | 合并到 Devtools，缺独立工作流 | P1 |
| Clipboard | 独立页 | 独立页 Mock | 合并 Devtools | P1 |
| Scripts | 独立页 | 独立页 Mock | 合并 Devtools | P1 |
| Skills | 蓝图/步骤/检查/权限 | 多状态 Mock | 基础表单、静态标签 | P1 |
| Scheduled Tasks | 空态 + 创建弹窗 | 动态创建 Mock，默认不执行 | 默认展示启用，抽屉开合异常 | P0 |
| Knowledge Graph | 0 节点空态 | 节点/Inspector 演示 | 0 节点空态拷贝 | P1 |
| Knowledge Center | 主页入口 | 独立页 | 合并知识/内容中心 | P1 |
| Local web apps / BFC | 浏览器网页，独立自身主题 | 浏览器 Shell 可容纳 | 知识中心模拟内容，混淆原生与外部网页 | P1 |
| Settings | 长列表 + AI 模型与性能 | 可交互分类 Mock | 左侧分类视觉，交互不足 | P1 |
| Agent Runtime Lab | 最新 master 有代码，截图未覆盖 | 旧分支基点未包含 | 未覆盖 | P1 |
| 审计 / 仓库 / 插件 / 命令库 | 主页入口可见 | 工具入口 | 明确入口与细化页缺失 | P2（可发现性） |

## 4. P0 阻断级缺陷（先修，再谈 A+）

### P0-01 公开截图与数据治理风险
- 证据：`gh api repos/code5m/browser-os` 返回 `private:false`、`visibility:public`。
- 26 张已公开 PNG 中部分包含人员数据字段、可识别个人信息和内部连接相关细节。
- 风险：截图直接可访问，普通新提交删除不能清理历史。
- 建议：立即评估将仓库转为 private 或限制传播；形成脱敏图集与映射；经用户审批后清理 Git 历史和缓存、检查 fork/克隆可见性；不要在报告中复写任何敏感值。
- 目前未执行权限修改或历史重写。

### P0-02 不能牺牲 Chrome 浏览器核心契约
- A+ `#tabs` 中关闭符号 `×` 只是按钮文本，点击执行 `go(route)`，不是 close。
- 浏览器返回、前进、刷新按钮没有事件。
- A+ 没有收藏栏视图，也没有独立浏览器路由。
- `Ctrl+L` 实测 FAIL，`Ctrl+K` 未见键盘监听；命令面板使用原生 `prompt()`，不是产品级交互。
- 目标：保留已存在的 `UnifiedTabBar`、`ActivityBar`、`BookmarkBar`、`BookmarkSuggestions` 和 native 事件适配；标准 Tab 关闭不触发无关资源销毁。

### P0-03 路由与入口不一致
- A+ 只有 12 个 `section.page`。
- `go('agent')` 被硬编码成 `studio`，使独立 `#agent` 永远不可达。
- 新 Tab 的 `＋` 只执行 `go('home')`，不会创建新的逻辑 Tab。
- 主页“浏览”模块也指向 `home`；Grid 无页面。
- 目标：建立路由对照表（真实 `browseros://` 路由、页面 Owner、入口名称），所有可见入口必须可达，导航和资源生命周期分离。

### P0-04 定时任务默认安全策略被设计反转
- 真实实机截图 13/14：新建任务“启用”默认关闭，确认后才运行。
- A+ 原型：`#taskDrawer` 初始就显示，内部开关带 `.switch.on`；首次按“新建任务”反而隐藏表单。
- `toast('任务已创建（原型）')` 不创建验证过的对象或展示已保存状态。
- 目标：默认 OFF、明确五段 Cron 与验证、执行范围/重试/补跑/超时可见、关闭与取消语义清楚；提交之后更新列表状态。危险权限未确认禁止实际启用。

### P0-05 真实能力指标与演示数字混用
- A+ 首页显示 CPU=12%、内存=4.3G、磁盘=124G、连接服务成功等；任务列表显示 24、5、1982、16。
- 这些值与用户实机截图不一致，页面内没有显式标为 Demo。
- 目标：原型所有数据明确 `DEMO` 标记；真实产品从唯一状态源显示，数据不可用显示“未知/未连接”，不能伪装成功。

### P0-06 生产落地不得用纯 DOM 冒充原生视图
- `AGENTS.md` 已明确 Linux GTK 子 WebView 不是普通 DOM；不能用 z-index 覆盖 native 子视图，也不能随意改变隐藏、恢复和销毁时序。
- 真正生产实现必须通过 Capability Public API / Existing Host 替换视觉层；HTML `div` Grid 只能当布局示意。
- 目标：编写 Owner + 生命周期迁移矩阵，原生坐标同步和窗口缩放单独验收。

## 5. P1 页面与交互细节（按业务模块）

| 模块 | 应保留现有能力 | 新设计应调整的部分 | 验收 |
|---|---|---|---|
| Home | 主页收藏、快捷方式、最近访问、工作区入口 | 一级功能分组、继续工作优先、统一图标 | 17 个现有主模块入口可检索；收藏可编辑，不丢数据 |
| Files | 文件/目录 CRUD、右键 Git/Terminal/IDEA 入口 | 收敛工具条、提供显式“更多”，完整上下文菜单 | 鼠标和键盘可访问所有实际命令，路径不会误走搜索 |
| Coding Studio | Monaco、LSP 状态、文件树、工具输出、计划 | 统一工具栏、项目上下文、运行与审批反馈 | 脏文件、保存失败、审批等待、任务完成全状态 |
| Agent | 模型、Plan、会话恢复、工具调用 | 分清模型选择/任务/工具/审批/回复 | 只读与写入审批准则一致；任务退出后状态可恢复 |
| Database | SQL、Schema、表数据、Inspector、事务、导出 | 增强主次优先级、会话状态、结果区尺寸 | SQL 和表数据可切换；只读/写入/回滚状态明确 |
| Git | Changes、Branches、Log、Diff、Inspector | 统一 Diff、提交确认、操作可发现性 | Diff 双栏真实变更、暂存/提交/Push 受控 |
| Grid | 2/4 与 AI 组合、群发/提取/重试统计 | 空态 CTA、视图选择、分屏矩形一致 | 原生 WebView 面板不被 DOM 覆盖，缩放不重建 |
| Skills | 概览/步骤/检查/权限 + 导入/导出 | 分步骤表单，错误与完成度贯穿 | 草稿保留、检查状态同步，权限默认拒绝 |
| Graph | 搜索/过滤、导入 JSON、重建索引 | 区分未加载、空数据、权限/后端错误 | 空态提供原因与下一步；非空态可选节点 |
| Terminal | 独立 1/2/4/9 布局、PTY 会话 | 与工具 Dock 保持一致 Token | 隐藏面板不杀 PTY，复显不重复创建 |
| Clipboard / Scripts | 各自 CRUD 和历史 | 可在工具中心发现，但保留独立工作页 | “工具中心”入口不能替代操作页面 |
| Settings | AI 模型、主题、快捷键、性能、工程健康 | 左导航+右分组，减少单页长滚动 | 选项能进入对应分组；保存/失败反馈显式 |
| Runtime Lab | Native/Rig/Canary/Benchmark | 放在“实验与开发者设置”，警示默认关闭 | 生产默认不变，所有实验设置仅影响新任务 |

## 6. 响应式与布局：具体缺陷

- 参考宽：1920 实机、1440 设计、1024 窗口收缩、760 小窗口。
- 新 A+ `--sidebar:236px` 且 `.hero{grid-template-columns:1fr 330px}`，只针对 `max-width:1300px` 做局部媒体查询。
- Chrome 760 实测首页 Hero 宽 117px，标题宽 59px，导致首页核心 CTA 难以正常操作。
- 不能简单给整个容器 `overflow:auto` 就算解决；导航、Hero、操作栏应重排或隐藏/抽屉化。
- 建议断点：≥1440 全布局；1024～1439 保留主内容并隐藏非关键右栏；800～1023 收起侧栏；小于 800 至少保证单列主内容、可达导航和搜索。
- Database / Git / Studio 在窄窗口允许明确的水平滚动，但必须可感知，不能让关键写操作在视口外消失。

## 7. 无障碍 / 语义 / 性能审查

### 无障碍
- 新 A+ 导航使用 `div.navitem[onclick]`、主页模块使用 `div.module[onclick]`，缺少按钮语义和默认 Tab 焦点。
- A+ 多处以 Unicode/Emoji 替代标准语义图标，并缺少视觉焦点和 `aria-label`。
- 旧 UI2 包含 `role=tab`、`role=listbox`、`aria-expanded`、`aria-selected`、键盘事件，但也需要正式屏幕阅读器验收。
- 必须实现：`Tab/Shift+Tab` 全程可访问、`Esc` 退出弹层、`Enter/Space` 激活、焦点回归、支持中文输入法、`prefers-reduced-motion`，文本对比 WCAG AA。

### 性能与资源
- 不得让主题切换或面板复显重建 WebView/PTY/SQL Session/Agent Task。
- 所有非平凡显示统计必须来自实际 Resource Owner；演示时清楚标注 Demo。
- 不能以静态 HTML 的 60fps 或 UI 截图代替安装版冷启动、PSS/RSS、输入延迟和 Grid IPC 性能。

## 8. 对既有工程组件的映射（仅建议，不修改代码）

| UI 设计对象 | 现有生产组件 / 文件（核查来源） | Owner / 注意点 |
|---|---|---|
| Chrome Tabs | `src/components/layout/UnifiedTabBar.vue` | Shell，不能破坏原生 Tab 关闭语义 |
| 导航 / 地址栏 | `src/components/layout/ActivityBar.vue` | Shell，与 native 子 WebView 关系需验收 |
| 收藏栏 / 候选 | `src/capabilities/bookmark/ui/BookmarkBar.vue`、`BookmarkSuggestions.vue` | Bookmark capability |
| Home | `src/capabilities/home/ui/HomePanel.vue`、`HomeLaunchers.vue` | Home capability |
| Coding Agent | `src/capabilities/agent/ui/CodingWorkbench.vue`、`AgentChatPanel.vue` | Agent capability |
| Database | `src/capabilities/database/ui/DatabaseWorkspace.vue`、`SqlEditor.vue` | DB capability / native session |
| Git | `src/capabilities/git/ui/GitStudio.vue`、`GitHistoryDiff.vue` | Git capability |
| Grid | `src/capabilities/grid/ui/GridRows.vue`、`useGridHost.ts` | Grid capability / UDS IPC |
| Terminal | `src/capabilities/terminal/ui/TerminalView.vue`、`TerminalDockPanel.vue` | PTY resource owner |
| Settings | `src/components/system/SettingsPanel.vue` | 最新 master 已加 Runtime Lab |
| Workspace root | `src/App.vue`、`src/components/layout/MainArea.vue` | 不复制并行 Shell |

这些对象来源于 `docs/architecture/ui-system/ui-components.yaml` 与仓库真实文件；应在生产实施前复核最新代码行号和 Owner，不应从原型文件名反推架构。

## 9. 目标 UI 2.0 的正确产品结构

建议分三层，而不是把所有模块都塞进左侧纵向后台导航：

1. **Chrome Shell**：Tab、地址栏、收藏栏、工具菜单、原生视图承载。外部网页和内部页同层管理。
2. **Workbench Launcher**：Home/Browser/Files/Tools 四个稳定一等入口；常用能力可收藏、搜索、从快捷动作进入。
3. **Capability Workspace**：Agent、Studio、Git、Database、Grid、Terminal、Tasks、Skills、Graph 等保持专业布局与独立 Owner。

统一的是设计 Token、导航规则、状态反馈和权限，不是每一页必须拥有相同的侧边栏/卡片数量。

## 10. 交付验收清单与优先级

### P0 · 不通过不得作为 A+ 基线
- [ ] 公开仓库截图数据完成风险处置审批（脱敏、访问控制、历史清理方案）。
- [ ] 17 个既有模块入口逐一映射：每个入口可导航到真实功能页面或明确说明聚合入口。
- [ ] 独立 Browser、Grid、Agent 工作区可达。
- [ ] 标签关闭、回退/前进/刷新、收藏栏、Ctrl+L/T/W、地址建议真实可操作。
- [ ] 任务创建默认关闭，抽屉首次点击打开，危险操作有审批。
- [ ] 原型所有 Mock 数据在视图中显式标为 DEMO，禁止伪造系统“运行正常”。
- [ ] 不修改真实 GTK/WebView 生命周期和当前用户资料。

### P1 · 功能与交互准入
- [ ] Home 高频入口、快捷方式和最近访问层级清楚。
- [ ] Studio、Git、Database 恢复完整工具功能与异常态。
- [ ] Terminal 保留 1/2/4/9，Grid 保留真实产品支持的布局，不降低能力。
- [ ] Skills / Tasks / Settings 表单有真实前端状态验证与撤销/恢复。
- [ ] 窗口 1920、1440、1024、760 视觉验收；窄窗口主 CTA 不消失。
- [ ] 全局颜色、圆角、输入/按钮、表格与 Inspector 统一设计 Token。
- [ ] 键盘/屏幕阅读器/高 DPI/中文输入法/暗黑主题测试通过。

### P2 · 体验优化
- [ ] 空态与 Loading/Error/Permission/Offline/Recovery 状态一致。
- [ ] 轻动画遵守 reduced-motion，不牺牲编辑/滚动性能。
- [ ] 信息密度模式：浏览默认舒适，专业工具默认紧凑。
- [ ] 设计组件文档、页面状态表、真实截图回归清单完备。

## 11. 修正建议：如何处理两个设计分支

- 保留旧 UI2 分支：其 Demo 交互与浏览器协议有价值，但不直接 cherry-pick 旧代码。
- 保留 A+ 分支：成为新的目标设计与验证基线；**目前不是可上线基线**。
- 在 A+ 分支的下一轮设计迭代中，先做浏览器 Shell、导航和安全状态，不要先扩增装饰效果。
- 生产代码改造另开功能分支；每个 Capability 独立实施、独立 UI 验收和回归，避免大面积重写。
- `master` 保持不动；真实安装版验收后再决策是否合并生产改动。

---

## 12. 追溯文件

- AS-IS 设计：`docs/product/browseros-current-version-20261101/PRODUCT-DESIGN.md`
- 现状设计板：`docs/prototypes/browseros-current-version-20261101/index.html`
- 26 张现状截图：`docs/prototypes/browseros-ui2-aplus-20261101/screenshots/`（**公开仓库有敏感信息风险，暂勿扩散原图**）
- A+ 原型：`docs/prototypes/browseros-ui2-aplus-20261101/index.html`
- 旧 UI2：`origin/design/browseros-ui2-aplus-20261101:docs/prototypes/browseros-ui2/`
- 原始概念比较：`docs/product/BROWSEROS-UI2-APLUS-COMPARISON-20261101.md`（其中此前 9.2 评审分**不再作为验收依据**）
- 当前审核报告：本文

**最终裁定：产品理念方向正确，A+ 当前原型功能设计未达标；先修 P0，完成交互与模块覆盖，再申报 A+ 验收。**
