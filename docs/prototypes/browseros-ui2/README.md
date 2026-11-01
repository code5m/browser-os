# BrowserOS UI 2.0 · Chrome × Apple × AI Native

> Design only · 基线 `master@9607e260f86b15a730219c619ab5a20cc01efe65` · 2026-11-01

## 使用

浏览器直接打开 [index.html](./index.html)，或在仓库目录执行 `python3 -m http.server 8800 --directory docs/prototypes/browseros-ui2` 再打开 `http://127.0.0.1:8800/`。此 HTML + CSS + JS 不需要构建、不联网、不引用 CDN、不依赖真实 Tauri API。数据均为演示，不能用以验证生产执行、模型连接、数据库查询、WebView 性能或真实 Git 修改。

入口：主页、浏览、文件、工具四个一级导航。其他页面在统一页签中打开：Developer Studio、Agent Center、Database Studio、Git Studio、终端、Grid、技能、定时任务、知识中心、知识图谱、脚本库、剪贴板、设置。主题切换、页签关闭、搜索筛选、文件选择、编辑、模拟 Agent 审批、SQL SELECT 模拟查询及 CSV 下载、Git Diff、终端安全演示命令、宫格尺寸切换、任务 Cron 配置、技能步骤、设置、地址建议可以直接点击。Ctrl+L / Ctrl+T / Ctrl+W / Ctrl+Shift+P / Ctrl+1~4 / Studio Ctrl+S 可演示。

**重要：这是体验设计样例，不是 production 工作区代码。** 不把写在文档里的能力冒充已集成。设计评审采用 CODE/UX/GUI 三条独立结论。

## 视觉原则

1. **Chrome 外壳**：42px 页签、46–50px 导航、32–35px 收藏栏、智能地址栏、标签自动收缩和固定窗口动作；延续用户已有使用习惯。
2. **Apple 克制**：语义层级与留白、克制动效、自然圆角、单一主操作色、统一文字间距。不是模仿 macOS 系统 Dock。
3. **专业内容密度**：数据库 / Git / IDE 的编辑区域信息密集，首页和设置更轻。复杂度按工作场景变化，Token 不变化。
4. **AI Native 但可控**：任务状态可见，副作用前审批，错误可恢复。模型输出不是系统动作的事实证明。
5. **一种导航多种资源**：逻辑 Tab Registry 可以统一投影；WebView/PTY/文件模型/数据库连接生命周期仍由原 Capability owner 管理。
6. **无障碍**：明暗主题对比度、焦点可见、语义 label、键盘访问、尊重 reduced-motion；未来生产需要屏幕阅读器与中文输入法实机验收。

## 视觉与组件详细尺寸

| 组件 | 规格 | 状态 | 行为与验收 |
|---|---|---|---|
| WindowFrame | 圆角 12–15px，仅原型 | normal/maximized/minimized | 生产保留原自定义无边框窗口，不改 mousedown 拖拽 |
| UnifiedTabBar | 42px，高亮内沿，宽 92–220px | active/hover/dirty/loading/closed | 同类型与跨类型关闭；普通 Web Tab 不弹确认、不自动 sessionSave；原型关闭不等于资源生命周期实现 |
| Omnibox | 高 35–38px、圆角 11px | idle/focused/suggesting/error | Ctrl+L、上下选建议、Enter、Esc；最多六个，生产 Browser/Grid 必须保留 native menu 不遮盖子 WebView |
| PrimaryNavigation | 四入口，每个 32–36px 触发区 | active/hover/disabled | 首页/浏览/文件/工具唯一主入口，快捷键 Ctrl+1..4 |
| BookmarkBar | 32–35px | visible/hidden | 显隐由用户偏好控制；原生下拉使用 native adapter |
| ContextPanel | 220–264px | expanded/collapsed/loading | 收藏/历史/上下文统一，互斥，不与主要内容竞争 |
| WorkspaceLayout | 左栏 220–260、右栏 280–340 | docked/hidden/resizing | 1024px 优先主内容，720px 隐藏辅助面板；隐藏不销毁 PTY/WebView |
| FileTree / FileList | 行高 34–40px | hover/selected/focused/empty/error | 箭头展开、右键菜单，文件 CRUD 仍归 workspace |
| DocumentManager | 独立稳定 documentId | dirty/saved/conflict/readonly | 模型、撤销历史、光标同步；单文件跨入口不能创建冲突真源 |
| EditorHost | Monaco 宿主；字体 12–13px 行高 19–21px | ready/loading/diff/diagnostic | 保留 Monaco Workers / LSP / SQL parser；原型仅用 textarea 演示，**不是 Monaco 集成** |
| AgentPanel | 320–360px；输入区 76–120px | plan/run/approval/success/fail/cancel | 仅真实 SDK / Agent Runtime 可执行，审批和工具结果必须显式显示 |
| DatabaseStudio | 三栏 + 编辑区 + 表格 | connection/readonly/query/transaction/fail | SQL 列表与 Schema、结果集只演示，真实 SQL 事务仍交给 DB Capability |
| GitStudio | 变更/历史 + 双栏 Diff + Inspector | clean/conflict/preview/confirm | 写入 Preview → Confirm → Execute；原型 Diff 不是仓库实际补丁 |
| TerminalDock | 常规底部 160–240px | attached/detached/recovering | 隐藏视图不销毁 PTY；真实进程不受原型控制 |
| Grid Workspace | 1/2/4 demo，生产保留现有 2/3/4/6/9 格 | creating/ready/suspending/error | Grid 原生桥和 UDS 生命周期不变；此处只有 HTML 矩形占位 |
| Skills | 概览/步骤/检查/权限 | draft/validated/waiting | 逐步定义，权限执行必须用户确认；不虚构后端能力 |
| Tasks | Cron + 类型 + 安全策略 | draft/enabled/running/failed | 表单需约束与校验，真实调度不在演示中启动 |
| Graph | 数据搜索 + 节点 Inspector | empty/data/loading/error | 真实 0 节点时提供带原因的空态；演示节点不表示用户数据 |
| Settings | 左导航+内容面板 | default/saved/error | AI 配置、主题、快捷键、性能、权限分组，不再长卷轴 |
| Status Feedback | 24–28px | ready/error/pending | 成功无 Toast，错误显示在状态栏或独立布局，不能覆盖 WebView |

### 主色与语义

Light：`--surface:#fff`，`--surface2:#f7f9fc`，`--text:#172337`，`--secondary:#64748b`，`--accent:#2563eb`，`--success:#18835f`，`--danger:#c4424a`。Dark 使用语义对应而非反色直接计算。4/8/12/16/24px 间距网格；6–12px 控件圆角；主文字 12–13px；工作区标题 26px。

## 实机截图评审

这次根据用户提供两批 2026-11-01 完整运行截图：主页/快捷方式、技能概览、编码 Agent 空项目/接入项目/问答、Database SQL/查询结果/表格、文件树与右键菜单、Grid 空白、知识图谱 0 节点、Git Log/双栏 Diff、设置页上下两屏、定时任务与 Cron 弹窗、终端、普通网页、地址建议、剪贴板、脚本库。

- 当前优点：Chrome 式外壳已基本成立；Git Diff、Database Inspector、Agent 三栏都具有较深业务功能；文件管理器遵循浏览器页签逻辑。
- 当前 P0 差异：浅色 / 深色模块风格割裂；首页平铺大量同级入口；Agent/Database/Git 每个模块自建控制密度和信息层级；工作流反馈与状态不够统一。
- 截图暴露的问题：Grid 在 19:54 左右截图没有实际内容，只能评审空态，不能断定真实 Grid 不可用；知识图谱 0 节点需要非模糊空态；脚本库和定时任务首次使用状态空旷；设置页很长；数据库工具按钮密集；文件列表行距较大、操作隐藏在右键里。
- 非产品外壳：截图中的 Ubuntu Dock、顶部 GNOME 系统状态栏、第三方网页/BFC 内容库都不归 BrowserOS 组件修改。

## 可靠性和边界

依据 `AGENTS.md`、`PROJECT-RULES.md`、`docs/design/browseros-native-chrome-v3-20261024.md`：**不得**用 CSS fixed/absolute 高 z-index 浮层覆盖 GTK 子 WebView，**不得**修改 WebView.hide/offscreen 方案，**不得**更改 DPI 坐标或 GTK size_allocate；普通 Web Tab 关闭不得确认、不得恢复被否决的 autosave；生产 UI 不能用演示 JS 的 `window.alert`/DOM 模态实现浏览器子视图上的弹窗。本原型仅作为独立网页展示，不能直接覆盖原 Shell。产品实现必须复用 Capability Public API/hostServices，确保每一资源唯一 Owner。 

对照数据请见 [差异矩阵](../../product/BROWSEROS-UI2-SCREENSHOT-GAP-MATRIX-20261101.md) 与 [迁移清单](../../product/BROWSEROS-UI2-MIGRATION-CHECKLIST-20261101.md)。
