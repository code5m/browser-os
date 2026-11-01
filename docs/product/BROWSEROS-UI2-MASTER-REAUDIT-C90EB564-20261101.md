# BrowserOS UI 2.0 · 基于最新 master 的生产代码复审

> 2026-11-01 · 只读审查 + 定向静态检查
>
> **生产基线**：`code5m/browser-os@c90eb564cb69b28da87d07eb95e9e6f78b86a9c6`（GitHub `master`）
>
> **A+ 设计分支**：`browseros-ui2-aplus-20261101`。此文件属于设计分支，不是生产代码。
>
> **历史审查**：[BROWSEROS-UI2-DESIGN-AUDIT-20261101.md](./BROWSEROS-UI2-DESIGN-AUDIT-20261101.md)。
>
> **本次结论**：master 的 Shell / Agent / Grid / Database / Git 的生产交互明显比现有 A+ HTML Mock 完整；**A+ 当前不得直接替换主界面**。保留生产交互契约，融合新版视觉；单独处理公开截图风险。

## 1. 为什么需要重新审核

先核实 GitHub：`c90eb564..master` = `identical`，无新增提交。上一轮基点没有变化，但以前的审核偏重 HTML 和截图，本次补充了 **master 文件级代码审查**，而非从 A+ Mock 推断 BrowserOS 的真实功能。

`master` 的近期两次有效变化：
- `5df888df`：Agent Channel Host、Rig Driver、实验设置的接入。
- `c90eb564`：Full Validation 默认选择快速模式，Rust / 安装版 GUI / 持续资源采样改为手动按需验证；**SKIPPED 不代表 PASS**。

重要区分：
- AS-IS：26 张截图整理的现状设计板，不是可操作的生产版。
- 生产 master：真实 Vue / Rust / Tauri 代码。
- 旧 UI2：较完整的本地交互 Demo，非生产。
- 本次 A+：视觉原型，目前存在关键模拟交互缺陷。

## 2. 直接核查到的生产能力

| 对象 | master 的真实证据 | 对 A+ 设计的强制约束 |
|---|---|---|
| Chrome 外壳 | `src/components/layout/UnifiedTabBar.vue` | 网页与模块页签分开关闭；真正“新建标签”必须调用原有模型而非 `go(home)` |
| 地址栏 / 收藏 | `src/components/layout/ActivityBar.vue` | URL/目录/历史/收藏共存；HTML 浮层只在无原生子 WebView 时使用，浏览/宫格走原生菜单 |
| 主区渲染 | `src/components/layout/MainArea.vue` | BrowserHost 的原生几何/可见性、Terminal resident 不得被视觉重构破坏 |
| Home | `src/capabilities/home/ui/HomeLaunchers.vue` | **18 个当前主工作区入口**。此前报告提到“17 个入口”，需要纠正 |
| Agent | `src/capabilities/agent/ui/CodingWorkbench.vue` | 会话恢复、流式响应、取消、审批、变更应用、脏文件和中断保护均需留存 |
| Agent 定义 | `src/capabilities/agent/ui/AgentManagerPanel.vue` | Coding Workbench + Blueprint 定义仍是两个视图；不能冒称完整通用 Agent 执行底座已与编码 Agent 完全合并 |
| Agent Runtime Lab | `src/components/system/AgentRuntimeExperimentPanel.vue` | Native/Rig/Canary 是默认关闭的实验能力；Benchmark 是离线 Fixture，不是模型质量/GPU/GUI 实测；应急回退标明“仅预留” |
| Grid | `src/capabilities/grid/ui/GridRows.vue` | 2 / 3 / 4 / 6 / 9 格与原生 WebView 生命周期必须保留；A+ 目前没有独立 Grid 视图 |
| Database | `src/capabilities/database/ui/DatabaseWorkspace.vue`、`SqlEditor.vue`、`TableDataEditor.vue` | 事务/SQL/表数据/Inspector 的真实操作与安全边界不能被卡片化 |
| Git | `src/capabilities/git/ui/GitStudio.vue`、`GitWriteConfirmDialog.vue` | Log/Changes/Diff/审查写操作必须保留；不能用静态假 Diff 替换 |
| 定时任务 | `src/capabilities/task/ui/TaskEditDialog.vue` | 仅有脚本/命令片段目标、默认禁用、严格五段 Cron、跳过/补跑、超时/重试；A+ “Agent 任务运行中”属虚构演示 |
| Settings | `src/components/system/SettingsPanel.vue` | AI 模型/快捷键/性能/原生默认浏览器/实验室/工程健康均须可达；新增分组不能丢配置 |

### 首页工作区口径修正（18 个）
浏览、文件、终端、剪贴板、知识库、应用、脚本库、命令库、工具箱、数据库、定时任务、技能、智能体中心、知识图谱、插件、仓库、审计、BFC 内容库。

### Agent 最新功能边界

`CodingWorkbench.vue` 显式拥有：
- `sessionRecoveryRequired` 防止重复执行未知副作用。
- `modelRequestRunning` 和流式片段反馈。
- `resolve_operation`、`claim_operation`、`continueAfterApproval`。
- 保存、应用失败后重新核对会话与磁盘内容，不自动重试有副作用操作。
- `cancel` 请求取消模型后续步骤。

`AgentRuntimeExperimentPanel.vue` 的设计原则是实验室与默认 Coding Agent 分离：实验关闭时使用 Native，Canary 仅作用于新任务；离线 Benchmark 不评估真实模型/工具/GUI 性能，且真实只读工具需要显式人工确认。因此视觉上应该作为 **Developer Settings → Agent Runtime Lab**，不能作为用户聊天页的“默认已启用”状态。

## 3. 已执行的 master 定向检查（真实 PASS）

**测试方法**：通过 `git archive origin/master` 将生产基线导出到授权机器的 `/tmp/browseros-master-ui-reaudit-c90eb564`，复用只读 Node 依赖，不执行 `git worktree`、不切换主仓分支、不开启 Full Validation。

| 命令 | 本轮结果 | 证明范围 |
|---|---|---|
| `node scripts/check-ui-product-quality.mjs` | PASS | 首页/导航、Omnibox、Grid/文件基础设计静态合同 |
| `node scripts/check-adaptive-shell.mjs` | PASS | Shell 代码边界、原生适配、导航策略静态合同 |
| `node scripts/check-browser-grid-navigation-regression.mjs` | PASS | 浏览器与 Grid 导航回归静态检查 |
| `node tests/check-real-ui-regressions.mjs` | **PASS，39 项** | 已编写的 Real UI 结构性/回归断言 |

> 上述是目标源代码的定向静态/契约检查，**并非真实安装版 GUI、Tauri Rust 编译、端到端 WebView、真实 SQL/Git/Agent 执行验证**。该轮没有运行完整 Rust CI、打包或资源基准。默认 `native_mode=off` 的 CI PASS 也不能代替这些结果。

## 4. 审核发现：哪些旧批评应调整

1. **“浏览器交互不完整”** 应明确指向新 A+ HTML Mock，而不是 master：master 已有真实标签关闭、地址建议、浏览/目录切换等路径。
2. **“Agent 只有简单聊天界面”** 已不成立：master 编码工作区具备审批与中断恢复。但这不意味着通用 Agent 与 Coding Agent 已完全统一。
3. **“Grid 只有 2/4 格”** 是截图快捷选项的表象；master 的高级选项是 2/3/4/6/9。
4. **“定时任务支持 Agent 类型并已运行”** 不能从现有生产表单得到支持；实际创建表单仅脚本和命令片段。
5. **“A+ 9.2/10”** 仍然不能作为验收结果：Chrome 实测显示 A+ 关闭标签、Ctrl+L、Agent 路由、Task 初次交互、小窗口排版均失败。
6. **“Settings 没有 Agent Runtime”** 对最新 master 不成立；已经通过异步组件挂载实验室，但未进行安装版视觉验收。
7. **“全局状态伪造”** 是 A+ Mock 的问题；master 的真实状态 UI 与原型数字不能混为一谈。

## 5. master 仍存在的设计与工程风险

| 优先级 | 问题 | 具体证据 | 建议 |
|---|---|---|---|
| P0 | 公开仓库截图暴露风险 | `gh api repos/code5m/browser-os` 返回 `visibility:public`；设计分支曾提交含内部连接信息及用户字段的实机截图 | 尽快处理访问控制/脱敏/历史清理审批。不要仅删除新分支文件就声称公开历史清除 |
| P0 | A+ 假动作破坏浏览器契约 | `docs/prototypes/browseros-ui2-aplus-20261101/index.html`：tab close 实际导航、Ctrl+L 未绑定、Browser/Grid 缺独立页面 | 不允许直接移植原型 DOM 到生产 |
| P0 | A+ 定时任务开关默认 ON | A+ Mock 的任务 Drawer 初始可见且 `.switch.on`，与 master 的 TaskEditDialog 默认禁用契约冲突 | 恢复默认 OFF，先配置校验再允许用户显式启用 |
| P1 | 真实 UI 视觉层不统一 | 实机截图与 `UI-TOKEN-AUDIT.md` 记录的重复 Token、面板和弹窗差异 | 从现有组件/Token 迁移，不重做 Capability |
| P1 | Shell 和跨域协调仍有耦合 | `App.vue`、`ActivityBar.vue`、`UnifiedTabBar.vue`、`MainArea.vue` 仍承担大量跨 Store/UI 逻辑 | 严守 Public API 与 Owner 边界；不要把新 A+ 大侧栏变成另一个总控 |
| P1 | Agent 双引擎处于实验阶段 | Runtime Lab 明示“默认关闭”“只影响新任务”“应急回退仅预留”，Benchmark 是 Fixture | 不对终端用户宣称统一生产级 Agent Runtime 完成 |
| P1 | 文档与代码存在漂移风险 | `UI-OWNERSHIP-GRAPH.md` 仍有 MainArea 直接渲染多个能力的旧链路描述，而 master MainArea 已大量切换 Contribution | 对照最新源码更新架构图，复验产物/Owner 结构 |
| P1 | 窄窗口设计缺失 | 新 A+ 在 760px 渲染首页 Hero 宽仅 117px、标题宽 59px | 断点重排，不能用 overflow 掩盖主操作被挤出 |
| P2 | 复杂模块控制区拥挤 | 26 张截图中的 Database、Agent、Git、Skills、Settings | 优化视觉分组与操作优先级，不降低功能数量 |

## 6. 针对 master 的正确设计方案

### 不该重建的基础
- `UnifiedTabBar` + Browser Store + Module Tabs 的语义。
- `ActivityBar` + Native Menu + `ResizeObserver` 处理地址栏行为。
- `MainArea` 现有 BrowserHost / Resident Terminal 的 Native 资源边界。
- Coding Agent 的项目树、Monaco、LSP、会话、审批与流式执行。
- DB Session / Git Diff / PTY / Grid WebView 的 Owner。

### 应当直接改进的视觉
- Home：把 18 个入口做为易发现的全量能力地图，但高频入口优先。
- Settings：左侧分组，右侧详情；Agent Runtime Lab 放在高级/实验区。
- Agent：统一项目上下文、任务状态、Plan、工具审批、文件变更，在视觉上明确边界。
- DB/Git：保留专业深色高密度和操作区，统一按钮层级、状态与 Diff 语言。
- Grid：不出现“空白即故障”的含糊状态；区分未启动/正在创建/空内容/加载失败/重试中。
- 浏览器/网页：不在所有普通网页前强制渲染一条永久产品侧边栏。
- 导航：稳定 Shell + 可收起 Workspace 导航，不建立与 `useLayoutStore` 冲突的第二套路由真源。
- Token：面板间距、字体、圆角、语义状态与焦点一致，专业区域可保留不同信息密度。

### 设计验收硬要求（逐项可测）

- [ ] 18 个现有入口 100% 可达（直接或通过明确工具聚合），包含独立 Browser、Grid、Terminal、Agent。
- [ ] Tab 新建/关闭/切换、前进后退/刷新、Ctrl+L/T/W、Omnibox URL/目录/历史和收藏完整。
- [ ] Grid 2/3/4/6/9 原生布局、大小改变、隐藏/恢复、连接失败和资源占用提示。
- [ ] Coding Agent 流式输出、取消、审批、变更预览、未知结果恢复与脏文件保护可演示。
- [ ] DB SQL、表数据、事务、Inspector 和 Git Log/Changes/Diff/写操作审核不退化。
- [ ] Task 仅展示生产支持的两类目标；严格 5 段 Cron、默认禁用、失败与重试状态完整。
- [ ] Runtime Lab 清楚标识实验、Fixture、默认关闭、API 只读和回退未自动实现。
- [ ] 所有 Mock 指标显示 DEMO，不宣称生产服务运行或用户数据真实同步。
- [ ] 1920 / 1440 / 1024 / 760px 窗口操作可达，键盘/屏幕阅读器/中文输入法可用。
- [ ] 所有 GUI/性能结论有来自同一 master SHA 的安装版真实证据。

## 7. 最终裁定

**master：核心行为与架构基础可以保留，主要问题是视觉一致性、信息层级和部分模块体验。**

**旧 UI2：更接近交互完整的设计实验，但不是生产替代品。**

**现有 A+：视觉有提升，但路由/导航/安全状态/窄窗口功能退化，仍然 NOT READY。**

**最合适的 UI 2.0：基于 master 的真实 Capability 和浏览器壳修视觉、做交互优化；绝不把视觉 Mock 当作业务替代实现。**

本文件是设计复审，不涉及代码变更或 master 合并。
